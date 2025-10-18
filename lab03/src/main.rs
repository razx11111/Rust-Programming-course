enum Aoleu {
    NotAscii,
    NotDigit,
    NonBase16,
    NotLetter,
    NonPrintable,
}

fn prime(n: u16) -> bool {
    if n == 1 {
        return false;
    }
    if n == 0 {
        return false;
    }
    let mut i: u16 = 2;
    loop {
        if n % i == 0 {
            return false;
        }
        i += 1;
        if i > n.isqrt() {
            break;
        }
    }
    true
}

fn next_prime(x: u16) -> Option<u16> {
    (x + 1..u16::MAX).find(|&i| prime(i))
}

fn add_check(a: u32, b: u32) -> u32 {
    if (a as u64 + b as u64) > u32::MAX as u64 {
        panic!("too much");
    } else {
        a + b
    }
}

fn res_mul_check(a: u32, b: u32) -> Result<u32, String> {
    if (a as u64 * b as u64) > u32::MAX as u64 {
        Err("too much".to_string())
    } else {
        Ok(a * b)
    }
}

fn res_add_check(a: u32, b: u32) -> Result<u32, String> {
    if (a as u64 + b as u64) > u32::MAX as u64 {
        Err("too much".to_string())
    } else {
        Ok(a + b)
    }
}

fn mul_check(a: u32, b: u32) -> u32 {
    if (a as u64 * b as u64) > u32::MAX as u64 {
        panic!("too much");
    } else {
        a * b
    }
}

fn to_uppercase(c: char) -> Result<char, Aoleu> {
    if !c.is_alphabetic() {
        Err(Aoleu::NotLetter)
    } else if c.is_lowercase() {
        Ok(c.to_ascii_uppercase())
    } else {
        Ok(c)
    }
}

fn to_lowercase(c: char) -> Result<char, Aoleu> {
    if !c.is_alphabetic() {
        Err(Aoleu::NotLetter)
    } else if c.is_uppercase() {
        Ok(c.to_ascii_lowercase())
    } else {
        Ok(c)
    }
}

fn print_char(c: char) -> Result<(), Aoleu> {
    if !c.is_ascii() {
        Err(Aoleu::NotAscii)
    } else if !c.is_control() {
        Err(Aoleu::NonPrintable)
    } else {
        print!("{}", c);
        Ok(())
    }
}

fn char_to_number(c: char) -> Result<u32, Aoleu> {
    if !c.is_ascii_digit() {
        Err(Aoleu::NotDigit)
    } else if !c.is_ascii() {
        Err(Aoleu::NotAscii)
    } else {
        Ok(c.to_digit(10).unwrap())
    }
}

fn char_to_numbeer_hex(c: char) -> Result<u32, Aoleu> {
    if !c.is_ascii_hexdigit() {
        Err(Aoleu::NonBase16)
    } else if !c.is_ascii() {
        Err(Aoleu::NotAscii)
    } else {
        Ok(c.to_digit(16).unwrap())
    }
}

fn print_error(e: Aoleu) {
    match e {
        Aoleu::NotAscii => println!("Character is not ASCII"),
        Aoleu::NotDigit => println!("Character is not a digit"),
        Aoleu::NonBase16 => println!("Character is not a base 16 digit"),
        Aoleu::NotLetter => println!("Character is not a letter"),
        Aoleu::NonPrintable => println!("Character is not printable"),
    }
}

//5. functie care verifica ca fiecare cuv aresufixul _gd

fn check_gd(prop: String) -> Result<String, String> {
    for cuv in prop.split_ascii_whitespace() { 
        if cuv[cuv.len()-3..cuv.len()].to_string() != "_gd" {
            return Err("non-valid".to_string());
        }
    }
    Ok(prop.to_string())
    
}

fn main() {
    let mut x: u16 = 65500;
    let mut y: Option<u16> = next_prime(x);
    println!("The next prime after {} is {:?}", x, y);
    while Option::is_some(&y) {
        y = next_prime(x);
        if Option::is_some(&y) {
            x = y.unwrap();
            println!("The next prime after {} is {:?}", x, y);
        }
    }

    println!("Testing add_check:");
    let success = add_check(10, 20);
    println!("Success: 10 + 20 = {}", success);

    println!("\nTesting add_check overflow:");
    let result = std::panic::catch_unwind(|| add_check(u32::MAX, 1));
    println!("Caught panic: {}", result.is_err());

    println!("\nTesting res_add_check:");
    match res_add_check(10, 20) {
        Ok(result) => println!("Success: 10 + 20 = {}", result),
        Err(e) => println!("Error: {:?}", e),
    }

    match res_add_check(u32::MAX, 1) {
        Ok(result) => println!("Success: {}", result),
        Err(e) => println!("Expected overflow error: {:?}", e),
    }

    println!("Testing mul_check:");
    let success = mul_check(10, 20);
    println!("Success: 10 * 20 = {}", success);

    println!("\nTesting mul_check overflow:");
    let result = std::panic::catch_unwind(|| mul_check(u32::MAX, 2));
    println!("Caught panic: {}", result.is_err());

    println!("\nTesting res_mul_check:");
    match res_mul_check(10, 20) {
        Ok(result) => println!("Success: 10 * 20 = {}", result),
        Err(e) => println!("Error: {:?}", e),
    }

    match res_mul_check(u32::MAX, 2) {
        Ok(result) => println!("Success: {}", result),
        Err(e) => println!("Expected overflow error: {:?}", e),
    }

    println!("\nTesting to_uppercase:");
    let test_chars = vec!['a', 'Z', '1', '❤'];
    for c in test_chars {
        match to_uppercase(c) {
            Ok(result) => println!("Success: {} -> {}", c, result),
            Err(e) => {
                print!("Error for '{}': ", c);
                print_error(e);
            }
        }
    }

    println!("\nTesting to_lowercase:");
    let test_chars = vec!['A', 'z', '1', '☺'];
    for c in test_chars {
        match to_lowercase(c) {
            Ok(result) => println!("Success: {} -> {}", c, result),
            Err(e) => {
                print!("Error for '{}': ", c);
                print_error(e);
            }
        }
    }

    println!("\nTesting print_char:");
    let test_chars = vec!['\n', 'x', '\t', '😀'];
    for c in test_chars {
        match print_char(c) {
            Ok(()) => println!(" -> Success"),
            Err(e) => {
                print!("Error for '{}': ", c);
                print_error(e);
            }
        }
    }

    println!("\nTesting char_to_number:");
    let test_chars = vec!['0', '9', 'A', '❤'];
    for c in test_chars {
        match char_to_number(c) {
            Ok(result) => println!("Success: {} -> {}", c, result),
            Err(e) => {
                print!("Error for '{}': ", c);
                print_error(e);
            }
        }
    }

    println!("\nTesting char_to_numbeer_hex:");
    let test_chars = vec!['0', 'F', 'G', '❤'];
    for c in test_chars {
        match char_to_numbeer_hex(c) {
            Ok(result) => println!("Success: {} -> {}", c, result),
            Err(e) => {
                print!("Error for '{}': ", c);
                print_error(e);
            }
        }
    }

    let good = "word_gd another_gd";
    match check_gd(good.to_string()) {
        Ok(s) => println!("Success: {:?}", s),
        Err(e) => println!("Error: {}", e),
    }

    let bad = "ok_gd badword notgd";
    match check_gd(bad.to_string()) {
        Ok(s) => println!("Unexpected success: {:?}", s),
        Err(e) => println!("Expected error: {}", e),
    }
}
