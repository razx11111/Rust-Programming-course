fn add_chars_n(s: &mut String, caracter: char, num: u8) {
    for _i in 0..num {
        s.push(caracter);
    }
}

fn add_space(s: &mut String, n: u32) {
    for _i in 0..n {
        s.push(' ');
    }
}

fn add_str(mut s: String, str: &str) -> String {
    s += str;
    s
}

fn int_len(mut n: u32) -> u8 {
    let mut len: u8 = 0;
    while n != 0 {
        n /= 10;
        len += 1;
    }
    len
}

fn add_integer(s: &mut String, mut n: i32, is_float: bool) {
    if n < 0 {
        s.push('-');
        n *= -1;
    }

    let mut exp: i32 = 10;
    exp = exp.pow((int_len(n as u32) - 1) as u32);
    let mut count: u8 = 0;

    while exp != 0 {
        if count % 3 == 0 && count != 0 && !is_float {
            s.push('_');
        }
        s.push((((n / exp) % 10) as u8 + b'0') as char);
        exp /= 10;
        count += 1;
    }
}

fn add_float(s: &mut String, mut n: f64) {
    const EPSILON: f64 = 1e-10;

    if n < 0.0 {
        s.push('-');
        n *= -1.0;
    }

    let int_part: i32 = n as i32;
    add_integer(s, int_part, true);
    s.push('.');
    let mut digit: u32;

    n -= int_part as f64;

    for _i in 0..6 {
        // merge si cu mai mult de 6 zecimale
        n *= 10.0;
        digit = (n + EPSILON).floor() as u32; // APARENT DACA ADAUG EPSILON NU MAI ARE ERORI DE PRECIZIE(cred idk)
        s.push((digit as u8 + b'0') as char); // revelatie
        n -= digit as f64;
    }

    while s.ends_with('0') {
        s.pop();
    }
}

fn main() {
    let mut s = String::from("");
    let mut i = 0;
    while i < 26 {
        let c = (i + b'a') as char;
        add_chars_n(&mut s, c, 26 - i);

        i += 1;
    }
    print!("{}", s);

    let mut s1: String = String::from("");

    s1 = add_str(s1, "\n");
    add_space(&mut s1, 40);
    s1 = add_str(s1, "I \u{1F49A} \n");
    add_space(&mut s1, 40);
    s1 = add_str(s1, "RUST. \n");
    s1 = add_str(s1, "Most");
    add_space(&mut s1, " downloaded ".len() as u32);
    s1 = add_str(s1, "crate");
    add_space(&mut s1, " has ".len() as u32);
    add_integer(&mut s1, 306437968, false);
    add_space(&mut s1, " downloads ".len() as u32);
    s1 = add_str(s1, "and");
    add_space(&mut s1, " the ".len() as u32);
    s1 = add_str(s1, "lastest");
    add_space(&mut s1, " version ".len() as u32);
    s1 = add_str(s1, "is\n");
    add_space(&mut s1, "Most ".len() as u32);
    s1 = add_str(s1, "downloaded");
    add_space(&mut s1, " crate ".len() as u32);
    s1 = add_str(s1, "has");
    add_space(&mut s1, " 306_437_968 ".len() as u32);
    s1 = add_str(s1, "downloads");
    add_space(&mut s1, " and ".len() as u32);
    s1 = add_str(s1, "the");
    add_space(&mut s1, " lastest ".len() as u32);
    s1 = add_str(s1, "version");
    add_space(&mut s1, " is ".len() as u32);
    add_float(&mut s1, 2.038);
    s1 = add_str(s1, ".");
    println!("{s1}");
}
