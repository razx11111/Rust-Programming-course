use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::rc::Rc;
use std::io::{Seek, SeekFrom, Read};

use crate::no_sql::*;
use crate::structs::*;
use crate::file_ops::*;

pub struct Vfs {
    pub(crate) inner: Rc<RefCell<Inner>>,
}

pub struct ReadDir {
    entries: Vec<DirEntry>,
    pos: usize,
}

#[derive(Debug)]
pub(crate) struct Inner {
    file: File,
    header: Header,
    next_inode: InodeId,
    inodes: HashMap<InodeId, Inode>,
    children: HashMap<(InodeId, String), InodeId>,
    scratch: Vec<u8>
}

impl Vfs {

    pub(crate) fn read_at(&self, inode: InodeId, off: u64, buf: &mut [u8]) -> Result<usize> {
        self.inner.borrow_mut().read_at(inode, off, buf)
    }

    pub(crate) fn write_at(&self, inode: InodeId, off: u64, buf: &[u8]) -> Result<usize> {
        self.inner.borrow_mut().write_at(inode, off, buf)
    }
    
    pub(crate) fn len(&self, inode: InodeId) -> Result<u64> {
        self.inner.borrow().len(inode)
    }

    pub(crate) fn truncate(&self, inode: InodeId, len: u64) -> Result<()> {
        self.inner.borrow_mut().truncate(inode, len)
    }

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        // backing file pt vfs
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        // fisier gol -> init
        let len = file.metadata()?.len();
        if len == 0 {
            // header + root inode
            let root = InodeId(1);

            // scriem header la începutul fișierului
            write_header(&mut file, DEFAULT_BLOCK_SIZE, root)?;

            // creăm root snapshot (inode alloc)
            let now = Timestamp::now();
            let root_snap = InodeSnapshot {
                id: root,
                parent: None,
                name: "".to_string(),
                kind: NodeKind::Dir,
                metadata: Metadata {
                    size: 0,
                    created_at: now,
                    modified_at: now,
                },
                extents: vec![],
            };

            // IMPORTANT:
            // în log, root-ul devine "prima operație" după header
            write_record(&mut file, &Record::InodeAlloc(root_snap.clone()))?;

            // apoi damn mount în memorie ca și cum am făcut replay
            let header = Header {
                magic: *b"CCCCCCCC",
                version: 1,
                block_size: DEFAULT_BLOCK_SIZE,
                root,
            };

            let mut inner = Inner {
                file,
                header,
                next_inode: InodeId(2), // următorul inode după root
                inodes: HashMap::new(),
                children: HashMap::new(),
                scratch: Vec::new(),
            };

            // aplicăm record-ul root ca să fie consistent cu log-ul
            inner.apply_record(&Record::InodeAlloc(root_snap))?;

            return Ok(Self {
                inner: Rc::new(RefCell::new(inner)),
            });
        }

        // dacă nu e gol citim header și facem replay
        let header = read_header(&mut file)?;

        let mut inner = Inner {
            file,
            header: header.clone(),
            next_inode: InodeId(1), // se va seta din replay 
            inodes: HashMap::new(),
            children: HashMap::new(),
            scratch: Vec::new(),
        };

        inner.mount_replay()?;

        Ok(Self {
            inner: Rc::new(RefCell::new(inner)),
        })
    }

    fn split_path(path: &str) -> Result<Vec<&str>> {
        if path.is_empty() {
            return Err(VfsError::InvalidPath("empty path".into()));
        }
        let parts: Vec<&str> = path.split('/').collect();
        if parts.iter().any(|p| p.is_empty() || *p == "." || *p == "..") {
            return Err(VfsError::InvalidPath(format!("invalid path: {path}")));
        }
        Ok(parts)
    }

    pub fn create_dir(&mut self, path: &str) -> Result<()> {
        let mut inner = self.inner.borrow_mut();
        inner.create_dir(path)
    }

    pub fn read_dir(&self, path: &str) -> Result<ReadDir> {
        let inner = self.inner.borrow();
        inner.read_dir(path)
    }

    pub fn create(&self, path: &str) -> Result<VfsFile> {
        let inode = self.inner.borrow_mut().create_file(path)?;
        Ok(VfsFile::new(self.inner.clone(), inode, true))
    }

    pub fn open_file(&self, path: &str) -> Result<VfsFile> {
        let inode = self.inner.borrow().path_to_inode(path)?;
        let inner = self.inner.borrow();
        let node = inner.inodes.get(&inode)
            .ok_or_else(|| VfsError::NotFound(path.into()))?;
        if node.kind != NodeKind::File {
            return Err(VfsError::NotAFile(path.into()));
        }
        Ok(VfsFile::new(self.inner.clone(), inode, false))
    }

    
}

