use std::fs::File;
use crate::structs::{Result, VfsError};

pub const MAGIC: &[u8; 4] = b"RVFS";
pub const VERSION: u32 = 1;

// Două superblock-uri fixe. Le rezervăm 64 bytes fiecare.
pub const SB_SIZE: u64 = 64;
pub const SB_A_OFF: u64 = 0;
pub const SB_B_OFF: u64 = SB_SIZE;

/// Superblock: spune unde e snapshot-ul curent.
#[derive(Debug, Clone)]
pub struct Superblock {
    pub version: u32,
    pub generation: u64,
    pub snapshot_offset: u64,
    pub snapshot_len: u64,
    pub snapshot_crc32: u32,
}

impl Superblock {
    pub fn invalid() -> Self {
        Self {
            version: VERSION,
            generation: 0,       
            snapshot_offset: 0,
            snapshot_len: 0,
            snapshot_crc32: 0,
        }
    }

     pub fn encode_fixed(&self) -> [u8; SB_SIZE as usize] {
        let mut out = [0u8; SB_SIZE as usize];

        out[0..4].copy_from_slice(MAGIC);                                 //pretty damn self explanatory
        out[4..8].copy_from_slice(&self.version.to_le_bytes());           //version (u32)
        out[8..16].copy_from_slice(&self.generation.to_le_bytes());       //generation (u64)
        out[16..24].copy_from_slice(&self.snapshot_offset.to_le_bytes()); //snapshot_offset (u64)
        out[24..32].copy_from_slice(&self.snapshot_len.to_le_bytes());    //snapshot_len (u64)
        out[32..36].copy_from_slice(&self.snapshot_crc32.to_le_bytes());  //snapshot_crc32 (u32)
        out //restul e padding
    }
}
