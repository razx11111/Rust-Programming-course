use crate::structs::{Result, VfsError, Header, InodeId, Extent, ExtentList};
use crc32fast::Hasher;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use crate::structs::{InodeSnapshot, Metadata, NodeKind, Timestamp, DirEntry};

const RECORD_MAGIC: &[u8; 4] = b"VFSR";
const HEADER_MAGIC: &[u8; 8] = b"RVFS0001";
const VERSION: u32 = 1;
const HEADER_LEN: u64 = 8 + 4 + 4 + 8; //aproape cum aveam pt superblock

pub struct Encoder {
    buf: Vec<u8>,
}

impl Encoder {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.buf
    }

    pub fn put_u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    pub fn put_u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn put_u64(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn put_i128(&mut self, v: i128) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn put_bytes(&mut self, bytes: &[u8]) {
        self.put_u64(bytes.len() as u64);
        self.buf.extend_from_slice(bytes);
    }

    pub fn put_string(&mut self, s: &str) {
        self.put_bytes(s.as_bytes());
    }
}

/// Decoder little endian. Asiguram si sa nu depasim lungimea bufferului
pub struct Decoder<'a> {
    input: &'a [u8],
    poz: usize,
}

impl<'a> Decoder<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self { input, poz: 0 }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .poz
            .checked_add(n)
            .ok_or_else(|| VfsError::CorruptLog("decoder position overflow".to_string()))?;
        if end > self.input.len() {
            return Err(VfsError::CorruptLog(
                "unexpected EOF while decoding".to_string(),
            ));
        }
        let out = &self.input[self.poz..end];
        self.poz = end;
        Ok(out)
    }

    pub fn get_u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    pub fn get_u32(&mut self) -> Result<u32> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn get_u64(&mut self) -> Result<u64> {
        let b = self.take(8)?;
        Ok(u64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    pub fn get_i128(&mut self) -> Result<i128> {
        let b = self.take(16)?;
        Ok(i128::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13],
            b[14], b[15],
        ]))
    }

    pub fn get_bytes(&mut self) -> Result<&'a [u8]> {
        let len = self.get_u64()? as usize;
        self.take(len)
    }

    pub fn get_string(&mut self) -> Result<String> {
        let b = self.get_bytes()?;
        std::str::from_utf8(b)
            .map(|s| s.to_string())
            .map_err(|_| VfsError::CorruptLog("invalid utf-8 string in snapshot".to_string()))
    }

    pub fn is_eof(&self) -> bool {
        self.poz == self.input.len()
    }
}

pub fn crc32(data: &[u8]) -> u32 {
    let mut hasher = Hasher::new();
    hasher.update(data);
    hasher.finalize()
}

pub fn write_header(file: &mut File, block_size: u32, root: InodeId) -> Result<()> {
    let mut e = Encoder::new();
    e.buf.extend_from_slice(HEADER_MAGIC);
    e.put_u32(VERSION);
    e.put_u32(block_size);
    e.put_u64(root.0);

    let bytes = e.into_inner();
    file.seek(SeekFrom::Start(0))?;
    file.write_all(&bytes)?;
    file.flush()?;
    Ok(())
}

pub fn read_header(file: &mut File) -> Result<Header> {
    file.seek(SeekFrom::Start(0))?;
    let mut buf = vec![0u8; HEADER_LEN as usize];
    file.read_exact(&mut buf)?;

    if &buf[0..8] != HEADER_MAGIC {
        return Err(VfsError::CorruptLog("invalid header magic".into()));
    }
    let mut d = Decoder::new(&buf[8..]);
    let version = d.get_u32()?;
    if version != VERSION {
        return Err(VfsError::UnsupportedVersion(version));
    }
    let block_size = d.get_u32()?;
    let root = InodeId(d.get_u64()?);

    Ok(Header {
        magic: *HEADER_MAGIC,
        version,
        block_size,
        root,
    })
}

fn encode_extent(e: &mut Encoder, ex: &Extent) {
    e.put_u64(ex.logical_offset);
    e.put_u64(ex.file_offset);
    e.put_u64(ex.len);
}

fn decode_extent(d: &mut Decoder<'_>) -> Result<Extent> {
    Ok(Extent {
        logical_offset: d.get_u64()?,
        file_offset: d.get_u64()?,
        len: d.get_u64()?,
    })
}

fn encode_inode_snapshot(e: &mut Encoder, snap: &InodeSnapshot) {
    e.put_u64(snap.id.0);

    // parent: Option<InodeId>
    match snap.parent {
        Some(p) => { e.put_u8(1); e.put_u64(p.0); }
        None => e.put_u8(0),
    }

    e.put_string(&snap.name);

    // kind
    e.put_u8(match snap.kind {
        NodeKind::File => 1,
        NodeKind::Dir => 2,
    });

    // metadata
    e.put_u64(snap.metadata.size);
    e.put_i128(snap.metadata.created_at.0);
    e.put_i128(snap.metadata.modified_at.0);

    // extents
    e.put_u64(snap.extents.len() as u64);
    for ex in &snap.extents {
        encode_extent(e, ex);
    }
}

fn decode_inode_snapshot(d: &mut Decoder<'_>) -> Result<InodeSnapshot> {
    let id = InodeId(d.get_u64()?);

    let parent = match d.get_u8()? {
        0 => None,
        1 => Some(InodeId(d.get_u64()?)),
        _ => return Err(VfsError::CorruptLog("invalid parent tag".into())),
    };

    let name = d.get_string()?;

    let kind = match d.get_u8()? {
        1 => NodeKind::File,
        2 => NodeKind::Dir,
        _ => return Err(VfsError::CorruptLog("invalid inode kind".into())),
    };

    let size = d.get_u64()?;
    let created_at = Timestamp(d.get_i128()?);
    let modified_at = Timestamp(d.get_i128()?);

    let extent_count = d.get_u64()? as usize;
    let mut extents: ExtentList = Vec::with_capacity(extent_count);
    for _ in 0..extent_count {
        extents.push(decode_extent(d)?);
    }

    Ok(InodeSnapshot {
        id,
        parent,
        name,
        kind,
        metadata: Metadata { size, created_at, modified_at },
        extents,
    })
}

fn encode_dir_entry(e: &mut Encoder, de: &DirEntry) {
    e.put_u64(de.parent.0);
    e.put_u64(de.inode.0);
    e.put_string(&de.name);
    e.put_u8(match de.kind {
        NodeKind::File => 1,
        NodeKind::Dir => 2,
    });
}

fn decode_dir_entry(d: &mut Decoder<'_>) -> Result<DirEntry> {
    let parent = InodeId(d.get_u64()?);
    let inode = InodeId(d.get_u64()?);
    let name = d.get_string()?;
    let kind = match d.get_u8()? {
        1 => NodeKind::File,
        2 => NodeKind::Dir,
        _ => return Err(VfsError::CorruptLog("invalid dir entry kind".into())),
    };
    Ok(DirEntry { parent, inode, name, kind })
}

