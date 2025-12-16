use crc32fast::Hasher;
use crate::structs::{VfsError, Result};
pub struct Encoder {
    buf: Vec<u8> 
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
        let end = self.poz.checked_add(n).ok_or_else(|| {
            VfsError::Corrupt("decoder position overflow".to_string())
        })?;
        if end > self.input.len() {
            return Err(VfsError::Corrupt("unexpected EOF while decoding".to_string()));
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
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
            b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15],
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
            .map_err(|_| VfsError::Corrupt("invalid utf-8 string in snapshot".to_string()))
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