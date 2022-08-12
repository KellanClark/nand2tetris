extern crate core;

use std::env;
use std::fs;
use std::fs::File;
use std::path::PathBuf;
use std::io::Write;
use std::collections::HashMap;

#[derive(Clone, Debug)]
enum CommandValue {
    Number(u16),
    Symbol(String)
}

#[derive(Clone, Debug)]
enum CommandType {
    CommandA(CommandValue),
    CommandC{destination_a: bool, destination_m: bool, destination_d: bool, operation: u16, jump_condition: u16},
    CommandL(CommandValue)
}

const debug_info: bool = false;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        panic!("Not enough arguments");
    }

    let input_path = PathBuf::from(&args[1]);
    let output_path = PathBuf::from(input_path.with_extension("hack"));
    println!("{} -> {}", input_path.display(), output_path.display());

    let in_file = match fs::read_to_string(&input_path) {
        Err(why) => panic!("couldn't open {}: {}", input_path.display(), why),
        Ok(file) => file,
    };

    let mut commands: Vec<CommandType> = in_file
        .lines() // Split into lines
        .map(|line| line.split_once("//").unwrap_or((line, "")).0.trim()) // Remove comments and whitespace
        .filter(|line| !line.is_empty()) // Remove empty lines
        .map(|line| parse_command(line)) // Parse everything
        .collect();
    if debug_info {
        println!("Parsed commands:\n{:#?}\n", commands);
    }

    replace_symbols(&mut commands);
    if debug_info {
        println!("With symbols replaced:\n{:#?}\n", commands);
    }

    commands = commands.into_iter().filter(|command| match command {
        CommandType::CommandL(_) => false,
        _ => true
    }).collect();
    if debug_info {
        println!("With labels removed:\n{:#?}\n", commands);
    }

    let mut out_file = match File::create(&output_path) {
        Err(why) => panic!("couldn't create {}: {}", output_path.display(), why),
        Ok(file) => file,
    };

    for command in &commands {
        writeln!(&mut out_file, "{:0>16b}", compile_command(command));
    }
}

fn parse_command(command: &str) -> CommandType {
    match command.chars().next().unwrap() {
        '@' => {
            CommandType::CommandA(match command[1..].parse::<u16>() {
                Err(_) => CommandValue::Symbol(command[1..].parse().unwrap()),
                Ok(result) => CommandValue::Number(result)
            })
        }
        '(' => {
            if !command.ends_with(")") {
                panic!("Invalid command: {}", command);
            }

            CommandType::CommandL(CommandValue::Symbol(command[1..command.len() - 1].parse().unwrap()))
        }
        _ => {
            let destination_str = command.split_once('=').unwrap_or(("", "")).0.to_uppercase();
            let mut operation_str = command.split_once('=').unwrap_or(("", command)).1.to_string();
            operation_str = operation_str.split_once(';').unwrap_or((&operation_str, "")).0.to_uppercase();
            let jump_str = command.split_once(';').unwrap_or(("", "")).1.to_uppercase();

            let destination_a = destination_str.contains('A');
            let destination_m = destination_str.contains('M');
            let destination_d = destination_str.contains('D');

            let operation: u16 = match operation_str.as_str() {
                "0" => 0b0_101010,
                "1" => 0b0_111111,
                "-1" => 0b0_111010,
                "D" => 0b0_001100,
                "A" => 0b0_110000,
                "M" => 0b1_110000,
                "!D" => 0b0_001101,
                "!A" => 0b0_110001,
                "!M" => 0b1_110001,
                "-D" => 0b0_001111,
                "-A" => 0b0_110001,
                "-M" => 0b1_110001,
                "D+1" => 0b0_011111,
                "A+1" => 0b0_110111,
                "M+1" => 0b1_110111,
                "D-1" => 0b0_001110,
                "A-1" => 0b0_110010,
                "M-1" => 0b1_110010,
                "D+A" => 0b0_000010,
                "D+M" => 0b1_000010,
                "D-A" => 0b0_010011,
                "D-M" => 0b1_010011,
                "A-D" => 0b0_000111,
                "M-D" => 0b1_000111,
                "D&A" => 0b0_000000,
                "D&M" => 0b1_000000,
                "D|A" => 0b0_010101,
                "D|M" => 0b1_010101,
                _ => panic!("Unknown operation {} in command {}", operation_str, command)
            };

            let jump_condition: u16 = match jump_str.as_str() {
                "" => 0b000,
                "JGT" => 0b001,
                "JEQ" => 0b010,
                "JGE" => 0b011,
                "JLT" => 0b100,
                "JNE" => 0b101,
                "JLE" => 0b110,
                "JMP" => 0b111,
                _ => panic!("Unknown jump condition {} in command {}", jump_str, command)
            };

            CommandType::CommandC {destination_a, destination_m, destination_d, operation, jump_condition}
        }
    }
}

fn replace_symbols(commands: &mut Vec<CommandType>) {
    let mut symbols_table: HashMap<String, u16> = HashMap::from([
        ("SP".to_string(), 0),
        ("LCL".to_string(), 1),
        ("ARG".to_string(), 2),
        ("THIS".to_string(), 3),
        ("THAT".to_string(), 4),
        ("R0".to_string(), 0), ("R1".to_string(), 1), ("R2".to_string(), 2), ("R3".to_string(), 3), ("R4".to_string(), 4), ("R5".to_string(), 5), ("R6".to_string(), 6), ("R7".to_string(), 7), ("R8".to_string(), 8), ("R9".to_string(), 9), ("R10".to_string(), 10), ("R11".to_string(), 11), ("R12".to_string(), 12), ("R13".to_string(), 13), ("R14".to_string(), 14), ("R15".to_string(), 15),
        ("SCREEN".to_string(), 0x4000),
        ("KBD".to_string(), 0x6000),
    ]);

    let mut rom_location: u16 = 0;
    for command in &mut *commands {
        match command {
            CommandType::CommandL(CommandValue::Symbol(symbol)) => {
                if symbols_table.contains_key(&*symbol) {
                    panic!("Label already exists: {}", symbol);
                } else {
                    symbols_table.insert(symbol.to_string(), rom_location);
                }

                *command = CommandType::CommandL(CommandValue::Number(*symbols_table.get(&*symbol).unwrap()));
            }
            _ => rom_location += 1
        };
    }
    if rom_location > 0x8000 {
        panic!("Too much code")
    }
    let mut variable_location: u16 = 16;
    for command in &mut *commands {
        match command {
            CommandType::CommandA(CommandValue::Symbol(symbol)) => {
                if !symbols_table.contains_key(&*symbol) {
                    symbols_table.insert(symbol.to_string(), variable_location);

                    variable_location += 1;
                }

                *command = CommandType::CommandA(CommandValue::Number(*symbols_table.get(&*symbol).unwrap()));
            }
            _ => ()
        };
    }
    if variable_location > 0x8000 {
        panic!("Too many variables")
    }
}

fn compile_command(command: &CommandType) -> u16 {
    match *command {
        CommandType::CommandA(CommandValue::Number(num)) => num,
        CommandType::CommandC {destination_a, destination_m, destination_d, operation, jump_condition} => {
            0xE000 | (operation << 6) | ((destination_a as u16) << 5) | ((destination_d as u16) << 4)| ((destination_m as u16) << 3) | jump_condition
        }
        CommandType::CommandA(CommandValue::Symbol(ref sym)) => panic!("Unparsed symbol {} in command\n{:#?}", sym, command),
        _ => panic!("Unknown command:\n{:#?}", command)
    }
}
