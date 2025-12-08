#[derive(Debug)]
pub enum VfsFileType {
    /// plain file
    File,
    /// director
    Directory,
}

#[derive(Debug)]
pub struct VfsMetadata {
    pub file_type: VfsFileType,
    pub len: u64,
}

#[derive(Debug)]
struct VFS {
    fs: Box<dyn FileSystem>,
}

pub struct VfsPath {
    path: String,
    fs: Arc<VFS>, //pentru thread safety cica idk
}