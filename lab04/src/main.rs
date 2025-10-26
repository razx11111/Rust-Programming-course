use std::{fs, io};

fn longest_message() -> Result<(), io::Error> {
    let file = std::fs::read_to_string("src/test.txt")?;
    if let Some(line) = file.lines().max_by_key(|line| line.len()) {
        println!("Longest line: {}", line)
    }
    Ok(())
}

fn rot13_transfer() -> Result<(), &'static str> {
    let file: String = std::fs::read_to_string("src/test_P2.txt").unwrap();
    let iter = file.chars();
    for c in iter {
        let new_c = match c {
            'A'..='M' | 'a'..='m' => ((c as u8) + 13) as char,
            'N'..='Z' | 'n'..='z' => ((c as u8) - 13) as char,
            ' ' => ' ',
            _ => return Err("non-ASCII"),
        };
        print!("{}", new_c);
    }
    println!();
    Ok(())
}

fn abbreviation() -> Result<(), io::Error> {
    let mut file = std::fs::read_to_string("src/test_P3.txt")?;
    let mut replace: String = String::from("");
    for word in file.split_whitespace() {
        let whole = match word {
            "pt" => "pentru",
            "ptr" => "pentru",
            "dl" => "domnul",
            "dna" => "doamna",
            _ => word,
        };
        replace.push_str(whole);
        replace.push(' ');
    }
    file = replace;
    println!();
    println!("Expanded text: {}", file);

    Ok(())
}

fn read_hosts() -> Result<(), io::Error> {
    let hosts_path = "src/test_P4.txts";
    let contents = fs::read_to_string(hosts_path)?;
    for line in contents.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut parts = trimmed.split_whitespace();

        if let (Some(ip), Some(hostname)) = (parts.next(), parts.next()) {
            println!("{} => {}", hostname, ip);
        }
    }

    Ok(())
}

fn main() {
    let _m = longest_message();
    let r = rot13_transfer();
    println!("{r:?}");
    let _a = abbreviation();
    let _h = read_hosts();
}
