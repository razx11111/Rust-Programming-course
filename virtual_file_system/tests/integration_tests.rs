use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::thread::sleep;
use std::time::Duration;
use virtual_file_system::Vfs;
use virtual_file_system::no_sql::*;
use virtual_file_system::structs::*;

#[test]
fn record_roundtrip_inode_alloc() -> Result<()> {
    let path = "target/test_log.vfs";
    let _ = std::fs::remove_file(path);

    let mut f = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(path)?;

    write_header(&mut f, 4096, InodeId(1))?;

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

    write_record(&mut f, &Record::InodeAlloc(snap))?;

    let off: u32 = 24;
    let (got, _) = read_next_record(&mut f, off as u64)?
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "no record found"))?;

    match &got.record {
        Record::InodeAlloc(s) => assert_eq!(s.id.0, 1),
        _ => panic!("wrong record"),
    }
    Ok(())
}

#[test]
fn can_init_and_reopen() -> Result<()> {
    let path = "target/mount_init.vfs";
    let _ = std::fs::remove_file(path);

    let _v1 = Vfs::mount(path)?;
    let _v2 = Vfs::mount(path)?;
    Ok(())
}

#[test]
fn create_dir_check_reopen() -> Result<()> {
    let path = "target/dirs.vfs";
    let _ = std::fs::remove_file(path);

    let mut v1 = Vfs::mount(path)?;
    v1.create_dir("rs")?;

    // reopen -> replay
    let mut v2 = Vfs::mount(path)?;
    // încercăm să creăm iar -> AlreadyExists
    let err = v2.create_dir("rs").unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("already exists"));
    Ok(())
}

#[test]
fn read_dir_lists_children() -> Result<()> {
    let path = "target/readdir.vfs";
    let _ = std::fs::remove_file(path);

    let mut v = Vfs::mount(path)?;
    v.create_dir("rs")?;
    v.create_dir("rs/a")?;
    v.create_dir("rs/b")?;

    let dir_vec = v.read_dir("rs")?;
    let mut names = vec![];

    for e in dir_vec {
        let e = e?;
        names.push(e.name);
    }
    names.sort();
    assert_eq!(names, vec!["a".to_string(), "b".to_string()]);
    Ok(())
}

#[test]
fn example_like_assignment() -> Result<()> {
    let path = "target/e2e.vfs";
    let _ = std::fs::remove_file(path);

    let mut vfs = Vfs::mount(path)?;
    vfs.create_dir("rs")?;

    {
        let mut f1 = vfs.create("rs/abc.txt")?;
        let mut f2 = vfs.create("rs/def.txt")?;
        f1.write_all(b"bafta ")?;
        f2.write_all(b"frate")?;
    }

    let vfs2 = Vfs::mount(path)?;

    let mut out = String::new();
    let mut total = String::new();

    for entry in vfs2.read_dir("rs")? {
        let entry = entry?;
        out.clear();
        let mut file = vfs2.open_file(&format!("rs/{}", entry.name))?;
        file.read_to_string(&mut out)?;
        total.push_str(&out);
    }

    assert_eq!(total, "bafta frate");
    Ok(())
}

#[test]
fn truncate_persists() -> Result<()> {
    let path = "target/truncate.vfs";
    let _ = std::fs::remove_file(path);

    let mut v = Vfs::mount(path)?;
    v.create_dir("rs")?;

    let mut f = v.create("rs/a.txt")?;
    f.write_all(b"hello world")?;
    f.set_len(5)?;
    drop(f);

    let v2 = Vfs::mount(path)?;
    let mut f2 = v2.open("rs/a.txt")?;

    let mut s = String::new();
    f2.read_to_string(&mut s)?;
    assert_eq!(s, "hello");
    Ok(())
}

#[test]
fn modified_time_persists_and_is_not_mount_time() -> Result<()> {
    let path = "target/meta_times.vfs";
    let _ = std::fs::remove_file(path);

    let mut v = Vfs::mount(path)?;
    v.create_dir("rs")?;

    let t0 = v.metadata("rs")?.modified_at;

    sleep(Duration::from_millis(5));

    // write într-un fișier nou
    let mut f = v.create("rs/a.txt")?;
    f.write_all(b"hello")?;
    drop(f);

    let t1 = v.metadata("rs/a.txt")?.modified_at;
    assert!(t1 >= t0);

    // reopen: timpii trebuie să rămână aceiași
    sleep(Duration::from_millis(5));
    let v2 = Vfs::mount(path)?;

    let t1b = v2.metadata("rs/a.txt")?.modified_at;
    assert_eq!(t1b, t1);
    Ok(())
}

#[test]
fn size_persists_after_truncate() -> Result<()> {
    let path = "target/meta_size.vfs";
    let _ = std::fs::remove_file(path);

    let mut v = Vfs::mount(path)?;
    v.create_dir("rs")?;

    let mut f = v.create("rs/a.txt")?;
    f.write_all(b"hello world")?;
    f.set_len(5)?;
    drop(f);

    let m1 = v.metadata("rs/a.txt")?;
    assert_eq!(m1.size, 5);

    let v2 = Vfs::mount(path)?;
    let m2 = v2.metadata("rs/a.txt")?;
    assert_eq!(m2.size, 5);
    Ok(())
}

