use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timestamp(pub i128);

impl Timestamp {
    pub fn now() -> Self {
        SystemTime::now().into()
    }
}

impl From<SystemTime> for Timestamp {
    fn from(t: SystemTime) -> Self {
        let dur = t.duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);
        let nanos = dur.as_secs() as i128 * 1_000_000_000i128 + dur.subsec_nanos() as i128;
        Timestamp(nanos)
    }
}

impl From<Timestamp> for SystemTime {
    fn from(t: Timestamp) -> Self {
        if t.0 <= 0 {
            return UNIX_EPOCH;
        }
        let secs = (t.0 / 1_000_000_000i128) as u64;
        let nanos = (t.0 % 1_000_000_000i128) as u32;
        UNIX_EPOCH + Duration::new(secs, nanos)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    File,
    Dir,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metadata {
    pub kind: NodeKind,
    pub size: u64,
    pub created: Timestamp,
    pub modified: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    pub path: String,
    pub metadata: Metadata,
}

#[derive(Debug)]
pub enum VfsError {
    NotFound(String),
    AlreadyExists(String),
    NotAFile(String),
    NotADir(String),
    InvalidPath(String),
    Corrupt(String),
    UnsupportedVersion(u32),
    Io(std::io::Error),
}

impl std::fmt::Display for VfsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VfsError::NotFound(p) => write!(f, "not found: {p}"),
            VfsError::AlreadyExists(p) => write!(f, "already exists: {p}"),
            VfsError::NotAFile(p) => write!(f, "not a file: {p}"),
            VfsError::NotADir(p) => write!(f, "not a dir: {p}"),
            VfsError::InvalidPath(p) => write!(f, "invalid path: {p}"),
            VfsError::Corrupt(m) => write!(f, "corrupt vfs: {m}"),
            VfsError::UnsupportedVersion(v) => write!(f, "unsupported version: {v}"),
            VfsError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for VfsError {}

impl From<std::io::Error> for VfsError {
    fn from(e: std::io::Error) -> Self {
        VfsError::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, VfsError>;
