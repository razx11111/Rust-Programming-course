use crate::structs::{DirEntry, Result};

pub struct Vfs {
    _private: (),
}

pub struct ReadDir {
    _private: (),
}

impl Iterator for ReadDir {
    type Item = Result<DirEntry>;
    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

impl Vfs {
    pub fn open<P: AsRef<std::path::Path>>(_path: P) -> Result<Self> {
        Ok(Self { _private: () })
    }

    pub fn create_dir(&mut self, _path: &str) -> Result<()> {
        Ok(())
    }

    pub fn read_dir(&self, _path: &str) -> Result<ReadDir> {
        Ok(ReadDir { _private: () })
    }
}
