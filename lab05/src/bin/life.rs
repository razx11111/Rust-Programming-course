use std::process::exit;
use std::thread::sleep;
use std::time::Duration;
use std::{fs, io};

type World = Box<[[Element; 85]; 41]>;

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
        }; 85]; 41],
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
            rand::random::<usize>() % 39 + 1,
            rand::random::<usize>() % 83 + 1,
        );
        if world[x_rand][y_rand].display_char == ' ' {
            world[x_rand][y_rand].alive = alive;
            world[x_rand][y_rand].alive_neighbors = alive_neighbors as u8;
            world[x_rand][y_rand].display_char = display_char;
        } else {
            loop {
                (x_rand, y_rand) = (
                    rand::random::<usize>() % 39 + 1,
                    rand::random::<usize>() % 83 + 1,
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

fn print(world: &World) -> u32{
    let mut live_cells: u32 = 0;
    for row in world.iter() {
        for element in row.iter() {
            live_cells += if element.alive { 1 } else { 0 };
            print!("{}", element.display_char);
        }
        println!();
    }
    live_cells
}

fn the_actual_event() {
    let mut world = initialize_world();
    world = parse_into_world(&mut world).unwrap();
    let mut generation: u64 = 0;
    let mut live_cells:u32;
    
    live_cells = print(&world);
    println!("Generation: {}, Live cells: {}", generation, live_cells);
    generation = 1;

    ctrlc::set_handler(move || {
        println!("Za Warudo");
        exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    add_blinker2(3, 3, &mut world);
    add_gosper_glider_gun(10, 10, &mut world);
    add_pentadecathlon(20, 50, &mut world);
    add_lwss(30, 70, &mut world);

    loop {
        sleep(Duration::from_millis(200));
        clearscreen::clear().expect("failed to clear screen");

        for x in 1..40 {
            for y in 1..84 {
                check_neighbors(&mut world, x, y);
            }
        }

        let mut next_gen = initialize_world();

        for x in 1..40 {
            for y in 1..84 {
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
        live_cells = print(&world);
        println!("Generation: {}, Live cells: {}", generation, live_cells);
        generation += 1;
    }
}

fn add_blinker2(x: usize, y: usize, world: &mut World) {
    world[x][y].alive = true;
    world[x][y].display_char = 'x';
    world[x][y + 1].alive = true;
    world[x][y + 1].display_char = 'x';
    world[x][y - 1].alive = true;
    world[x][y - 1].display_char = 'x';
}

fn add_gosper_glider_gun(x: usize, y: usize, world: &mut World) {
    world[x + 5][y + 1].alive = true;
    world[x + 5][y + 1].display_char = 'x';
    world[x + 5][y + 2].alive = true;
    world[x + 5][y + 2].display_char = 'x';
    world[x + 6][y + 1].alive = true;
    world[x + 6][y + 1].display_char = 'x';
    world[x + 6][y + 2].alive = true;
    world[x + 6][y + 2].display_char = 'x';

    world[x + 3][y + 13].alive = true;
    world[x + 3][y + 13].display_char = 'x';
    world[x + 3][y + 14].alive = true;
    world[x + 3][y + 14].display_char = 'x';
    world[x + 4][y + 12].alive = true;
    world[x + 4][y + 12].display_char = 'x';
    world[x + 4][y + 16].alive = true;
    world[x + 4][y + 16].display_char = 'x';
    world[x + 5][y + 11].alive = true;
    world[x + 5][y + 11].display_char = 'x';
    world[x + 5][y + 17].alive = true;
    world[x + 5][y + 17].display_char = 'x';
    world[x + 6][y + 11].alive = true;
    world[x + 6][y + 11].display_char = 'x';
    world[x + 6][y + 15].alive = true;
    world[x + 6][y + 15].display_char = 'x';
    world[x + 6][y + 17].alive = true;
    world[x + 6][y + 17].display_char = 'x';
    world[x + 6][y + 18].alive = true;
    world[x + 6][y + 18].display_char = 'x';
    world[x + 7][y + 11].alive = true;
    world[x + 7][y + 11].display_char = 'x';
    world[x + 7][y + 17].alive = true;
    world[x + 7][y + 17].display_char = 'x';
    world[x + 8][y + 12].alive = true;
    world[x + 8][y + 12].display_char = 'x';
    world[x + 8][y + 16].alive = true;
    world[x + 8][y + 16].display_char = 'x';
    world[x + 9][y + 13].alive = true;
    world[x + 9][y + 13].display_char = 'x';
    world[x + 9][y + 14].alive = true;
    world[x + 9][y + 14].display_char = 'x';

    world[x + 1][y + 25].alive = true;
    world[x + 1][y + 25].display_char = 'x';
    world[x + 2][y + 23].alive = true;
    world[x + 2][y + 23].display_char = 'x';
    world[x + 2][y + 25].alive = true;
    world[x + 2][y + 25].display_char = 'x';
    world[x + 3][y + 21].alive = true;
    world[x + 3][y + 21].display_char = 'x';
    world[x + 3][y + 22].alive = true;
    world[x + 3][y + 22].display_char = 'x';
    world[x + 4][y + 21].alive = true;
    world[x + 4][y + 21].display_char = 'x';
    world[x + 4][y + 22].alive = true;
    world[x + 4][y + 22].display_char = 'x';
    world[x + 5][y + 21].alive = true;
    world[x + 5][y + 21].display_char = 'x';
    world[x + 5][y + 22].alive = true;
    world[x + 5][y + 22].display_char = 'x';
    world[x + 6][y + 23].alive = true;
    world[x + 6][y + 23].display_char = 'x';
    world[x + 6][y + 25].alive = true;
    world[x + 6][y + 25].display_char = 'x';
    world[x + 7][y + 25].alive = true;
    world[x + 7][y + 25].display_char = 'x';

    world[x + 3][y + 35].alive = true;
    world[x + 3][y + 35].display_char = 'x';
    world[x + 3][y + 36].alive = true;
    world[x + 3][y + 36].display_char = 'x';
    world[x + 4][y + 35].alive = true;
    world[x + 4][y + 35].display_char = 'x';
    world[x + 4][y + 36].alive = true;
    world[x + 4][y + 36].display_char = 'x';
}

fn add_pentadecathlon(x: usize, y: usize, world: &mut World) {
    world[x][y + 2].alive = true;
    world[x][y + 2].display_char = 'x';
    world[x + 1][y + 1].alive = true;
    world[x + 1][y + 1].display_char = 'x';
    world[x + 1][y + 3].alive = true;
    world[x + 1][y + 3].display_char = 'x';
    world[x + 2][y + 1].alive = true;
    world[x + 2][y + 1].display_char = 'x';
    world[x + 2][y + 3].alive = true;
    world[x + 2][y + 3].display_char = 'x';
    world[x + 3][y + 2].alive = true;
    world[x + 3][y + 2].display_char = 'x';
    world[x + 4][y + 2].alive = true;
    world[x + 4][y + 2].display_char = 'x';
    world[x + 5][y + 1].alive = true;
    world[x + 5][y + 1].display_char = 'x';
    world[x + 5][y + 3].alive = true;
    world[x + 5][y + 3].display_char = 'x';
    world[x + 6][y + 1].alive = true;
    world[x + 6][y + 1].display_char = 'x';
    world[x + 6][y + 3].alive = true;
    world[x + 6][y + 3].display_char = 'x';
    world[x + 7][y + 2].alive = true;
    world[x + 7][y + 2].display_char = 'x';
}

fn add_lwss(x: usize, y: usize, world: &mut World) {
    world[x][y + 1].alive = true;
    world[x][y + 1].display_char = 'x';
    world[x][y + 4].alive = true;
    world[x][y + 4].display_char = 'x';
    world[x + 1][y].alive = true;
    world[x + 1][y].display_char = 'x';
    world[x + 2][y].alive = true;
    world[x + 2][y].display_char = 'x';
    world[x + 2][y + 4].alive = true;
    world[x + 2][y + 4].display_char = 'x';
    world[x + 3][y].alive = true;
    world[x + 3][y].display_char = 'x';
    world[x + 3][y + 1].alive = true;
    world[x + 3][y + 1].display_char = 'x';
    world[x + 3][y + 2].alive = true;
    world[x + 3][y + 2].display_char = 'x';
    world[x + 3][y + 3].alive = true;
    world[x + 3][y + 3].display_char = 'x';
}

fn main() {
    the_actual_event();
}
