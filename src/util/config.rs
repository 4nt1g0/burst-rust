use config::{Config, File};

use util::deserialization::from_str;

lazy_static! {
    pub static ref CONFIG: Config = {
        let mut config = Config::default();
        config.merge(File::with_name("Settings-default")).expect("aa");
        config.merge(File::with_name("Settings")).ok();
        config
    };
}

#[derive(Debug, Deserialize, Clone)]
pub struct DeviceConfig {
    platform_id: u32,
    device_id: u32,
    global_work_size: u32,
    local_work_size: u32,
    hashes_number: u32,
}

impl DeviceConfig {
    pub fn platform_id(&self) -> u32 {
        self.platform_id
    }
    pub fn device_id(&self) -> u32 {
        self.device_id
    }
    pub fn global_work_size(&self) -> u32 {
        self.global_work_size
    }
    pub fn local_work_size(&self) -> u32 {
        self.local_work_size
    }
    pub fn hashes_number(&self) -> u32 {
        self.hashes_number
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct WorkConfig {
    #[serde(deserialize_with = "from_str")]
    address: u64,
    passphrase: String,
    wallet_url: String,
    mining_info_interval_seconds: u64,
    #[serde(deserialize_with = "from_str")]
    target_deadline: u64,
    submission_retry_number: u64,
    submission_retry_interval_seconds: u64,
}

impl WorkConfig {
    pub fn address(&self) -> u64 {
        self.address
    }
    pub fn passphrase(&self) -> &str {
        &self.passphrase
    }
    pub fn wallet_url(&self) -> &str {
        &self.wallet_url
    }
    pub fn mining_info_interval_seconds(&self) -> u64 {
        self.mining_info_interval_seconds
    }
    pub fn target_deadline(&self) -> u64 {
        self.target_deadline
    }
    pub fn submission_retry_number(&self) -> u64 {
        self.submission_retry_number
    }
    pub fn submission_retry_interval_seconds(&self) -> u64 {
        self.submission_retry_interval_seconds
    }
}
