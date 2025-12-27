use crate::structs::InodeId;
use crate::vfs::Vfs;
use crate::VfsError;
use std::cell::RefCell;
use std::io::{Read, Seek, SeekFrom, Write};
use std::rc::Rc;

#[derive(Debug)]
pub struct VfsFile {
    inner: Rc<RefCell<crate::vfs::Inner>>,
    inode: InodeId,
    cursor: u64,
    writable: bool,
}

impl VfsFile {
    pub(crate) fn new(inner: Rc<RefCell<crate::vfs::Inner>>, inode: InodeId, writable: bool) -> Self {
        Self { inner, inode, cursor: 0, writable }
    }

    fn vfs(&self) -> Vfs {
        Vfs { inner: self.inner.clone() }
    }

    pub fn len(&self) -> Result<u64, VfsError> {
        self.vfs().len(self.inode)
    }

    pub fn is_empty(&self) -> Result<bool, VfsError> {
        Ok(self.len()? == 0)
    }

    pub fn set_len(&mut self, len: u64) -> Result<(), VfsError> {
        self.vfs().truncate(self.inode, len)
    }
}

impl Read for VfsFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let vfs = self.vfs();
        let n = vfs
            .read_at(self.inode, self.cursor, buf)
            .map_err(|e| std::io::Error::other(format!("{e:?}")))?;
        self.cursor += n as u64;
        Ok(n)
    }
}

impl Write for VfsFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if !self.writable {
            return Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "file not opened for writing"));
        }
        let vfs = self.vfs();
        let n = vfs
            .write_at(self.inode, self.cursor, buf)
            .map_err(|e| std::io::Error::other(format!("{e:?}")))?;
        self.cursor += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Seek for VfsFile {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let len = self
            .len()
            .map_err(|e| std::io::Error::other(format!("{e:?}")))?;
        let next = match pos {
            SeekFrom::Start(o) => o,
            SeekFrom::End(d) => {
                if d >= 0 {
                    len.saturating_add(d as u64)
                } else {
                    len.saturating_sub((-d) as u64)
                }
            }
            SeekFrom::Current(d) => {
                if d >= 0 {
                    self.cursor.saturating_add(d as u64)
                } else {
                    self.cursor.saturating_sub((-d) as u64)
                }
            }
        };
        self.cursor = next;
        Ok(self.cursor)
    }
}
