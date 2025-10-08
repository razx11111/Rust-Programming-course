fn add_chars_n(s: &mut String, caracter: char, num: u8) {
    for _i in 0..num {
        s.push(caracter);
    }
}

fn add_space(s: &mut String, n:u32) {
    for _i in 0..n {
        s.push(' ');
    }
}

fn add_str(mut s:String, str: String) -> String {
    s += &str;
    s
}

fn int_len(mut n:u32) -> u8 {
    let mut len:u8 = 0;
    while n != 0 {
        n /= 10;
        len += 1;
    }
    len
}

fn add_integer(s: &mut String, mut n: i32) {
    if n < 0 {
        s.push('-');
        n *= -1;
    }
    
    let mut exp:i32 = 10;
    exp = exp.pow((int_len(n as u32) - 1) as u32);
    let mut count:u8 = 0;
    
    while exp != 0 {
        if count % 3 == 0 && count != 0 {
            s.push('_');
        }
        s.push((((n / exp) % 10) as u8 + '0' as u8) as char); //idk daca trb atatea paranteze but why not
        exp /= 10;
        count += 1;
    }
}

fn pnct_index(mut n: f32) -> u8 {
    let mut index:u8 = 0;
    while n > 0.1 {
        n /= 10.0;
        index += 1;
    }
    index - 1
}

fn add_float( s: &mut String, n: f32) {
    if n < 0 {
        s.push('-');
        n *= -1;
    }
    

}

fn main() {
    let mut s = String::from("");
    let mut i = 0;
    while i < 26 {
        let c = (i as u8 + 'a' as u8) as char;
        add_chars_n(&mut s, c, 26 - i);

        i += 1;
    }
    add_integer(&mut s, -323443);
    print!("{}", s);
}
