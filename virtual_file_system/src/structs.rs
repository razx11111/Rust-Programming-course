use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const DEFAULT_BLOCK_SIZE: u32 = 4096;

/// normalized timestamp representation stored as UNIX nanoseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(pub i128);

impl Timestamp {
    pub fn now() -> Self {
        SystemTime::now().into()
    }
}

impl From<SystemTime> for Timestamp {
    fn from(value: SystemTime) -> Self {
        match value.duration_since(UNIX_EPOCH) {
            Ok(dur) => Timestamp(dur.as_nanos() as i128),
            Err(err) => {
                let dur: Duration = err.duration();
                Timestamp(-(dur.as_nanos() as i128))
            }
        }
    }
}

impl From<Timestamp> for SystemTime {
    fn from(value: Timestamp) -> Self {
        if value.0 >= 0 {
            UNIX_EPOCH + Duration::from_nanos(value.0 as u64)
        } else {
            UNIX_EPOCH - Duration::from_nanos((-value.0) as u64)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InodeId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    File,
    Dir,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metadata {
    pub size: u64,
    pub created_at: Timestamp,
    pub modified_at: Timestamp,
}

/// logical range pointing to bytes inside the backing file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extent {
    /// logical offset within the file.
    pub logical_offset: u64,
    /// offset inside the backing store where this extent begins.
    pub file_offset: u64,
    pub len: u64,
}

/// file data is stored as a set of extents.
pub type ExtentList = Vec<Extent>;

/// on-disk directory entry metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    pub parent: InodeId,
    pub inode: InodeId,
    pub name: String,
    pub kind: NodeKind,
}

/// in-memory inode representation reconstructed during mount/replay.
#[derive(Debug, Clone)]
pub struct Inode {
    pub id: InodeId,
    pub parent: Option<InodeId>,
    pub name: String,
    pub kind: NodeKind,
    pub metadata: Metadata,
    pub extents: ExtentList,
}

/// header persisted at the start of the backing file.
#[derive(Debug, Clone, PartialEq)]
pub struct Header {
    pub magic: [u8; 8],
    pub version: u32,
    pub block_size: u32,
    pub root: InodeId,
}

/// snapshot of the free list and inode table used to accelerate mounts.
#[derive(Debug, Clone, PartialEq)]
pub struct Checkpoint {
    pub next_inode: InodeId,
    pub free_extents: ExtentList,
    pub inodes: Vec<InodeSnapshot>,
}

/// inode snapshot persisted in checkpoints.
#[derive(Debug, Clone, PartialEq)]
pub struct InodeSnapshot {
    pub id: InodeId,
    pub parent: Option<InodeId>,
    pub name: String,
    pub kind: NodeKind,
    pub metadata: Metadata,
    pub extents: ExtentList,
}

///operations persisted in the log
#[derive(Debug, Clone, PartialEq)]
pub enum Record {
    Header(Header),
    Checkpoint(Checkpoint),
    InodeAlloc(InodeSnapshot),
    DirEntryAdd {
        entry: DirEntry,
    },
    DataWrite {
        inode: InodeId,
        logical_offset: u64,
        len: u64,
        checksum: u32,
    },
    Truncate {
        inode: InodeId,
        len: u64,
    },
    SetTimes {
        inode: InodeId,
        created_at: Option<Timestamp>,
        modified_at: Option<Timestamp>,
    },
    Rename {
        inode: InodeId,
        old_parent: InodeId,
        new_parent: InodeId,
        old_name: String,
        new_name: String,
    },
}

#[derive(Debug)]
pub enum VfsError {
    NotFound(String),
    AlreadyExists(String),
    NotAFile(String),
    NotADir(String),
    InvalidPath(String),
    CorruptLog(String),
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
            VfsError::CorruptLog(m) => write!(f, "corrupt log: {m}"),
            VfsError::UnsupportedVersion(v) => write!(f, "unsupported version: {v}"),
            VfsError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl From<std::io::Error> for VfsError {
    fn from(err: std::io::Error) -> Self {
        VfsError::Io(err)
    }
}

pub type Result<T> = std::result::Result<T, VfsError>;