impl Inner {
    fn mount_replay(&mut self) -> Result<()> {
        // logu incepe dupa header
        // trebuie să fie exact aceeași constantă ca în no_sql
        let mut offset: u64 = 24;

        // mergem record cu record până când read_next_record spune None
        // None = EOF sau tail incomplet 
        while let Some((decoded, next_offset)) = read_next_record(&mut self.file, offset)? {
            self.apply_decoded(decoded)?;
            offset = next_offset;
        }

        // după replay, next_inode trebuie să fie > max inode id
        let mut max_id = 0u64;
        for id in self.inodes.keys() {
            max_id = max_id.max(id.0);
        }
        self.next_inode = InodeId(max_id + 1);

        // validare minimă: root există
        if !self.inodes.contains_key(&self.header.root) {
            return Err(VfsError::CorruptLog("missing root inode after replay".into()));
        }

        Ok(())
    }

    fn apply_decoded(&mut self, decoded: crate::no_sql::DecodedRecord) -> Result<()> {
    match &decoded.record {
        Record::DataWrite { inode, logical_offset, len, .. } => {
            let data_off = decoded.data_payload_offset.ok_or_else(|| {
                VfsError::CorruptLog("DataWrite missing data offset".into())
            })?;

            // găsim inode-ul și adăugăm extent
            let node = self.inodes.get_mut(inode).ok_or_else(|| {
                VfsError::CorruptLog("DataWrite inode missing".into())
            })?;

            node.extents.push(crate::structs::Extent {
                logical_offset: *logical_offset,
                file_offset: data_off,
                len: *len,
            });

            // update size (max)
            let end = logical_offset.saturating_add(*len);
            if end > node.metadata.size {
                node.metadata.size = end;
            }
            // modified_at (opțional: aici sau prin SetTimes record)
            node.metadata.modified_at = Timestamp::now();

            Ok(())
        }
        _ => {
            self.apply_record(&decoded.record)
        }
        }
    }

    fn apply_record(&mut self, rec: &Record) -> Result<()> {
        match rec {
            Record::InodeAlloc(snap) => {
                self.apply_inode_alloc(snap)?;
            }
            Record::DirEntryAdd { entry } => {
                self.apply_dir_entry_add(entry)?;
            }
            // în MVP ignorăm restul (urmează în pașii următori)
            _ => {}
        }
        Ok(())
    }

    fn apply_inode_alloc(&mut self, snap: &InodeSnapshot) -> Result<()> {
        // Reconstruim un Inode in-memory din snapshot
        let inode = Inode {
            id: snap.id,
            parent: snap.parent,
            name: snap.name.clone(),
            kind: snap.kind,
            metadata: snap.metadata.clone(),
            extents: snap.extents.clone(),
        };

        // dacă există deja, e corupție / log inconsistent
        if self.inodes.contains_key(&inode.id) {
            return Err(VfsError::CorruptLog(format!(
                "duplicate inode alloc for {:?}",
                inode.id
            )));
        }

        self.inodes.insert(inode.id, inode);
        Ok(())
    }

    fn apply_dir_entry_add(&mut self, entry: &DirEntry) -> Result<()> {
        // verificăm parent există și e dir
        let parent: &Inode = self
            .inodes
            .get(&entry.parent)
            .ok_or_else(|| VfsError::CorruptLog("direntry add parent missing".into()))?;

        if parent.kind != NodeKind::Dir {
            return Err(VfsError::CorruptLog(
                "direntry add parent is not a dir".into(),
            ));
        }

        // verificăm inode-ul țintă există
        if !self.inodes.contains_key(&entry.inode) {
            return Err(VfsError::CorruptLog(
                "direntry add inode missing".into(),
            ));
        }

        let key = (entry.parent, entry.name.clone());

        // dacă există deja, înseamnă că log-ul încearcă să dubleze același nume
        if self.children.contains_key(&key) {
            return Err(VfsError::CorruptLog(
                "direntry add duplicate name".into(),
            ));
        }

        self.children.insert(key, entry.inode);
        Ok(())
    }

    fn path_to_inode(&self, path: &str) -> Result<InodeId> {
        let parts = Vfs::split_path(path)?;
        let mut cur = self.header.root;

        for name in parts {
            let key = (cur, name.to_string());
            let next = self
                .children
                .get(&key)
                .copied()
                .ok_or_else(|| VfsError::NotFound(path.into()))?;
            cur = next;
        }
        Ok(cur)
    }

