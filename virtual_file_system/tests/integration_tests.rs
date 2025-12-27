use std::fs::OpenOptions;
use std::io::{Read, Write};
use virtual_file_system::Vfs;
use virtual_file_system::no_sql::*;
use virtual_file_system::structs::*;
use std::thread::sleep;
use std::time::Duration;

#[test]
fn record_roundtrip_inode_alloc() {
    let path = "target/test_log.vfs";
    let _ = std::fs::remove_file(path);

    let mut f = match OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(path)
    {
        Ok(file) => file,
        Err(e) => panic!("failed to create test log file: {:?}", e),
    };

    match write_header(&mut f, 4096, InodeId(1)) {
        Ok(_) => {}
        Err(e) => panic!("failed to write header: {:?}", e),
    };

    let now = Timestamp::now();
    let snap = InodeSnapshot {
        id: InodeId(1),
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

    match write_record(&mut f, &Record::InodeAlloc(snap)) {
        Ok(_) => {}
        Err(e) => panic!("failed to write record: {:?}", e),
    };

    let off: u32 = 24;
    let got = match read_next_record(&mut f, off as u64) {
        Ok(Some((rec, _))) => rec,
        Ok(None) => panic!("no record found"),
        Err(e) => panic!("failed to read record: {:?}", e),
    };

    match &got.record {
        Record::InodeAlloc(s) => assert_eq!(s.id.0, 1),
        _ => panic!("wrong record"),
    }
}

#[test]
fn can_init_and_reopen() {
    let path = "target/mount_init.vfs";
    let _ = std::fs::remove_file(path);

    let _v1 = Vfs::mount(path).expect("init");
    let _v2 = Vfs::mount(path).expect("reopen");
}

#[test]
fn create_dir_check_reopen() {
    let path = "target/dirs.vfs";
    let _ = std::fs::remove_file(path);

    let mut v1 = match Vfs::mount(path) {
        Err(e) => panic!("init failed: {:?}", e),
        Ok(vfs) => vfs,
    };
    match v1.create_dir("rs") {
        Err(e) => panic!("create_dir failed: {:?}", e),
        Ok(_) => {}
    };

    // reopen -> replay
    let mut v2 = match Vfs::mount(path) {
        Err(e) => panic!("reopen failed: {:?}", e),
        Ok(vfs) => vfs,
    };
    // încercăm să creăm iar -> AlreadyExists
    let err = v2.create_dir("rs").unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("already exists"));
}

#[test]
fn read_dir_lists_children() {
    let path = "target/readdir.vfs";
    let _ = std::fs::remove_file(path);

    let mut v = match Vfs::mount(path) {
        Err(e) => panic!("init failed: {:?}", e),
        Ok(vfs) => vfs,
    };
    match v.create_dir("rs") {
        Err(e) => panic!("create_dir failed: {:?}", e),
        Ok(_) => {}
    };
    match v.create_dir("rs/a") {
        Err(e) => panic!("create_dir failed: {:?}", e),
        Ok(_) => {}
    };
    match v.create_dir("rs/b") {
        Err(e) => panic!("create_dir failed: {:?}", e),
        Ok(_) => {}
    };

    let dir_vec = match v.read_dir("rs") {
        Err(e) => panic!("read_dir failed: {:?}", e),
        Ok(entries) => entries,
    };
    let mut names = vec![];

    for e in dir_vec {
        let e = e.unwrap();
        names.push(e.name);
    }
    names.sort();
    assert_eq!(names, vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn example_like_assignment() -> std::io::Result<()> {
    let path = "target/e2e.vfs";
    let _ = std::fs::remove_file(path);

    let mut vfs = Vfs::mount(path).unwrap();
    vfs.create_dir("rs").unwrap();

    {
        let mut f1 = vfs.create("rs/abc.txt").unwrap();
        let mut f2 = vfs.create("rs/def.txt").unwrap();
        f1.write_all(b"bafta ").unwrap();
        f2.write_all(b"frate").unwrap();
    }

    let vfs2 = Vfs::mount(path).unwrap();

    let mut out = String::new();
    let mut total = String::new();

    for entry in vfs2.read_dir("rs").unwrap() {
        let entry = entry.unwrap();
        out.clear();
        let mut file = vfs2.open_file(&format!("rs/{}", entry.name)).unwrap();
        file.read_to_string(&mut out)?;
        total.push_str(&out);
    }

    assert_eq!(total, "bafta frate");
    Ok(())
}

#[test]
fn truncate_persists() {
    let path = "target/truncate.vfs";
    let _ = std::fs::remove_file(path);

    let mut v = Vfs::mount(path).unwrap();
    v.create_dir("rs").unwrap();

    let mut f = v.create("rs/a.txt").unwrap();
    f.write_all(b"hello world").unwrap();
    f.set_len(5).unwrap();
    drop(f);

    let v2 = Vfs::mount(path).unwrap();
    let mut f2 = v2.open("rs/a.txt").unwrap();

    let mut s = String::new();
    f2.read_to_string(&mut s).unwrap();
    assert_eq!(s, "hello");
}

#[test]
fn set_times_stable_across_reopen() {
    let path = "target/times.vfs";
    let _ = std::fs::remove_file(path);

    let mut v = Vfs::mount(path).unwrap();
    v.create_dir("rs").unwrap();

    // dormim ca să fie clar diferit dacă ar folosi now() la replay
    sleep(Duration::from_millis(5));

    let _v2 = Vfs::mount(path).unwrap();
    
}