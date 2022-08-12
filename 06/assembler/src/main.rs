#![warn(clippy::pedantic)]

extern crate core;

use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

#[derive(Clone, Debug)]
enum CommandValue {
    Number(u16),
    Symbol(String),
}

#[derive(Clone, Debug)]
enum CommandType {
    CommandA(CommandValue),
    CommandC {
        destination_a: bool,
        destination_m: bool,
        destination_d: bool,
        operation: u16,
        jump_condition: u16,
    },
    CommandL(CommandValue),
}

const DEBUG_INFO: bool = false;

fn main() {
    let args: Vec<String> = env::args().collect();
    assert!(args.len() >= 2, "Not enough arguments");

    let input_path = PathBuf::from(&args[1]);
    let output_path = input_path.with_extension("hack");
    println!("{} -> {}", input_path.display(), output_path.display());

    let in_file = match fs::read_to_string(&input_path) {
        Err(why) => panic!("couldn't open {}: {}", input_path.display(), why),
        Ok(file) => file,
    };

    let mut commands: Vec<CommandType> = in_file
        .lines() // Split into lines
        .map(|line| line.split_once("//").unwrap_or((line, "")).0.trim()) // Remove comments and whitespace
        .filter(|line| !line.is_empty()) // Remove empty lines
        .map(parse_command) // Parse everything
        .collect();
    if DEBUG_INFO {
        println!("Parsed commands:\n{:#?}\n", commands);
    }

    replace_symbols(&mut commands);
    if DEBUG_INFO {
        println!("With symbols replaced:\n{:#?}\n", commands);
    }

    commands = commands
        .into_iter()
        .filter(|command| !matches!(command, CommandType::CommandL(_)))
        .collect();
    if DEBUG_INFO {
        println!("With labels removed:\n{:#?}\n", commands);
    }

    let mut out_file = match File::create(&output_path) {
        Err(why) => panic!("couldn't create {}: {}", output_path.display(), why),
        Ok(file) => file,
    };

    for command in &commands {
        if let Err(why) = writeln!(&mut out_file, "{:0>16b}", compile_command(command)) {
            panic!("couldn't write {}: {}", output_path.display(), why)
        }
    }
}

fn parse_command(command: &str) -> CommandType {
    match command.chars().next().unwrap() {
        '@' => CommandType::CommandA(match command[1..].parse::<u16>() {
            Err(_) => CommandValue::Symbol(command[1..].parse().unwrap()),
            Ok(result) => CommandValue::Number(result),
        }),
        '(' => {
            assert!(command.ends_with(')'), "Invalid command: {}", command);

            CommandType::CommandL(CommandValue::Symbol(
                command[1..command.len() - 1].parse().unwrap(),
            ))
        }
        _ => {
            let destination_str = command.split_once('=').unwrap_or(("", "")).0.to_uppercase();
            let mut operation_str = command
                .split_once('=')
                .unwrap_or(("", command))
                .1
                .to_string();
            operation_str = operation_str
                .split_once(';')
                .unwrap_or((&operation_str, ""))
                .0
                .to_uppercase();
            let jump_str = command.split_once(';').unwrap_or(("", "")).1.to_uppercase();

            let destination_a = destination_str.contains('A');
            let destination_m = destination_str.contains('M');
            let destination_d = destination_str.contains('D');

            let operation: u16 = match operation_str.as_str() {
                "0" => 0b010_1010,
                "1" => 0b011_1111,
                "-1" => 0b011_1010,
                "D" => 0b000_1100,
                "A" => 0b011_0000,
                "M" => 0b111_0000,
                "!D" => 0b000_1101,
                "!A" => 0b011_0001,
                "!M" => 0b111_0001,
                "-D" => 0b000_1111,
                "-A" => 0b011_0011,
                "-M" => 0b111_0011,
                "D+1" => 0b001_1111,
                "A+1" => 0b011_0111,
                "M+1" => 0b111_0111,
                "D-1" => 0b000_1110,
                "A-1" => 0b011_0010,
                "M-1" => 0b111_0010,
                "D+A" => 0b000_0010,
                "D+M" => 0b100_0010,
                "D-A" => 0b001_0011,
                "D-M" => 0b101_0011,
                "A-D" => 0b000_0111,
                "M-D" => 0b100_0111,
                "D&A" => 0b000_0000,
                "D&M" => 0b100_0000,
                "D|A" => 0b001_0101,
                "D|M" => 0b101_0101,
                _ => panic!("Unknown operation {} in command {}", operation_str, command),
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
                _ => panic!("Unknown jump condition {} in command {}", jump_str, command),
            };

            CommandType::CommandC {
                destination_a,
                destination_m,
                destination_d,
                operation,
                jump_condition,
            }
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
        ("R0".to_string(), 0),
        ("R1".to_string(), 1),
        ("R2".to_string(), 2),
        ("R3".to_string(), 3),
        ("R4".to_string(), 4),
        ("R5".to_string(), 5),
        ("R6".to_string(), 6),
        ("R7".to_string(), 7),
        ("R8".to_string(), 8),
        ("R9".to_string(), 9),
        ("R10".to_string(), 10),
        ("R11".to_string(), 11),
        ("R12".to_string(), 12),
        ("R13".to_string(), 13),
        ("R14".to_string(), 14),
        ("R15".to_string(), 15),
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
                    symbols_table.insert((*symbol).to_string(), rom_location);
                }

                *command = CommandType::CommandL(CommandValue::Number(
                    *symbols_table.get(&*symbol).unwrap(),
                ));
            }
            _ => rom_location += 1,
        };
    }
    assert!(rom_location <= 0x8000, "Too much code");

    let mut variable_location: u16 = 16;
    for command in &mut *commands {
        if let CommandType::CommandA(CommandValue::Symbol(symbol)) = command {
            if !symbols_table.contains_key(&*symbol) {
                symbols_table.insert((*symbol).to_string(), variable_location);

                variable_location += 1;
            }

            *command =
                CommandType::CommandA(CommandValue::Number(*symbols_table.get(&*symbol).unwrap()));
        }
    }
    assert!(variable_location <= 0x8000, "Too many variables");
}

fn compile_command(command: &CommandType) -> u16 {
    match *command {
        CommandType::CommandA(CommandValue::Number(num)) => num,
        CommandType::CommandC {
            destination_a,
            destination_m,
            destination_d,
            operation,
            jump_condition,
        } => {
            0xE000
                | (operation << 6)
                | (u16::from(destination_a) << 5)
                | (u16::from(destination_d) << 4)
                | (u16::from(destination_m) << 3)
                | jump_condition
        }
        CommandType::CommandA(CommandValue::Symbol(ref sym)) => {
            panic!("Unparsed symbol {} in command\n{:#?}", sym, command)
        }
        _ => panic!("Unknown command:\n{:#?}", command),
    }
}
