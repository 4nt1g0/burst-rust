#![feature(conservative_impl_trait)]
#![feature(try_from)]

extern crate burst_rust;

use std::sync::mpsc::channel;
use std::time::Instant;

use burst_rust::util::config::CONFIG;
use burst_rust::util::config::{DeviceConfig, WorkConfig};
use burst_rust::plot::ocl_nonce_computer::OclNonceComputer;
use burst_rust::mine::{MiningInfo, format_duration_from_seconds};
use burst_rust::util::constants::PLOT_SIZE;
use burst_rust::mine::MiningInfoListener;
use burst_rust::mine::NonceSubmitter;
use burst_rust::plot::ocl_nonce_computer::continuous_nonce_computer;
use std::sync::mpsc::sync_channel;
use burst_rust::plot::PlotResult;

fn main() {
    let device_config: DeviceConfig = CONFIG.get("device").expect("Missing device config");
    let work_config: WorkConfig = CONFIG.get("work").expect("Missing work config");

    eprintln!("Target Deadline: {}", format_duration_from_seconds(work_config.target_deadline()));

    let (mining_info_tx, mining_info_rx) = channel();

    MiningInfoListener::start(work_config.clone(), mining_info_tx);

    let nonce_submitter = NonceSubmitter::new(work_config.clone());

    let nonce_computer = OclNonceComputer::new(device_config.clone(), work_config.address()).expect("Invalid config");

    let (nonces_tx, nonces_rx) = sync_channel(2);
    let (nonces_idx_tx, nonces_idx_rx) = channel();
    continuous_nonce_computer(nonce_computer, nonces_idx_rx, nonces_tx);

    let mut mining_info = mining_info_rx.recv().expect("Could not get mining info");
    let mut scoop_number = mining_info.scoop_number();
    print_mining_info(&mining_info, scoop_number);

    let mut start = Instant::now();
    let mut best_deadline = <u64>::max_value();
    loop {
        if let Ok(new_mining_info) = mining_info_rx.try_recv() {
            mining_info = new_mining_info;
            scoop_number = mining_info.scoop_number();
            best_deadline = <u64>::max_value();
            nonces_idx_tx.send(0).expect("could not reset nonce");
            nonce_submitter.cancel_submission();
            start = Instant::now();
            print_mining_info(&mining_info, scoop_number);
        }

        let plot = nonces_rx.recv().expect("could not get next nonces");

        let deadlines = compute_deadlines(&mining_info, &plot, scoop_number);

        let &(nonce, new_best_deadline) = deadlines.iter().min_by_key(|a| a.1).expect("No best deadline found");

        if new_best_deadline < best_deadline {
            best_deadline = new_best_deadline;
            eprintln!("Nonce: {}: DL {:?} = {} ", nonce, best_deadline, format_duration_from_seconds(best_deadline));

            if best_deadline <= work_config.target_deadline() {
                nonce_submitter.submit(nonce);
            }
        }

        let current_nonce = plot.start_nonce() + plot.num_nonces();
        eprintln!("Current Block Nonce: {} => Pseudo Plot Size: {}GB. Speed: {:.0} Nonces/min",
                  current_nonce,
                  current_nonce * PLOT_SIZE as u64 / (1024 * 1024 * 1024),
                  current_nonce as f64 / (start.elapsed().as_secs() as f64 / 60.0));
    }
}

fn compute_deadlines(mining_info: &MiningInfo, plot: &PlotResult, scoop_number: u16) -> Vec<(u64, u64)> {
    (plot.start_nonce()..plot.start_nonce() + plot.num_nonces())
        .zip(plot.nonces()
            .map(|nonce|
                nonce.scoop_data(scoop_number).calculate_deadline(&mining_info)
                    .expect("Deadline could not be calculated"))).collect()
}

fn print_mining_info(mining_info: &MiningInfo, scoop_number: u16) {
    eprintln!("================\nNew Block\nHeight: {:}\nScoop: {:}\n================",
              mining_info.height(), scoop_number);
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_works() {
        let device_config = DeviceConfig {
            platform_id: 1,
            device_id: 0,
            global_work_size: 8192,
            local_work_size: 128,
            hashes_number: 8192,
        };


        let mining_info = MiningInfo {
            generation_signature: vec![233, 36, 246, 242, 87, 223, 13, 96, 189, 243, 238, 93, 70, 224, 34, 49, 217, 12, 178, 207, 182, 244, 24, 126, 226, 177, 148, 68, 138, 37, 253, 176],
            height: 465699,
            base_target: 43899,
            scoop_number: None,
        };

        let work_config = WorkConfig {
            address: 11433454602339013530,
            passphrase: "",
            wallet_url: String::from(""),
            mining_info_interval_seconds: 0,
            target_deadline: 0,
            submission_retry_number: 0,
            submission_retry_interval_seconds: 0,
        };

        let mut nonce_computer = OclNonceComputer::new(device_config, work_config.address()).expect("Invalid config");

        let nonces = nonce_computer.compute_next_nonces().unwrap();
        println!("nonces {:x}", &nonces[64..128].as_hex());


        let scoop_data = extract_scoop_data(&nonces, 0);

        println!("Scoop data[1] = {:x}", scoop_data[1].as_hex());
        assert_eq!(calculate_deadline(scoop_data[0], &mining_info).unwrap(), 304653882166113, "Scoop 0 Nonce 0");
        assert_eq!(calculate_deadline(scoop_data[42], &mining_info).unwrap(), 142426830646534, "Scoop 0 Nonce 42");


        let scoop_data = extract_scoop_data(&nonces, 1337);
        assert_eq!(calculate_deadline(scoop_data[0], &mining_info).unwrap(), 282452543406894, "Scoop 1337 Nonce 0");
        assert_eq!(calculate_deadline(scoop_data[42], &mining_info).unwrap(), 146916916496699, "Scoop 1337 Nonce 42");
    }
}