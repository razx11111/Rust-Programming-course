use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom};
use std::path::Path;
use std::rc::Rc;

use crate::no_sql::{read_header, read_next_record, write_header, write_record};
use crate::structs::{
    DirEntry, Header, Inode, InodeId, InodeSnapshot, Metadata, NodeKind, Record, Result, Timestamp,
    VfsError, DEFAULT_BLOCK_SIZE,
};

pub struct Vfs {
    pub(crate) inner: Rc<RefCell<Inner>>,
}

pub struct ReadDir {
    entries: Vec<DirEntry>,
    pos: usize,
}

pub(crate) struct Inner {
    file: File,
    header: Header,
    next_inode: InodeId,
    inodes: HashMap<InodeId, Inode>,
    children: HashMap<(InodeId, String), InodeId>,
}

impl Vfs {
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
            };

            // aplicăm record-ul root ca să fie consistent cu log-ul
            inner.apply_record(&Record::InodeAlloc(root_snap))?;

            return Ok(Self {
                inner: Rc::new(RefCell::new(inner)),
            });
        }

        // 3) dacă nu e gol citim header și facem replay
        let header = read_header(&mut file)?;

        let mut inner = Inner {
            file,
            header: header.clone(),
            next_inode: InodeId(1), // se va seta din replay 
            inodes: HashMap::new(),
            children: HashMap::new(),
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
}

impl Inner {
    fn mount_replay(&mut self) -> Result<()> {
        // logu incepe dupa header
        // trebuie să fie exact aceeași constantă ca în no_sql
        let mut offset: u64 = 24;

        // mergem record cu record până când read_next_record spune None
        // None = EOF sau tail incomplet 
        while let Some((rec, next_offset)) = read_next_record(&mut self.file, offset)? {
            self.apply_record(&rec)?;
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