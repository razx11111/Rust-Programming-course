use sqrt_rs::babylonian_sqrt;

fn prime(n: u32) -> bool {
    if n == 1 {
        return false;
    }
    if n == 2 {
        return true;
    }
    if n == 0 {
        return false;
    }
    let mut i: u32 = 2;
    loop {
        if n % i == 0 {
            return false;
        }
        i = i + 1;
        if i as f32 > babylonian_sqrt(n as f32) { //am aflat ulterior ca sqrt e facuta metoda in rust ceea ce face mult sens but this is more fun asa ca las asa :))
            break;
        }
    }
    true
}

fn coprime(mut a: u32, mut b: u32) -> bool {
    if a == 0 || b == 0 {
        return false;
    }
    while a != b {
        if a > b {
            a = a - b;
        } else if a < b {
            b = b - a;
        }
    }
    if a == 1 {
        return true; //(a,b) = 1
    }
    false
}

fn bottles_of_beer() {
    let mut beers: u32 = 99;
    loop {
        println!("{beers} bottles of beer on the wall, \n{beers} bottles of beer.");
        beers -= 1;
        println!("Take one down pass it around,");
        if beers != 0 {
            println!("{beers} bottles of beer on the wall.\n");
        } else {
            println!("No more bottles of beer on the wall.");
            break;
        }
    }
}

fn main() {
    print!("Lista numere prime: ");
    for i in 0..100 {
        if prime(i) {
            print!("{i}, ");
        }
    }
    println!("\nLista numere coprime: ");

    for i in 0..100 {
        for j in 0..100 {
            if coprime(i, j) {
                print!("{i} si {j}, ");
            }
        }
    }

    println!();

    bottles_of_beer();
}