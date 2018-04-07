use byteorder::{ByteOrder, BigEndian};
use util::sph_shabal;
use reqwest;
use serde_json;
use util::deserialization::{from_str, bytes_from_hex_string};
use failure::Error;
use std::io::{Read, Write};
use util::constants::SCOOPS_PER_PLOT;
use std::sync::mpsc::Sender;
use util::config::WorkConfig;
use std::thread;
use std;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::channel;

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MiningInfo {
    #[serde(deserialize_with = "bytes_from_hex_string")]
    generation_signature: Vec<u8>,
    #[serde(deserialize_with = "from_str")]
    height: u64,
    #[serde(deserialize_with = "from_str")]
    base_target: u64,
    scoop_number: Option<u16>,
}

impl MiningInfo {
    pub fn generation_signature(&self) -> &[u8] {
        &self.generation_signature[..]
    }
    pub fn height(&self) -> u64 {
        self.height
    }
    pub fn base_target(&self) -> u64 {
        self.base_target
    }
    fn calculate_scoop_number(&mut self) -> Result<u16, Error> {
        let mut height = [0u8; 8];
        BigEndian::write_u64(&mut height, self.height);

        let mut scoop_prefix: [u8; 40] = [0; 40];

        (&mut scoop_prefix[0..32]).write(&self.generation_signature[..])?;
        (&mut scoop_prefix[32..40]).write(&height)?;

        let scoop_prefix_shabal = sph_shabal::shabal256(&scoop_prefix);

        Ok(BigEndian::read_u16(&scoop_prefix_shabal[30..]) % SCOOPS_PER_PLOT as u16)
    }
    pub fn scoop_number(&mut self) -> u16 {
        if let Some(scoop_number) = self.scoop_number {
            scoop_number
        } else {
            let scoop_number = self.calculate_scoop_number().expect("could not calculate scoop number");
            self.scoop_number = Some(scoop_number);
            scoop_number
        }
    }

}

pub fn load_mining_info(wallet_url: &str) -> Result<MiningInfo, Error> {
    let result = reqwest::get(&format!("{}/burst?requestType=getMiningInfo", wallet_url))?
        .text()?;

    serde_json::from_str::<MiningInfo>(&result).map_err(|e| e.into())
}

pub fn submit_deadline(wallet_url: &str, address: u64, passphrase: &str, nonce: u64) -> Result<String, Error> {
    // TODO: reuse client
    let client = reqwest::Client::new();

    let mut url = reqwest::Url::parse(&format!("{}/burst?requestType=submitNonce", wallet_url))?;
    url.query_pairs_mut()
        .append_pair("secretPhrase", passphrase)
        .append_pair("nonce", &nonce.to_string())
        .append_pair("address", &address.to_string());

    let mut res = client
        .post(url)
        .send()?;

    let mut out = String::new();
    res.read_to_string(&mut out)?;

    res.error_for_status()?;
    // TODO parse json
    Ok(out)
}

pub fn format_duration_from_seconds(seconds: u64) -> String {
    let mut seconds = seconds;
    let years = seconds / (60*60*24*30*12);
    seconds -= years * (60*60*24*30*12);
    let months = seconds / (60*60*24*30);
    seconds -= months * (60*60*24*30);
    let days = seconds / (60*60*24);
    seconds -= days * (60*60*24);
    let hours = seconds / (60*60);
    seconds -= hours * (60*60);
    let minutes = seconds / 60;
    seconds -= minutes * 60;

    String::from(format!("{}y {}m {}d {}h {}m {}s", years, months, days, hours, minutes, seconds))
}

pub struct MiningInfoListener {
    current_height: u64,
    work_config: WorkConfig,
    send_channel: Sender<MiningInfo>,
}

impl MiningInfoListener {
    pub fn start(work_config: WorkConfig, send_channel: Sender<MiningInfo>) {
        let mut listener = Self {current_height: 0, work_config, send_channel };
        thread::spawn(move || listener.listen());
    }

    pub fn listen(&mut self) {
        loop {
            match load_mining_info(&self.work_config.wallet_url()) {
                Ok(mining_info) => {
                    if mining_info.height() > self.current_height {
                        self.current_height = mining_info.height();
                        self.send_channel.send(mining_info).ok();
                    }
                },
                Err(_) => eprintln!("Getting Mining info failed"),
            }
            thread::sleep(std::time::Duration::from_secs(self.work_config.mining_info_interval_seconds()));
        }
    }
}


pub struct NonceSubmitter {
    _work_config: WorkConfig,
    sender: Sender<Option<u64>>,
}

impl NonceSubmitter {
    pub fn new(work_config: WorkConfig) -> Self {
        let (tx, rx) = channel();
        let config = work_config.clone();
        thread::spawn(move || Self::submission_loop(rx, config));
        Self {_work_config: work_config, sender: tx}
    }

    fn submission_loop(nonce_receiver: Receiver<Option<u64>>, work_config: WorkConfig) {
        loop {
            let current_nonce = nonce_receiver.recv().unwrap_or(None);
            let mut retries = 0;

            if let Some(mut nonce) = current_nonce {
                while retries < work_config.submission_retry_number() {
                    if let Ok(new_nonce) = nonce_receiver.try_recv() {
                        match new_nonce {
                            Some(new_nonce) => {
                                retries = 0;
                                nonce = new_nonce;
                            },
                            None => break
                        }
                    }

                    match submit_deadline(&work_config.wallet_url(), work_config.address(), &work_config.passphrase(), nonce) {
                        Ok(result) => {
                            eprintln!("Submitted! {}", result);
                            break;
                        },// TODO: check result success
                        Err(_) => {
                            retries += 1;
                            eprintln!("Submitting deadline failed {}/{}", retries, work_config.submission_retry_number());
                        }
                    }
                    thread::sleep(std::time::Duration::from_secs(work_config.submission_retry_interval_seconds()));
                }
            }
        }

    }

    pub fn submit(&self, nonce: u64) {
        // TODO: check for target deadline and better earlier submission
        // TODO: get notified of new mining info and cancel/prevent submission
        self.sender.send(Some(nonce)).ok();
    }

    pub fn cancel_submission(&self) {
        self.sender.send(None).ok();
    }
}
