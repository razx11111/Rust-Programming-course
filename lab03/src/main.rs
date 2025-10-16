use core::panic;

enum Panica {
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
    for i in x + 1..u16::MAX { // clippy spune sa fac asa: (x + 1..u16::MAX).find(|&i| prime(i)) but NOO thanks
        if prime(i) {
            return Some(i);
        }
    }
    None
}

fn add_check(a:u32, b:u32) -> u32 {
    if (a as u64 + b as u64) > u32::MAX as u64 {
        panic!("too much");
    } else {
        a + b
    }
}

fn res_mul_check(a:u32, b:u32) -> Result<u32, String> {
    if (a as u64 * b as u64) > u32::MAX as u64 {
        Err("too much".to_string())
    } 
    else {
        Ok(a * b)
    }
}

fn res_add_check(a:u32, b:u32) -> Result<u32, String> {
    if (a as u64 + b as u64) > u32::MAX as u64 {
        Err("too much".to_string())
    } else {
        Ok(a + b)
    }
}

fn mul_check(a:u32, b:u32) -> u32 {
    if (a as u64 * b as u64) > u32::MAX as u64 {
        panic!("too much");
    } else {
        a * b
    }
}

fn to_uppercase(c: char ) -> Result<char, Panica> {
    if !c.is_alphabetic() {
        Err(Panica::NotLetter)
    } else if c.is_lowercase() {
        Ok(c.to_ascii_uppercase())
    } else {
        Ok(c)
    }
}

fn to_lowercase(c: char ) -> Result<char, Panica> {
    if !c.is_alphabetic() {
        Err(Panica::NotLetter)
    } else if c.is_uppercase() {
        Ok(c.to_ascii_lowercase())
    } else {
        Ok(c)
    }
}

fn print_char(c: char ) -> Result<(), Panica> {
    if !c.is_ascii() {
        Err(Panica::NotAscii)
    } else if !c.is_control() {
        Err(Panica::NonPrintable)
    } else {
        print!("{}", c);
        Ok(())
    }
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
    println!("\n=== Testing Addition Operations ===");
    // Test add_check
    println!("Testing add_check:");
    let success = add_check(10, 20);
    println!("Success: 10 + 20 = {}", success);
    
    println!("\nTesting add_check overflow:");
    let result = std::panic::catch_unwind(|| add_check(u32::MAX, 1));
    println!("Caught panic: {}", result.is_err());



    // Test res_add_check
    println!("\nTesting res_add_check:");
    match res_add_check(10, 20) {
        Ok(result) => println!("Success: 10 + 20 = {}", result),
        Err(e) => println!("Error: {:?}", e),
    }

    match res_add_check(u32::MAX, 1) {
        Ok(result) => println!("Success: {}", result),
        Err(e) => println!("Expected overflow error: {:?}", e),
    }

    println!("\n=== Testing Multiplication Operations ===");
    // Test mul_check
    println!("Testing mul_check:");
    let success = mul_check(10, 20);
    println!("Success: 10 * 20 = {}", success);

    println!("\nTesting mul_check overflow:");
    let result = std::panic::catch_unwind(|| mul_check(u32::MAX, 2));
    println!("Caught panic: {}", result.is_err());

    // Test res_mul_check
    println!("\nTesting res_mul_check:");
    match res_mul_check(10, 20) {
        Ok(result) => println!("Success: 10 * 20 = {}", result),
        Err(e) => println!("Error: {:?}", e),
    }

    match res_mul_check(u32::MAX, 2) {
        Ok(result) => println!("Success: {}", result),
        Err(e) => println!("Expected overflow error: {:?}", e),
    }
}
