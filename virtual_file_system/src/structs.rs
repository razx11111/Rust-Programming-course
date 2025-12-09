use std::time::SystemTime;
struct InodeId(u64);

enum NodeKind {
    File,
    Dir,
}

struct Inode {
    id: InodeId,
    parent: Option<InodeId>,
    name: String,
    kind: NodeKind,
    size: u64,
    created_at: SystemTime,
    modified_at: SystemTime,
    data: FileData,
} 

enum FileData {
    Empty,
    Extents(Vec<Extent>),
}

struct Extent {
    offset: u64,
    len: u64,
}
