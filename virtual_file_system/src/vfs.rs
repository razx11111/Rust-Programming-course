use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::rc::Rc;

use crate::file_ops::*;
use crate::no_sql::*;
use crate::structs::*;

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
    scratch: Vec<u8>,
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

    pub fn mount<P: AsRef<Path>>(path: P) -> Result<Self> {
        // backing file pt vfs
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
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
        if parts
            .iter()
            .any(|p| p.is_empty() || *p == "." || *p == "..")
        {
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
        let node = inner
            .inodes
            .get(&inode)
            .ok_or_else(|| VfsError::NotFound(path.into()))?;
        if node.kind != NodeKind::File {
            return Err(VfsError::NotAFile(path.into()));
        }
        Ok(VfsFile::new(self.inner.clone(), inode, false))
    }

    pub fn open(&self, path: &str) -> Result<VfsFile> {
        self.open_file(path)
    }

    pub fn exists(&self, path: &str) -> bool {
        let inner = self.inner.borrow();
        inner.path_to_inode(path).is_ok()
    }
    
    pub fn metadata(&self, path: &str) -> Result<Metadata> {
        let inner = self.inner.borrow();
        inner.metadata(path)
    }

    pub fn remove_file(&mut self, path: &str) -> Result<()> {
        self.inner.borrow_mut().unlink(path, NodeKind::File)
    }

    pub fn remove_dir(&mut self, path: &str) -> Result<()> {
        self.inner.borrow_mut().unlink(path, NodeKind::Dir)
    }

    pub fn rename(&mut self, old_path: &str, new_path: &str) -> Result<()> {
        self.inner.borrow_mut().rename(old_path, new_path)
    }

    pub fn checkpoint(&mut self) -> Result<()> {
        self.inner.borrow_mut().write_checkpoint()
    }

}

impl Inner {
    fn mount_replay(&mut self) -> Result<()> {
        let header_len: u64 = 24;
        let mut offset = header_len;

        // Pass 1: găsim ultimul checkpoint valid
        let mut last_cp: Option<(crate::structs::Checkpoint, u64)> = None;

        while let Some((decoded, next)) = read_next_record(&mut self.file, offset)? {
            if let Record::Checkpoint(cp) = decoded.record {
                last_cp = Some((cp, next));
            }
            offset = next;
        }

        // Dacă am găsit checkpoint:
        if let Some((cp, replay_from)) = last_cp {
            self.load_from_checkpoint(&cp)?;

            // replay doar după checkpoint
            let mut off2 = replay_from;
            while let Some((decoded, next)) = read_next_record(&mut self.file, off2)? {
                self.apply_decoded(decoded)?;
                off2 = next;
            }

            self.recalc_next_inode();

            return Ok(());
        }

        // Dacă nu există checkpoint: replay normal de la început
        self.inodes.clear();
        self.children.clear();

        let mut off3 = header_len;
        while let Some((decoded, next)) = read_next_record(&mut self.file, off3)? {
            self.apply_decoded(decoded)?;
            off3 = next;
        }

        self.recalc_next_inode();

        if !self.inodes.contains_key(&self.header.root) {
            return Err(VfsError::CorruptLog("missing root inode after replay".into()));
        }

        Ok(())
    }

    fn apply_decoded(&mut self, decoded: crate::no_sql::DecodedRecord) -> Result<()> {
        match &decoded.record {
            Record::DataWrite {
                inode,
                logical_offset,
                len,
                ..
            } => {
                let data_off = decoded
                    .data_payload_offset
                    .ok_or_else(|| VfsError::CorruptLog("DataWrite missing data offset".into()))?;

                // găsim inode-ul și adăugăm extent
                let node = self
                    .inodes
                    .get_mut(inode)
                    .ok_or_else(|| VfsError::CorruptLog("DataWrite inode missing".into()))?;

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

                Ok(())
            }
            _ => self.apply_record(&decoded.record),
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
            Record::Truncate { inode, len } => {
                self.apply_truncate(*inode, *len)?;
            }
            Record::SetTimes {
                inode,
                created_at,
                modified_at,
            } => {
                self.apply_set_times(*inode, created_at, modified_at)?;
            }
            Record::DirEntryRemove { parent, name, inode } => {
                self.apply_dir_entry_remove(*parent, name, *inode)?;
            }
            Record::Rename { inode, old_parent, new_parent, old_name, new_name } => {
                self.apply_rename(*inode, *old_parent, *new_parent, old_name, new_name)?;
            }
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
            return Err(VfsError::CorruptLog("direntry add inode missing".into()));
        }

        let key = (entry.parent, entry.name.clone());

        // dacă există deja, înseamnă că log-ul încearcă să dubleze același nume
        if self.children.contains_key(&key) {
            return Err(VfsError::CorruptLog("direntry add duplicate name".into()));
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
                cur = *self
                    .children
                    .get(&key)
                    .ok_or_else(|| VfsError::NotFound(path.into()))?;
            }
            cur
        };

