use crate::VfsError;
use crate::structs::*;
use crc32fast::Hasher;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};

const RECORD_MAGIC: &[u8; 4] = b"VFSR";
const HEADER_MAGIC: &[u8; 8] = &[67u8, 67u8, 67u8, 67u8, 67u8, 67u8, 67u8, 67u8];
const VERSION: u32 = 1;
const HEADER_LEN: u64 = 24; //aproape cum aveam pt superblock 8 magic 4 version 4 bsize 8 root

pub struct Encoder {
    buf: Vec<u8>,
}

impl Default for Encoder {
    fn default() -> Self {
        Self::new()
    }
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
        Some(p) => {
            e.put_u8(1);
            e.put_u64(p.0);
        }
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
        metadata: Metadata {
            size,
            created_at,
            modified_at,
        },
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
    Ok(DirEntry {
        parent,
        inode,
        name,
        kind,
    })
}

pub fn write_record(file: &mut File, record: &Record) -> Result<u64> {
    let mut e = Encoder::new();

    match record {
        Record::InodeAlloc(snap) => {
            e.put_u8(1);
            encode_inode_snapshot(&mut e, snap);
        }
        Record::DirEntryAdd { entry } => {
            e.put_u8(2);
            encode_dir_entry(&mut e, entry);
        }
        Record::Truncate { inode, len } => {
            e.put_u8(4);
            encode_truncate(&mut e, *inode, *len);
        }
        Record::SetTimes {
            inode,
            created_at,
            modified_at,
        } => {
            e.put_u8(5);
            encode_set_times(&mut e, *inode, created_at, modified_at);
        }
        Record::DirEntryRemove { parent, name, inode } => {
            e.put_u8(6);
            encode_dir_entry_remove(&mut e, *parent, name, *inode);
        }
        Record::Rename { inode, old_parent, new_parent, old_name, new_name } => {
            e.put_u8(7);
            encode_rename(&mut e, *inode, *old_parent, *new_parent, old_name, new_name);
        }
        _ => {
            return Err(VfsError::CorruptLog(
                "write_record: record not implemented".into(),
            ));
        }
    }

    let payload = e.into_inner();
    let payload_len = payload.len() as u64;

    let mut scratch = Vec::with_capacity(12 + payload.len());
    scratch.extend_from_slice(RECORD_MAGIC);
    scratch.extend_from_slice(&payload_len.to_le_bytes());
    scratch.extend_from_slice(&payload);

    let header_crc = crc32(&scratch);

    let off = file.seek(SeekFrom::End(0))?;
    file.write_all(&scratch)?;
    file.write_all(&header_crc.to_le_bytes())?;
    file.flush()?;

    Ok(off)
}

