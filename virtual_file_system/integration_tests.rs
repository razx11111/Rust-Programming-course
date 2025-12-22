use std::fs::{self, OpenOptions};
use virtual_file_system::no_sql::{read_next_record, write_header, write_record};
use virtual_file_system::structs::{InodeId, InodeSnapshot, Metadata, NodeKind, Record, Timestamp};
use virtual_file_system::Vfs;

#[test]
fn record_roundtrip_inode_alloc() {
    let path = "target/test_log.vfs";
    let _ = std::fs::remove_file(path);

    let mut f = match OpenOptions::new().create(true).read(true).write(true).open(path) {
        Ok(file) => file,
        Err(e) => panic!("failed to create test log file: {:?}", e),
    };

    match write_header(&mut f, 4096, InodeId(1)){
        Ok(_) => {},
        Err(e) => panic!("failed to write header: {:?}", e),
    };

    let now = Timestamp::now();
    let snap = InodeSnapshot {
        id: InodeId(1),
        parent: None,
        name: "".to_string(),
        kind: NodeKind::Dir,
        metadata: Metadata { size: 0, created_at: now, modified_at: now },
        extents: vec![],
    };

    match write_record(&mut f, &Record::InodeAlloc(snap)) {
        Ok(_) => {},
        Err(e) => panic!("failed to write record: {:?}", e),
    };

    let off:u32 = 8 + 4 + 4 + 8;
    let got = match read_next_record(&mut f, off as u64) {
        Ok(Some((rec, _))) => rec,
        Ok(None) => panic!("no record found"),
        Err(e) => panic!("failed to read record: {:?}", e),
    };

    match got {
        Record::InodeAlloc(s) => assert_eq!(s.id.0, 1),
        _ => panic!("wrong record"),
    }
}

#[test]
fn can_init_and_reopen() {
    let path = "target/mount_init.vfs";
    let _ = std::fs::remove_file(path);

    let _v1 = Vfs::open(path).expect("init");
    let _v2 = Vfs::open(path).expect("reopen");
}