    fn create_dir(&mut self, path: &str) -> Result<()> {
        let parts = Vfs::split_path(path)?;
        let (parent_parts, leaf) = parts.split_at(parts.len() - 1);
        let name = leaf[0];

        // parent inode
        let parent = if parent_parts.is_empty() {
            self.header.root
        } else {
            let mut cur = self.header.root;
            for p in parent_parts {
                let key = (cur, (*p).to_string());
                cur = *self.children.get(&key).ok_or_else(|| VfsError::NotFound(path.into()))?;
            }
            cur
        };

        // verificare ca parintele sa fie folder
        let p_inode = self
            .inodes
            .get(&parent)
            .ok_or_else(|| VfsError::CorruptLog("parent inode missing".into()))?;
        if p_inode.kind != NodeKind::Dir {
            return Err(VfsError::NotADir(format!("{path}")));
        }

        // există deja în parent -> AlreadyExists
        let key = (parent, name.to_string());
        if self.children.contains_key(&key) {
            return Err(VfsError::AlreadyExists(path.into()));
        }

        // inode nou pentru director
        let new_id = self.next_inode;
        self.next_inode = InodeId(self.next_inode.0 + 1);

        let now = Timestamp::now();
        let snap = InodeSnapshot {
            id: new_id,
            parent: Some(parent),
            name: name.to_string(),
            kind: NodeKind::Dir,
            metadata: Metadata {
                size: 0,
                created_at: now,
                modified_at: now,
            },
            extents: vec![],
        };

        // scriem record-uri în log 
        // scriem pe disk înainte să modificăm definitiv structurile 
        write_record(&mut self.file, &Record::InodeAlloc(snap.clone()))?;

        let de = DirEntry {
            parent,
            inode: new_id,
            name: name.to_string(),
            kind: NodeKind::Dir,
        };
        write_record(&mut self.file, &Record::DirEntryAdd { entry: de.clone() })?;

        self.apply_record(&Record::InodeAlloc(snap))?;
        self.apply_record(&Record::DirEntryAdd { entry: de })?;

        Ok(())
    }

    fn read_dir(&self, path: &str) -> Result<ReadDir> {
        // determinăm inode-ul directorului "" inseamna ca e root idk daca o sa schimb asta
        let dir_id = if path.is_empty() {
            self.header.root
        } else {
            self.path_to_inode(path)?
        };

        // verificăm că e director
        let inode = self
            .inodes
            .get(&dir_id)
            .ok_or_else(|| VfsError::NotFound(path.into()))?;

        if inode.kind != NodeKind::Dir {
            return Err(VfsError::NotADir(path.into()));
        }

        // colectăm toate intrările cu parent == dir_id
        let mut entries = Vec::new();
        for ((parent, name), child) in self.children.iter() {
            if *parent == dir_id {
                let child_inode = self
                    .inodes
                    .get(child)
                    .ok_or_else(|| VfsError::CorruptLog("child inode missing".into()))?;

                entries.push(DirEntry {
                    parent: dir_id,
                    inode: *child,
                    name: name.clone(),
                    kind: child_inode.kind,
                });
            }
        }

        // sortăm pentru rezultate deterministe 
        entries.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(ReadDir { entries, pos: 0 })
    }

     fn create_file(&mut self, path: &str) -> Result<InodeId> {
        let parts = Vfs::split_path(path)?;
        let (parent_parts, leaf) = parts.split_at(parts.len() - 1);
        let name = leaf[0];

        let parent = if parent_parts.is_empty() {
            self.header.root
        } else {
            let mut cur = self.header.root;
            for p in parent_parts {
                let key = (cur, (*p).to_string());
                cur = *self.children.get(&key)
                    .ok_or_else(|| VfsError::NotFound(path.into()))?;
            }
            cur
        };

        let p_inode = self.inodes.get(&parent)
            .ok_or_else(|| VfsError::CorruptLog("parent inode missing".into()))?;
        if p_inode.kind != NodeKind::Dir {
            return Err(VfsError::NotADir(path.into()));
        }

        let key = (parent, name.to_string());
        if self.children.contains_key(&key) {
            return Err(VfsError::AlreadyExists(path.into()));
        }

        let new_id = self.next_inode;
        self.next_inode = InodeId(self.next_inode.0 + 1);

        let now = Timestamp::now();
        let snap = InodeSnapshot {
            id: new_id,
            parent: Some(parent),
            name: name.to_string(),
            kind: NodeKind::File,
            metadata: Metadata { size: 0, created_at: now, modified_at: now },
            extents: vec![],
        };

        // persist (write → apply)
        write_record(&mut self.file, &Record::InodeAlloc(snap.clone()))?;
        let de = DirEntry { parent, inode: new_id, name: name.to_string(), kind: NodeKind::File };
        write_record(&mut self.file, &Record::DirEntryAdd { entry: de.clone() })?;

        self.apply_record(&Record::InodeAlloc(snap))?;
        self.apply_record(&Record::DirEntryAdd { entry: de })?;

        Ok(new_id)
    }