        // verificare ca parintele sa fie folder
        let p_inode = self
            .inodes
            .get(&parent)
            .ok_or_else(|| VfsError::CorruptLog("parent inode missing".into()))?;
        if p_inode.kind != NodeKind::Dir {
            return Err(VfsError::NotADir(path.to_string()));
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

        write_record(&mut self.file, &Record::SetTimes {
            inode: new_id,
            created_at: Some(now),
            modified_at: Some(now),
        })?;
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
                cur = *self
                    .children
                    .get(&key)
                    .ok_or_else(|| VfsError::NotFound(path.into()))?;
            }
            cur
        };

        let p_inode = self
            .inodes
            .get(&parent)
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
            metadata: Metadata {
                size: 0,
                created_at: now,
                modified_at: now,
            },
            extents: vec![],
        };

        // persist (write → apply)
        write_record(&mut self.file, &Record::InodeAlloc(snap.clone()))?;
        let de = DirEntry {
            parent,
            inode: new_id,
            name: name.to_string(),
            kind: NodeKind::File,
        };
        write_record(&mut self.file, &Record::DirEntryAdd { entry: de.clone() })?;
        write_record(&mut self.file, &Record::SetTimes {
            inode: new_id,
            created_at: Some(now),
            modified_at: Some(now),
        })?;
        self.apply_record(&Record::InodeAlloc(snap))?;
        self.apply_record(&Record::DirEntryAdd { entry: de })?;

        Ok(new_id)
    }

    fn write_at(&mut self, inode: InodeId, off: u64, buf: &[u8]) -> Result<usize> {
        let node = self
            .inodes
            .get_mut(&inode)
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

        let now = Timestamp::now();
        write_record(&mut self.file, &Record::SetTimes {
            inode,
            created_at: None,
            modified_at: Some(now),
        })?;
        self.apply_record(&Record::SetTimes {
            inode,
            created_at: None,
            modified_at: Some(now),
        })?;

        Ok(buf.len())
    }

    fn read_at(&mut self, inode: InodeId, off: u64, buf: &mut [u8]) -> Result<usize> {
        let node = self
            .inodes
            .get(&inode)
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
        for b in &mut buf[..n] {
            *b = 0;
        }

        // intervale din buffer care încă trebuie umplute (în coordonate “buffer”)
        let mut holes: Vec<(usize, usize)> = vec![(0, n)];

        // iterăm extents în reverse: ultima scriere are prioritate
        for ex in node.extents.iter().rev() {
            if holes.is_empty() {
                break;
            }

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
        let node = self
            .inodes
            .get(&inode)
            .ok_or_else(|| VfsError::NotFound(format!("{inode:?}")))?;
        if node.kind != NodeKind::File {
            return Err(VfsError::NotAFile(node.name.clone()));
        }
        Ok(node.metadata.size)
    }

    fn truncate(&mut self, inode: InodeId, len: u64) -> Result<()> {
        // verificăm inode există și e file
        let node = self
            .inodes
            .get(&inode)
            .ok_or_else(|| VfsError::NotFound(format!("{inode:?}")))?;
        if node.kind != NodeKind::File {
            return Err(VfsError::NotAFile(node.name.clone()));
        }

        write_record(&mut self.file, &Record::Truncate { inode, len })?;

        // apply in-memory (aceeași logică ca replay)
        self.apply_record(&Record::Truncate { inode, len })?;

        let now = Timestamp::now();
        write_record(&mut self.file, &Record::SetTimes {
            inode,
            created_at: None,
            modified_at: Some(now),
        })?;
        self.apply_record(&Record::SetTimes {
            inode,
            created_at: None,
            modified_at: Some(now),
        })?;
        Ok(())
    }

    fn apply_truncate(&mut self, inode: InodeId, len: u64) -> Result<()> {
        let node = self
            .inodes
            .get_mut(&inode)
            .ok_or_else(|| VfsError::CorruptLog("truncate inode missing".into()))?;

        if node.kind != NodeKind::File {
            return Err(VfsError::CorruptLog("truncate target not a file".into()));
        }

        node.metadata.size = len;

        Ok(())
    }

    fn apply_set_times( &mut self, inode: InodeId, created_at: &Option<Timestamp>,modified_at: &Option<Timestamp>) -> 
    Result<()> {
        let node = self
            .inodes
            .get_mut(&inode)
            .ok_or_else(|| VfsError::CorruptLog("set_times inode missing".into()))?;

        if let Some(c) = created_at {
            node.metadata.created_at = *c;
        }
        if let Some(m) = modified_at {
            node.metadata.modified_at = *m;
        }
        Ok(())
    }

    fn metadata(&self, path: &str) -> Result<Metadata> {
        let inode_id = if path.is_empty() {
            self.header.root
        } else {
            self.path_to_inode(path)?
        };

        let inode = self
            .inodes
            .get(&inode_id)
            .ok_or_else(|| VfsError::NotFound(path.into()))?;

        Ok(inode.metadata.clone())
    }

    fn apply_dir_entry_remove(&mut self, parent: InodeId, name: &str, inode: InodeId) -> Result<()> {
        // parent trebuie să existe și să fie dir
        let p = self.inodes.get(&parent)
            .ok_or_else(|| VfsError::CorruptLog("direntry remove parent missing".into()))?;
        if p.kind != NodeKind::Dir {
            return Err(VfsError::CorruptLog("direntry remove parent not dir".into()));
        }

        let key = (parent, name.to_string());

        // trebuie să existe entry-ul
        let existing = self.children.get(&key)
            .copied()
            .ok_or_else(|| VfsError::CorruptLog("direntry remove missing entry".into()))?;

        // trebuie să corespundă inode-ului din log
        if existing != inode {
            return Err(VfsError::CorruptLog("direntry remove inode mismatch".into()));
        }

        self.children.remove(&key);
        Ok(())
    }

    fn find_parent_and_leaf(&self, path: &str) -> Result<(InodeId, String)> {
        let parts = Vfs::split_path(path)?;
        let (parent_parts, leaf) = parts.split_at(parts.len() - 1);
        let name = leaf[0].to_string();

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

        Ok((parent, name))
    }

    fn unlink(&mut self, path: &str, expect_kind: NodeKind) -> Result<()> {
        let (parent, name) = self.find_parent_and_leaf(path)?;
        let key = (parent, name.clone());

        let inode = self.children.get(&key)
            .copied()
            .ok_or_else(|| VfsError::NotFound(path.into()))?;

        let node = self.inodes.get(&inode)
            .ok_or_else(|| VfsError::CorruptLog("unlink inode missing".into()))?;

        if node.kind != expect_kind {
            return match expect_kind {
                NodeKind::File => Err(VfsError::NotAFile(path.into())),
                NodeKind::Dir => Err(VfsError::NotADir(path.into())),
            };
        }

        // dacă e dir, trebuie să fie gol (fără copii)
        if expect_kind == NodeKind::Dir {
            let has_child = self.children.keys().any(|(p, _)| *p == inode);
            if has_child {
                return Err(VfsError::InvalidPath("directory not empty".into()));
            }
        }

        // persist
        let rec = Record::DirEntryRemove { parent, name: name.clone(), inode };
        self.file.seek(SeekFrom::End(0))?;
        write_record(&mut self.file, &rec)?;
        self.file.sync_all()?;

        // apply
        self.apply_record(&rec)?;
        Ok(())
    }

    fn apply_rename(
        &mut self,
        inode: InodeId,
        old_parent: InodeId,
        new_parent: InodeId,
        old_name: &str,
        new_name: &str,
    ) -> Result<()> {
        // Validate parents and old entry before getting mutable borrow
        // old_parent / new_parent trebuie să existe și să fie dir
        let op = self.inodes.get(&old_parent)
            .ok_or_else(|| VfsError::CorruptLog("rename old_parent missing".into()))?;
        if op.kind != NodeKind::Dir {
            return Err(VfsError::CorruptLog("rename old_parent not dir".into()));
        }

        let np = self.inodes.get(&new_parent)
            .ok_or_else(|| VfsError::CorruptLog("rename new_parent missing".into()))?;
        if np.kind != NodeKind::Dir {
            return Err(VfsError::CorruptLog("rename new_parent not dir".into()));
        }

        // trebuie să existe vecheaentry și să pointeze la inode
        let old_key = (old_parent, old_name.to_string());
        let existing = self.children.get(&old_key)
            .copied()
            .ok_or_else(|| VfsError::CorruptLog("rename old entry missing".into()))?;
        if existing != inode {
            return Err(VfsError::CorruptLog("rename old entry inode mismatch".into()));
        }

        // noua destinație trebuie să fie liberă (MVP: fără overwrite)
        let new_key = (new_parent, new_name.to_string());
        if self.children.contains_key(&new_key) {
            return Err(VfsError::CorruptLog("rename destination exists".into()));
        }

        // inode trebuie să existe
        let node = self.inodes.get_mut(&inode)
            .ok_or_else(|| VfsError::CorruptLog("rename inode missing".into()))?;

        // mutarea efectivă
        self.children.remove(&old_key);
        self.children.insert(new_key, inode);

        // update inode in-memory
        node.parent = Some(new_parent);
        node.name = new_name.to_string();

        Ok(())
    }
    
    fn rename(&mut self, old_path: &str, new_path: &str) -> Result<()> {
        // old: (old_parent, old_name, inode)
        let (old_parent, old_name) = self.find_parent_and_leaf(old_path)?;
        let old_key = (old_parent, old_name.clone());

        let inode = self.children.get(&old_key)
            .copied()
            .ok_or_else(|| VfsError::NotFound(old_path.into()))?;

        // new: (new_parent, new_name)
        let (new_parent, new_name) = self.find_parent_and_leaf(new_path)?;
        let new_key = (new_parent, new_name.clone());

        // new parent trebuie să existe și să fie dir
        let np = self.inodes.get(&new_parent)
            .ok_or_else(|| VfsError::NotFound(new_path.into()))?;
        if np.kind != NodeKind::Dir {
            return Err(VfsError::NotADir(new_path.into()));
        }

        // destinația trebuie să fie liberă (MVP)
        if self.children.contains_key(&new_key) {
            return Err(VfsError::AlreadyExists(new_path.into()));
        }

        // persist record
        let rec = Record::Rename {
            inode,
            old_parent,
            new_parent,
            old_name: old_name.clone(),
            new_name: new_name.clone(),
        };
        write_record(&mut self.file, &rec)?;

        // apply in-memory
        self.apply_record(&rec)?;

        // persist SetTimes (modified_at)
        let now = Timestamp::now();
        let times = Record::SetTimes {
            inode,
            created_at: None,
            modified_at: Some(now),
        };
        write_record(&mut self.file, &times)?;
        self.apply_record(&times)?;

        Ok(())
    }

    fn make_checkpoint(&self) -> crate::structs::Checkpoint {
        let mut snaps = Vec::with_capacity(self.inodes.len());
        for inode in self.inodes.values() {
            snaps.push(InodeSnapshot {
                id: inode.id,
                parent: inode.parent,
                name: inode.name.clone(),
                kind: inode.kind,
                metadata: inode.metadata.clone(),
                extents: inode.extents.clone(),
            });
        }

        snaps.sort_by_key(|s| s.id.0);

        Checkpoint {
            next_inode: self.next_inode,
            free_extents: vec![], 
            inodes: snaps,
        }
    }

    fn write_checkpoint(&mut self) -> Result<()> {
        let cp = self.make_checkpoint();
        let rec = Record::Checkpoint(cp);

        write_record(&mut self.file, &rec)?;

        Ok(())
    }

    fn load_from_checkpoint(&mut self, cp: &crate::structs::Checkpoint) -> Result<()> {
        
        self.inodes.clear();
        self.children.clear();

        // reconstruim inodes
        for snap in &cp.inodes {
            let inode = Inode {
                id: snap.id,
                parent: snap.parent,
                name: snap.name.clone(),
                kind: snap.kind,
                metadata: snap.metadata.clone(),
                extents: snap.extents.clone(),
            };
            self.inodes.insert(inode.id, inode);
        }

        // reconstruim children
        for inode in self.inodes.values() {
            if let Some(p) = inode.parent {
                let key = (p, inode.name.clone());

                if self.children.contains_key(&key) {
                    return Err(VfsError::CorruptLog("checkpoint has duplicate (parent,name)".into()));
                }

                self.children.insert(key, inode.id);
            }
        }

        self.next_inode = cp.next_inode; // din checkpoint

        if !self.inodes.contains_key(&self.header.root) {
            return Err(VfsError::CorruptLog("checkpoint missing root inode".into()));
        }

        Ok(())
    }

    fn recalc_next_inode(&mut self) {
        let mut max_id = 0u64;
        for id in self.inodes.keys() {
            max_id = max_id.max(id.0);
        }
        self.next_inode = InodeId(max_id + 1);
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
