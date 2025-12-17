use crate::structs::{Result, VfsError};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};

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

        out[0..4].copy_from_slice(MAGIC); //pretty damn self explanatory
        out[4..8].copy_from_slice(&self.version.to_le_bytes()); //version (u32)
        out[8..16].copy_from_slice(&self.generation.to_le_bytes()); //generation (u64)
        out[16..24].copy_from_slice(&self.snapshot_offset.to_le_bytes()); //snapshot_offset (u64)
        out[24..32].copy_from_slice(&self.snapshot_len.to_le_bytes()); //snapshot_len (u64)
        out[32..36].copy_from_slice(&self.snapshot_crc32.to_le_bytes()); //snapshot_crc32 (u32)
        out //restul e padding
    }

    pub fn decode_fixed(data: &[u8; SB_SIZE as usize]) -> Result<Self> {
        if &data[0..4] != MAGIC {
            return Err(VfsError::Corrupt("invalid superblock magic".to_string()));
        }

        let version = match data[4..8].try_into() {
            Ok(b) => u32::from_le_bytes(b),
            Err(_) => return Err(VfsError::Corrupt("superblock truncated (version)".into())),
        };
        let generation = match data[8..16].try_into() {
            Ok(b) => u64::from_le_bytes(b),
            Err(_) => {
                return Err(VfsError::Corrupt(
                    "superblock truncated (generation)".into(),
                ));
            }
        };
        let snapshot_offset = match data[16..24].try_into() {
            Ok(b) => u64::from_le_bytes(b),
            Err(_) => {
                return Err(VfsError::Corrupt(
                    "superblock truncated (snapshot_offset)".into(),
                ));
            }
        };
        let snapshot_len = match data[24..32].try_into() {
            Ok(b) => u64::from_le_bytes(b),
            Err(_) => {
                return Err(VfsError::Corrupt(
                    "superblock truncated (snapshot_len)".into(),
                ));
            }
        };
        let snapshot_crc32 = match data[32..36].try_into() {
            Ok(b) => u32::from_le_bytes(b),
            Err(_) => {
                return Err(VfsError::Corrupt(
                    "superblock truncated (snapshot_crc32)".into(),
                ));
            }
        };
        Ok(Self {
            version,
            generation,
            snapshot_offset,
            snapshot_len,
            snapshot_crc32,
        })
    }
    
    /// Citeste superblock de la offset (0 sau 64)
    pub fn read_superblock(file: &mut File, off: u64) -> Result<Superblock> {
        let mut buf = [0u8; SB_SIZE as usize];
        file.seek(SeekFrom::Start(off))?;
        file.read_exact(&mut buf)?;
        Superblock::decode_fixed(&buf)
    }

    /// Scrie superblock la offset si da flush
    pub fn write_superblock(file: &mut File, off: u64, sb: &Superblock) -> Result<()> {
        let buf = sb.encode_fixed();
        file.seek(SeekFrom::Start(off))?;
        file.write_all(&buf)?;
        file.flush()?;
        Ok(())
    }
}
