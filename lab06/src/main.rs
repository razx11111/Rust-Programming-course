use std::{fs, process::exit};

struct PingCommand { }

struct CountCommand { }

struct TimesCommand {
    count: usize
}

struct CatCommand { }

struct Terminal {
    registered_commands: Vec<Box<dyn Command>>,
}

impl Terminal {
    fn new() -> Terminal {
        return Terminal {
            registered_commands: Vec::new()
        }
    }

    fn register(&mut self, command:Box<dyn Command>) {
        self.registered_commands.push(command);
    }

    fn is_registered(&mut self, first_arg: &String, args: &[String]) -> bool {
        for i in 0..self.registered_commands.len() {
            if self.registered_commands[i].get_name().to_string().to_ascii_lowercase() == *first_arg {
                self.registered_commands[i].exec(args);
                return true
            }
        }
        println!("Invalid Command");
        false
    }

    fn run(&mut self) {
        let content = fs::read_to_string("src/command_list.txt").unwrap();
        for lines in content.lines() {
            let parts: Vec<String> = lines.split_whitespace().map(|s| s.to_string()).collect();
            if parts.is_empty() {
                println!("empty line");
                continue;
            }
            let cmd_name = &parts[0];
            let args = &parts[1..];
            
            if cmd_name.to_ascii_lowercase() == "stop" {
                println!("Process stopped");
                exit(0);
            }

            self.is_registered(cmd_name, args);
        }
    }

}

trait Command {
    fn get_name(&self) -> &'static str;
    fn exec(&mut self, str: &[String]);
}

impl Command for PingCommand {
    fn get_name(&self) -> &'static str {
        "ping"
    }
    fn exec(&mut self, _str: &[String]) {
        println!("pong!");
    }
}

impl Command for CountCommand {
    fn get_name(&self) -> &'static str {
        "count"
    }
    fn exec(&mut self, str: &[String]) {
        let mut i:u8 = 0;
        for _s in str.iter() {
            i += 1;
        }
        println!("Recieved {i} arguments");
    }
}

impl Command for TimesCommand {
    fn get_name(&self) -> &'static str {
        "times"
    }
    fn exec(&mut self, _str: &[String]) {
        self.count += 1; 
        println!("Been called {} times", self.count);
    }
}

impl Command for CatCommand {
    fn get_name(&self) -> &'static str {
        "cat"
    }
    fn exec(&mut self, str: &[String]) {
        // let path:String = String::from("./");
        // path.push_str(new)
    }
}

fn main() {
    let mut terminal = Terminal::new();

    terminal.register(Box::new(PingCommand {}));
    terminal.register(Box::new(CountCommand {}));
    terminal.register(Box::new(TimesCommand { count: 0 }));

    terminal.run();
}