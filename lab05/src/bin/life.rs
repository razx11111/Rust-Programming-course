use std::process::exit;
use std::thread::sleep;
use std::time::Duration;
use std::{fs, io};

type World = Box<[[Element; 101]; 51]>;

#[derive(Clone, Copy)]
struct Element {
    alive: bool,
    alive_neighbors: u8,
    display_char: char,
}

fn check_neighbors(world: &mut World, x: usize, y: usize) {
    world[x][y].alive_neighbors = 0;
    if world[x][y + 1].alive {
        world[x][y].alive_neighbors += 1;
    }
    if world[x][y - 1].alive {
        world[x][y].alive_neighbors += 1;
    }
    if world[x + 1][y].alive {
        world[x][y].alive_neighbors += 1;
    }
    if world[x - 1][y].alive {
        world[x][y].alive_neighbors += 1;
    }
    if world[x + 1][y + 1].alive {
        world[x][y].alive_neighbors += 1;
    }
    if world[x - 1][y - 1].alive {
        world[x][y].alive_neighbors += 1;
    }
    if world[x + 1][y - 1].alive {
        world[x][y].alive_neighbors += 1;
    }
    if world[x - 1][y + 1].alive {
        world[x][y].alive_neighbors += 1;
    }
}

fn initialize_world() -> World {
    Box::new(
        [[Element {
            alive: false,
            alive_neighbors: 0,
            display_char: ' ',
        }; 101]; 51],
    )
}

fn parse_into_world(world: &mut World) -> Result<World, io::Error> {
    let content = fs::read_to_string("src/bin/alive_list.txt")?;
    for line in content.lines() {
        let mut elements = line.split(", ");
        let alive: bool = elements.next().unwrap().parse().unwrap();
        let alive_neighbors: usize = elements.next().unwrap().parse().unwrap();
        let display_char: char = elements.next().unwrap().chars().next().unwrap();
        let (mut x_rand, mut y_rand) = (
            rand::random::<usize>() % 49 + 1,
            rand::random::<usize>() % 99 + 1,
        );
        if world[x_rand][y_rand].display_char == ' ' {
            world[x_rand][y_rand].alive = alive;
            world[x_rand][y_rand].alive_neighbors = alive_neighbors as u8;
            world[x_rand][y_rand].display_char = display_char;
        } else {
            loop {
                (x_rand, y_rand) = (
                    rand::random::<usize>() % 49 + 1,
                    rand::random::<usize>() % 99 + 1,
                );
                if world[x_rand][y_rand].display_char == ' ' {
                    world[x_rand][y_rand].alive = alive;
                    world[x_rand][y_rand].alive_neighbors = alive_neighbors as u8;
                    world[x_rand][y_rand].display_char = display_char;
                    break;
                }
            }
        }
    }
    Ok(world.clone())
}

fn print(world: &World) {
    for row in world.iter() {
        for element in row.iter() {
            print!("{}", element.display_char);
        }
        println!();
    }
}

fn the_actual_event() {
    let mut world = initialize_world();
    world = parse_into_world(&mut world).unwrap();
    print(&world);

    // ctrlc::set_handler(move || {
    //     println!("Thannos snapped :(");
    //     print(&world);
    //     exit(0);
    // })
    // .expect("Error setting Ctrl-C handler");
    world[33][33].alive = true;
    world[33][33].display_char = 'x';
    world[33][34].alive = true;
    world[33][34].display_char = 'x';
    world[33][32].alive = true;
    world[33][32].display_char = 'x';

    loop {
        sleep(Duration::from_millis(300));
        clearscreen::clear().expect("failed to clear screen");

        for x in 1..50 {
            for y in 1..100 {
                check_neighbors(&mut world, x, y);
            }
        }

        let mut next_gen = initialize_world();

        for x in 1..50 {
            for y in 1..100 {
                let neighbors = world[x][y].alive_neighbors;

                if world[x][y].alive {
                    if neighbors == 2 || neighbors == 3 {
                        next_gen[x][y].alive = true;
                        next_gen[x][y].display_char = 'x';
                    }
                } else if neighbors == 3 {
                    next_gen[x][y].alive = true;
                    next_gen[x][y].display_char = 'x';
                }
            }
        }

        world = next_gen;
        print(&world);
    }
}

fn main() {
    the_actual_event();
}
