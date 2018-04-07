pub const HASH_SIZE: u32 = 32;
pub const HASHES_PER_SCOOP: usize = 2;
pub const SCOOP_SIZE: usize = HASHES_PER_SCOOP * HASH_SIZE as usize;
pub const SCOOPS_PER_PLOT: u16 = 4096;
pub const PLOT_SIZE: usize = SCOOPS_PER_PLOT as usize * SCOOP_SIZE;
pub const GEN_SIZE: usize = PLOT_SIZE + 16;