#[test]
fn remove_file_persists() -> Result<()> {
    let path = "target/remove_file.vfs";
    let _ = std::fs::remove_file(path);

    let mut v = Vfs::mount(path)?;
    v.create_dir("rs")?;

    let mut f = v.create("rs/a.txt")?;
    f.write_all(b"hello")?;
    drop(f);

    assert!(v.exists("rs/a.txt"));

    v.remove_file("rs/a.txt")?;
    assert!(!v.exists("rs/a.txt"));

    // reopen: încă nu există
    let v2 = Vfs::mount(path)?;
    assert!(!v2.exists("rs/a.txt"));
    Ok(())
}

#[test]
fn remove_dir_only_if_empty() -> Result<()> {
    let path = "target/remove_dir.vfs";
    let _ = std::fs::remove_file(path);

    let mut v = Vfs::mount(path)?;
    v.create_dir("rs")?;
    v.create_dir("rs/sub")?;

    // rs nu e gol => error
    assert!(v.remove_dir("rs").is_err());

    // sub e gol => ok
    v.remove_dir("rs/sub")?;
    assert!(!v.exists("rs/sub"));

    // reopen persist
    let v2 = Vfs::mount(path)?;
    assert!(!v2.exists("rs/sub"));
    Ok(())
}
#[test]
fn rename_in_same_dir() -> Result<()> {
    let path = "target/rename1.vfs";
    let _ = std::fs::remove_file(path);

    let mut v = Vfs::mount(path)?;
    v.create_dir("rs")?;

    let mut f = v.create("rs/a.txt")?;
    f.write_all(b"hello")?;
    drop(f);

    v.rename("rs/a.txt", "rs/b.txt")?;

    assert!(!v.exists("rs/a.txt"));
    assert!(v.exists("rs/b.txt"));

    let mut s = String::new();
    v.open("rs/b.txt")?.read_to_string(&mut s)?;
    assert_eq!(s, "hello");
    Ok(())
}

#[test]
fn rename_across_dirs() -> Result<()> {
    let path = "target/rename2.vfs";
    let _ = std::fs::remove_file(path);

    let mut v = Vfs::mount(path)?;
    v.create_dir("a")?;
    v.create_dir("b")?;

    let mut f = v.create("a/x.txt")?;
    f.write_all(b"data")?;
    drop(f);

    v.rename("a/x.txt", "b/y.txt")?;

    assert!(!v.exists("a/x.txt"));
    assert!(v.exists("b/y.txt"));

    let mut s = String::new();
    v.open("b/y.txt")?.read_to_string(&mut s)?;
    assert_eq!(s, "data");
    Ok(())
}

#[test]
fn rename_persists_after_reopen() -> Result<()> {
    let path = "target/rename3.vfs";
    let _ = std::fs::remove_file(path);

    {
        let mut v = Vfs::mount(path)?;
        v.create_dir("rs")?;
        let _ = v.create("rs/a.txt")?;
        v.rename("rs/a.txt", "rs/b.txt")?;
    }

    let v2 = Vfs::mount(path)?;
    assert!(!v2.exists("rs/a.txt"));
    assert!(v2.exists("rs/b.txt"));
    Ok(())
}

#[test]
fn checkpoint_reopen_ok() -> Result<()> {
    let path = "target/checkpoint1.vfs";
    let _ = std::fs::remove_file(path);

    {
        let mut v = Vfs::mount(path)?;
        v.create_dir("rs")?;

        let mut f = v.create("rs/a.txt")?;
        f.write_all(b"hello")?;
        drop(f);

        v.checkpoint()?;
    }

    let v2 = Vfs::mount(path)?;

    let mut s = String::new();
    v2.open("rs/a.txt")?.read_to_string(&mut s)?;
    assert_eq!(s, "hello");
    Ok(())
}

#[test]
fn checkpoint_then_more_ops() -> Result<()> {
    let path = "target/checkpoint2.vfs";
    let _ = std::fs::remove_file(path);

    {
        let mut v = Vfs::mount(path)?;
        v.create_dir("rs")?;

        let mut f = v.create("rs/a.txt")?;
        f.write_all(b"hello")?;
        drop(f);

        v.checkpoint()?;

        // Operație DUPĂ checkpoint
        v.rename("rs/a.txt", "rs/b.txt")?;
    }

    let v2 = Vfs::mount(path)?;
    assert!(!v2.exists("rs/a.txt"));
    assert!(v2.exists("rs/b.txt"));

    let mut s = String::new();
    v2.open("rs/b.txt")?.read_to_string(&mut s)?;
    assert_eq!(s, "hello");
    Ok(())
}

#[test]
fn test_cerinta() -> Result<()> {
    let mut vfs = Vfs::mount("target/test_cerinta.vfs")?;
    let _ = std::fs::remove_file("target/test_cerinta.vfs");

    vfs.create_dir("rs")?;
    {
        let mut f1 = vfs.create("rs/abc.txt")?;
        let mut f2 = vfs.create("rs/def.txt")?;

        f1.write_all(b"hello")?;
        f2.write_all(b"world")?;
    }

    let mut data = String::new();
    for entry in vfs.read_dir("rs")? {
        let entry = entry?;
        data.clear();

        let mut file = vfs.open(format!("rs/{}", entry.name).as_str())?;
        file.read_to_string(&mut data)?;

        print!("{}", data);
    }
    println!();
    Ok(())
}