    fn write_at(&mut self, inode: InodeId, off: u64, buf: &[u8]) -> Result<usize> {
        let node = self.inodes.get_mut(&inode)
            .ok_or_else(|| VfsError::NotFound(format!("{inode:?}")))?;

        if node.kind != NodeKind::File {
            return Err(VfsError::NotAFile(node.name.clone()));
        }

        // mergem la final (append-only)
        self.file.seek(SeekFrom::End(0))?;

        let (_data_crc, data_payload_offset) =
            write_data_write_record(&mut self.file, inode, off, buf, &mut self.scratch)?;

        // aplicăm în memorie ca la replay
        node.extents.push(crate::structs::Extent {
            logical_offset: off,
            file_offset: data_payload_offset,
            len: buf.len() as u64,
        });

        let end = off.saturating_add(buf.len() as u64);
        if end > node.metadata.size {
            node.metadata.size = end;
        }
        node.metadata.modified_at = Timestamp::now();

        Ok(buf.len())
    }

     fn read_at(&mut self, inode: InodeId, off: u64, buf: &mut [u8]) -> Result<usize> {
        let node = self.inodes.get(&inode)
            .ok_or_else(|| VfsError::NotFound(format!("{inode:?}")))?;

        if node.kind != NodeKind::File {
            return Err(VfsError::NotAFile(node.name.clone()));
        }

        let file_len = node.metadata.size;
        if off >= file_len {
            return Ok(0);
        }

        // pana la eof
        let max_n = (file_len - off) as usize;
        let n = buf.len().min(max_n);

        // buffer target
        for b in &mut buf[..n] { *b = 0; }

        // intervale din buffer care încă trebuie umplute (în coordonate “buffer”)
        let mut holes: Vec<(usize, usize)> = vec![(0, n)];

        // iterăm extents în reverse: ultima scriere are prioritate
        for ex in node.extents.iter().rev() {
            if holes.is_empty() { break; }

            let ex_lo = ex.logical_offset;
            let ex_hi = ex.logical_offset + ex.len;

            let req_lo = off;
            let req_hi = off + n as u64;

            // dacă extent nu intersectează zona cerută, skip
            if ex_hi <= req_lo || ex_lo >= req_hi {
                continue;
            }

            // intersecția în coordonate logice
            let i_lo = ex_lo.max(req_lo);
            let i_hi = ex_hi.min(req_hi);

            // mapare în coordonate buffer
            let buf_lo = (i_lo - off) as usize;
            let buf_hi = (i_hi - off) as usize;

            // acum trebuie să scriem doar bucățile din [buf_lo, buf_hi) care sunt încă “holes”
            let mut new_holes = Vec::new();

            for (h_lo, h_hi) in holes.into_iter() {
                // fără intersecție: păstrăm hole-ul
                if h_hi <= buf_lo || h_lo >= buf_hi {
                    new_holes.push((h_lo, h_hi));
                    continue;
                }

                // dacă există parte înainte de intersecție
                if h_lo < buf_lo {
                    new_holes.push((h_lo, buf_lo));
                }
                // dacă există parte după intersecție
                if buf_hi < h_hi {
                    new_holes.push((buf_hi, h_hi));
                }

                // partea intersectată [max(h_lo,buf_lo), min(h_hi,buf_hi)) trebuie citită din backing file
                let w_lo = h_lo.max(buf_lo);
                let w_hi = h_hi.min(buf_hi);

                let logical_w_lo = off + w_lo as u64; // offset logic real
                let within_extent = logical_w_lo - ex.logical_offset; // offset în interiorul extentului
                let backing_off = ex.file_offset + within_extent;

                self.file.seek(SeekFrom::Start(backing_off))?;
                self.file.read_exact(&mut buf[w_lo..w_hi])?;
            }

            holes = new_holes;
        }

        Ok(n)
    }

    fn len(&self, inode: InodeId) -> Result<u64> {
        let node = self.inodes.get(&inode)
            .ok_or_else(|| VfsError::NotFound(format!("{inode:?}")))?;
        if node.kind != NodeKind::File {
            return Err(VfsError::NotAFile(node.name.clone()));
        }
        Ok(node.metadata.size)
    }

    fn truncate(&mut self, inode: InodeId, len: u64) -> Result<()> {
        let node = self.inodes.get_mut(&inode)
            .ok_or_else(|| VfsError::NotFound(format!("{inode:?}")))?;
        if node.kind != NodeKind::File {
            return Err(VfsError::NotAFile(node.name.clone()));
        }

        node.metadata.size = len;
        node.metadata.modified_at = Timestamp::now();

        Ok(())
    }
}

impl Iterator for ReadDir {
    type Item = Result<DirEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.entries.len() {
            return None;
        }
        let e = self.entries[self.pos].clone();
        self.pos += 1;
        Some(Ok(e))
    }
}