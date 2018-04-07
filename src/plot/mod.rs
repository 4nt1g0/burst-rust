use byteorder::{ByteOrder, LittleEndian};
use failure::Error;
use std::io::Write;

use mine::MiningInfo;
use util::sph_shabal;
use util::constants::{SCOOPS_PER_PLOT, SCOOP_SIZE, PLOT_SIZE, GEN_SIZE};

pub mod ocl_nonce_computer;

pub struct ScoopData<'a> {
    data: &'a [u8; SCOOP_SIZE],
}

impl<'a> ScoopData<'a> {
    pub fn from_bytes(data: &'a [u8; SCOOP_SIZE]) -> Self {
        Self { data }
    }
    pub fn from_slice(data: &'a [u8]) -> Self {
        Self { data: array_ref!(data, 0, SCOOP_SIZE) }
    }
    pub fn calculate_deadline(&self, mining_info: &MiningInfo) -> Result<u64, Error> {
        let mut input = [0u8; 32 + 32 + 32]; // gensig + scoop data

        (&mut input[0..32]).write(&mining_info.generation_signature()[..])?;
        (&mut input[32..96]).write(self.data)?;

        let shabal = sph_shabal::shabal256(&input);

        let target = LittleEndian::read_u64(&shabal[0..8]);

        Ok(target / mining_info.base_target())
    }
    pub fn bytes(&self) -> &[u8; SCOOP_SIZE] {
        self.data
    }
}

pub struct Nonce<'a> {
    data: &'a [u8; PLOT_SIZE as usize]
}

impl<'a> Nonce<'a> {
    pub fn from_bytes(data: &'a [u8; PLOT_SIZE as usize]) -> Self {
        Self { data }
    }
    pub fn from_slice(data: &'a [u8]) -> Self {
        Self { data: array_ref!(data, 0, PLOT_SIZE) }
    }
    pub fn scoop_data(&self, scoop_number: u16) -> ScoopData {
        assert!(scoop_number < SCOOPS_PER_PLOT);
        let offset = scoop_number as usize * SCOOP_SIZE;
        ScoopData::from_slice(&self.data[offset..offset + SCOOP_SIZE])
    }
    pub fn calculate_deadline(&self, mining_info: &MiningInfo) -> Result<u64, Error> {
        // TODO: move to mining package?
        let scoop_data = self.scoop_data(1);
        let mut input = [0u8; 32 + 32 + 32]; // gensig + scoop data

        (&mut input[0..32]).write(&mining_info.generation_signature()[..])?;
        (&mut input[32..96]).write(scoop_data.bytes())?;

        let shabal = sph_shabal::shabal256(&input);

        let target = LittleEndian::read_u64(&shabal[0..8]);

        Ok(target / mining_info.base_target())
    }
}

/// A (unoptimized) plot consisting of nonces with scoop data
pub struct PlotResult {
    start_nonce: u64,
    pub data: Vec<u8>, // FIXME
}

impl PlotResult {
    pub fn from_bytes(start_nonce: u64, data: Vec<u8>) -> Self {
        assert_eq!(data.len() % GEN_SIZE, 0, "Plot size {} not a multiple of {}", data.len(), GEN_SIZE);
        Self { start_nonce, data }
    }
    pub fn start_nonce(&self) -> u64 {
        self.start_nonce
    }
    pub fn num_nonces(&self) -> u64 {
        (self.data.len() / PLOT_SIZE) as u64
    }
    pub fn nonce_by_number(&self, nonce_number: u64) -> Option<Nonce> {
        if nonce_number < self.start_nonce {
            return None;
        }

        self.nonce_by_index(nonce_number - self.start_nonce)
    }
    pub fn nonce_by_index(&self, nonce_index: u64) -> Option<Nonce> {
        if nonce_index > self.num_nonces() {
            return None;
        }
        let offset = nonce_index as usize * GEN_SIZE as usize;
        Some(Nonce::from_slice(&self.data[offset..offset + PLOT_SIZE]))
    }
    pub fn nonces(&self) -> impl Iterator<Item=Nonce> {
        self.data.chunks(GEN_SIZE).map(|slice| Nonce::from_slice(slice))
    }
}
