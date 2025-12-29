use virtual_file_system::{Vfs, VfsError};
use std::io::{Read, Write};

fn test_crate() -> std::result::Result<(), VfsError> {
    let mut vfs = Vfs::mount("target/test_crate.vfs")?;
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

fn main() {
    match test_crate() {
        Ok(_) => println!("Test completed successfully."),
        Err(e) => eprintln!("Error during test: {}", e),
    }
}
