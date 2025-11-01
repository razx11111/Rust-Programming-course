use serde_derive::Deserialize;
use std::{fs, io};

#[derive(Debug, Deserialize)]
struct Student {
    name: String,
    phone_no: String,
    age: u8,
}

fn create_student() -> Student {
    Student {
        name: String::from(""),
        phone_no: String::from(""),
        age: 0,
    }
}

fn stud_parse() -> Result<(String, String), io::Error> {
    let content = std::fs::read_to_string("src/test_P1.txt")?;
    let mut students: [Student; 4] = std::array::from_fn(|_| create_student());

    for (i, line) in content.lines().enumerate() {
        let mut elements = line.split(",");
        students[i].name = elements.next().unwrap().to_string();
        students[i].phone_no = elements.next().unwrap().to_string();
        students[i].age = elements.next().unwrap().parse().unwrap();
    }

    let mut max = 0;
    let mut min = students[0].age;
    let (mut youngest, mut oldest) = ("".to_string(), "".to_string());

    for student in students {
        if student.age > max {
            max = student.age;
            oldest = student.name.clone();
        }
        if student.age < min {
            min = student.age;
            youngest = student.name.clone();
        }
    }

    Ok((youngest, oldest))
}

fn stud_json_parse() -> Result<(String, String), io::Error> {
    let content = fs::read_to_string("src/studs.json")?;
    let mut students: [Student; 4] = std::array::from_fn(|_| create_student());

    for (i, line) in content.lines().enumerate() {
        let s = line.trim();
        if s.is_empty() {
            continue;
        }
        students[i] = serde_json::from_str(s)?;
    }

    let mut max = 0;
    let mut min = students[0].age;
    let (mut youngest, mut oldest) = ("".to_string(), "".to_string());

    for student in students {
        if student.age > max {
            max = student.age;
            oldest = student.name.clone();
        }
        if student.age < min {
            min = student.age;
            youngest = student.name.clone();
        }
    }

    Ok((youngest, oldest))
}

fn main() {
    let (youngest, oldest) = stud_parse().unwrap();
    println!("The youngest student is: {}", youngest);
    println!("The oldest student is: {}", oldest);
    let (youngest, oldest) = stud_json_parse().unwrap();
    println!("The youngest student is: {}", youngest);
    println!("The oldest student is: {}", oldest);
}