pub fn read_next_record(file: &mut File, offset: u64) -> Result<Option<(DecodedRecord, u64)>> {
    file.seek(SeekFrom::Start(offset))?;

    let mut magic = [0u8; 4];
    if file.read_exact(&mut magic).is_err() {
        return Ok(None);
    }
    if &magic != RECORD_MAGIC {
        return Err(VfsError::CorruptLog("bad record magic".into()));
    }

    let mut len_buf = [0u8; 8];
    file.read_exact(&mut len_buf)?;
    let rec_len = u64::from_le_bytes(len_buf);

    // poziția imediat după rec_len
    let record_body_start = offset + 4 + 8;

    // citim tag-ul (1 byte)
    let mut tag_buf = [0u8; 1];
    if file.read_exact(&mut tag_buf).is_err() {
        return Ok(None);
    }
    let tag = tag_buf[0];

    match tag {
        1 | 2 | 4 | 5 | 6 | 7 => {
            // Pentru record-uri “mici”: citim tot body-ul rămas în memorie
            // Am consumat deja 1 byte (tag), deci mai rămân rec_len - 1 bytes
            let remaining = (rec_len as usize)
                .checked_sub(1)
                .ok_or_else(|| VfsError::CorruptLog("record len underflow".into()))?;

            let mut rest = vec![0u8; remaining];
            if file.read_exact(&mut rest).is_err() {
                return Ok(None);
            }

            // Reconstruim payload complet (tag + rest)
            let mut payload = Vec::with_capacity(1 + rest.len());
            payload.push(tag);
            payload.extend_from_slice(&rest);

            // Citim CRC-ul de la final (4 bytes)
            let mut crc_buf = [0u8; 4];
            if file.read_exact(&mut crc_buf).is_err() {
                return Ok(None);
            }
            let expected_crc = u32::from_le_bytes(crc_buf);

            // Verificăm CRC: MAGIC + LEN + PAYLOAD
            let mut scratch = Vec::with_capacity(4 + 8 + payload.len());
            scratch.extend_from_slice(RECORD_MAGIC);
            scratch.extend_from_slice(&rec_len.to_le_bytes());
            scratch.extend_from_slice(&payload);

            let got_crc = crc32(&scratch);
            if got_crc != expected_crc {
                return Err(VfsError::CorruptLog("crc mismatch".into()));
            }

            let mut d = Decoder::new(&payload);
            let tag2 = d.get_u8()?;
            let record = match tag2 {
                1 => Record::InodeAlloc(decode_inode_snapshot(&mut d)?),
                2 => Record::DirEntryAdd {
                    entry: decode_dir_entry(&mut d)?,
                },
                4 => {
                    let (inode, len) = decode_truncate(&mut d)?;
                    Record::Truncate { inode, len }
                }
                5 => {
                    let (inode, created_at, modified_at) = decode_set_times(&mut d)?;
                    Record::SetTimes {
                        inode,
                        created_at,
                        modified_at,
                    }
                }
                6 => {
                    let (parent, name, inode) = decode_dir_entry_remove(&mut d)?;
                    Record::DirEntryRemove { parent, name, inode }
                }
                7 => {
                    let (inode, old_parent, new_parent, old_name, new_name) = decode_rename(&mut d)?;
                    Record::Rename { inode, old_parent, new_parent, old_name, new_name }
                }
                _ => return Err(VfsError::CorruptLog("unexpected tag".into())),
            };
            if !d.is_eof() {
                return Err(VfsError::CorruptLog("trailing bytes".into()));
            }

            let next_offset = record_body_start + rec_len + 4;
            Ok(Some((
                DecodedRecord {
                    record,
                    data_payload_offset: None,
                },
                next_offset,
            )))
        }

        3 => {
            // DataWrite: body = [tag][inode u64][logical u64][len u64][data_crc u32][header_crc u32][data bytes]
            // Am citit deja tag. Mai citim restul header-ului mic:

            let mut hdr = [0u8; 28];
            if file.read_exact(&mut hdr).is_err() {
                return Ok(None);
            }

            let inode = InodeId(u64::from_le_bytes(match hdr[0..8].try_into() {
                Ok(b) => b,
                Err(_) => return Err(VfsError::CorruptLog("invalid inode bytes".into())),
            }));
            let logical_offset = u64::from_le_bytes(match hdr[8..16].try_into() {
                Ok(b) => b,
                Err(_) => return Err(VfsError::CorruptLog("invalid logical offset bytes".into())),
            });
            let len = u64::from_le_bytes(match hdr[16..24].try_into() {
                Ok(b) => b,
                Err(_) => return Err(VfsError::CorruptLog("invalid len bytes".into())),
            });
            let data_crc = u32::from_le_bytes(match hdr[24..28].try_into() {
                Ok(b) => b,
                Err(_) => return Err(VfsError::CorruptLog("invalid data crc bytes".into())),
            });

            // header_crc (4 bytes)
            let mut crc_buf = [0u8; 4];
            if file.read_exact(&mut crc_buf).is_err() {
                return Ok(None);
            }
            let expected_header_crc = u32::from_le_bytes(crc_buf);

            // Reconstituim “payload mic” (tag + inode + logical + len + data_crc) și verificăm CRC
            let mut scratch = Vec::with_capacity(1 + 28);
            scratch.push(3);
            scratch.extend_from_slice(&hdr);

            let got_header_crc = crc32(&scratch);
            if got_header_crc != expected_header_crc {
                return Ok(None);
            }

            // AICI este punctul important:
            // poziția curentă în fișier este exact începutul datelor
            let data_payload_offset = file.stream_position().map_err(VfsError::Io)?;

            // verificăm dacă datele există complet (altfel crash-tail => stop replay)
            let end = file.seek(SeekFrom::End(0))?;
            let need_end = data_payload_offset.saturating_add(len);
            if need_end > end {
                return Ok(None);
            }

            // sărim peste data bytes (fără să le citim)
            file.seek(SeekFrom::Start(need_end))?;

            let record = Record::DataWrite {
                inode,
                logical_offset,
                len,
                checksum: data_crc,
            };

            let next_offset = record_body_start + rec_len;
            Ok(Some((
                DecodedRecord {
                    record,
                    data_payload_offset: Some(data_payload_offset),
                },
                next_offset,
            )))
        }
        _ => Err(VfsError::CorruptLog("unknown record tag".into())),
    }
}

pub struct DecodedRecord {
    pub record: crate::structs::Record,
    pub data_payload_offset: Option<u64>,
}

pub fn write_data_write_record<W: Write + Seek>(
    w: &mut W,
    inode: InodeId,
    logical_offset: u64,
    data: &[u8],
    scratch: &mut Vec<u8>,
) -> Result<(u32, u64)> {
    const TAG_DATA_WRITE: u8 = 3;

    // payload mic (fără data bytes)
    scratch.clear();
    scratch.push(TAG_DATA_WRITE);
    scratch.extend_from_slice(&inode.0.to_le_bytes());
    scratch.extend_from_slice(&logical_offset.to_le_bytes());
    scratch.extend_from_slice(&(data.len() as u64).to_le_bytes());

    let data_crc = crc32(data);
    scratch.extend_from_slice(&data_crc.to_le_bytes());

    // CRC peste payload mic
    let header_crc = crc32(scratch);

    // rec_len = payload_mic + header_crc(4) + data_bytes
    let rec_len = scratch.len() as u64 + 4 + data.len() as u64;

    // scriem framing + payload + header_crc
    w.write_all(RECORD_MAGIC).map_err(VfsError::Io)?;
    w.write_all(&rec_len.to_le_bytes()).map_err(VfsError::Io)?;
    w.write_all(scratch).map_err(VfsError::Io)?;
    w.write_all(&header_crc.to_le_bytes())
        .map_err(VfsError::Io)?;

    // NOW get the offset before writing data
    let data_payload_offset = w.stream_position()?;

    w.write_all(data).map_err(VfsError::Io)?;

    Ok((data_crc, data_payload_offset))
}

fn encode_truncate(e: &mut Encoder, inode: crate::structs::InodeId, len: u64) {
    e.put_u64(inode.0);
    e.put_u64(len);
}

fn decode_truncate(d: &mut Decoder<'_>) -> Result<(crate::structs::InodeId, u64)> {
    let inode = crate::structs::InodeId(d.get_u64()?);
    let len = d.get_u64()?;
    Ok((inode, len))
}

fn encode_opt_timestamp(e: &mut Encoder, t: &Option<Timestamp>) {
    match t {
        None => e.put_u8(0),
        Some(v) => {
            e.put_u8(1);
            e.put_i128(v.0);
        }
    }
}

fn decode_opt_timestamp(d: &mut Decoder<'_>) -> Result<Option<Timestamp>> {
    match d.get_u8()? {
        0 => Ok(None),
        1 => Ok(Some(Timestamp(d.get_i128()?))),
        _ => Err(VfsError::CorruptLog("invalid opt timestamp tag".into())),
    }
}

fn encode_set_times(
    e: &mut Encoder,
    inode: InodeId,
    created: &Option<Timestamp>,
    modified: &Option<Timestamp>,
) {
    e.put_u64(inode.0);
    encode_opt_timestamp(e, created);
    encode_opt_timestamp(e, modified);
}

fn decode_set_times(
    d: &mut Decoder<'_>,
) -> Result<(InodeId, Option<Timestamp>, Option<Timestamp>)> {
    let inode = InodeId(d.get_u64()?);
    let created = decode_opt_timestamp(d)?;
    let modified = decode_opt_timestamp(d)?;
    Ok((inode, created, modified))
}

fn encode_dir_entry_remove(e: &mut Encoder, parent: crate::structs::InodeId, name: &str, inode: crate::structs::InodeId) {
    e.put_u64(parent.0);
    e.put_string(name);
    e.put_u64(inode.0);
}

fn decode_dir_entry_remove(d: &mut Decoder<'_>) -> Result<(crate::structs::InodeId, String, crate::structs::InodeId)> {
    let parent = crate::structs::InodeId(d.get_u64()?);
    let name = d.get_string()?;
    let inode = crate::structs::InodeId(d.get_u64()?);
    Ok((parent, name, inode))
}

fn encode_rename(
    e: &mut Encoder,
    inode: InodeId,
    old_parent: InodeId,
    new_parent: crate::structs::InodeId,
    old_name: &str,
    new_name: &str,
) {
    e.put_u64(inode.0);
    e.put_u64(old_parent.0);
    e.put_u64(new_parent.0);
    e.put_string(old_name);
    e.put_string(new_name);
}

fn decode_rename(d: &mut Decoder<'_>,) -> Result<(InodeId, InodeId, InodeId, String,String)> {
    let inode = InodeId(d.get_u64()?);
    let old_parent = InodeId(d.get_u64()?);
    let new_parent = InodeId(d.get_u64()?);
    let old_name = d.get_string()?;
    let new_name = d.get_string()?;
    Ok((inode, old_parent, new_parent, old_name, new_name))
}