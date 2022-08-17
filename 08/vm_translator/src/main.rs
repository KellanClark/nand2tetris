#![warn(clippy::pedantic)]

extern crate core;

use std::fs;
use std::fs::File;
use std::path::PathBuf;
use std::io::Write;
use clap::Parser;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
/// Translates Jack VM language to Hack assembly.
struct Args {
    /// A .vm file or directory of .vm files to translate
    input_path: String,

    #[clap(short, long, action = clap::ArgAction::SetFalse)]
    use_bootstrap: bool,

    /// Prints parsed commands and compiled code to console
    #[clap(short, long, action = clap::ArgAction::SetTrue)]
    debug: bool,
}

fn main() {
    let args = Args::parse();

    let input_path = PathBuf::from(&args.input_path.trim_end_matches(|c| c == '/' || c == '\\'));
    let output_path = if input_path.is_dir() {
        input_path.with_file_name(format!("{}/{}.asm", input_path.display(), input_path.file_stem().unwrap().to_str().unwrap()))
    } else {
        input_path.with_extension("asm")
    };
    println!("{} -> {}", input_path.display(), output_path.display());

    let mut asm: String = "".to_string();
    if args.use_bootstrap {
        asm.push_str("@256\nD=A\n@SP\nM=D\n");
        compile_file("", &vec![parse_command("call Sys.init 0")], &mut asm);
    }

    for file in WalkDir::new(&input_path)
            .into_iter()
            .filter_map(|f| f.ok())
            .filter(|f| f.file_name().to_string_lossy().ends_with(".vm")) {
        let in_file = match fs::read_to_string(file.path()) {
            Err(why) => panic!("couldn't open {}: {}", input_path.display(), why),
            Ok(file) => file,
        };

        let commands: Vec<CommandType> = in_file
            .lines() // Split into lines
            .map(|line| line.split_once("//").unwrap_or((line, "")).0.trim()) // Remove comments and whitespace
            .filter(|line| !line.is_empty()) // Remove empty lines
            .map(parse_command) // Parse everything
            .collect();
        if args.debug {
            println!("Parsed commands for {}:\n{:#?}", file.path().display(), commands);
        }

        let file_name = file.path().file_stem().unwrap().to_str().unwrap();
        compile_file(file_name, &commands, &mut asm);
        if args.debug {
            println!("Generated code for {}:\n{}", file.path().display(), asm);
        }
    }
    let mut out_file = match File::create(&output_path) {
        Err(why) => panic!("couldn't create {}: {}", output_path.display(), why),
        Ok(file) => file,
    };
    if let Err(why) = writeln!(&mut out_file, "{}", asm) {
        panic!("couldn't write {}: {}", output_path.display(), why)
    }
}

#[derive(Clone, Copy, Debug)]
enum ArithmeticOperation {
    Add,
    Sub,
    Neg,
    Eq,
    Gt,
    Lt,
    And,
    Or,
    Not,
}

#[derive(Clone, Copy, Debug)]
enum Segment {
    Local,
    Argument,
    This,
    That,
    Constant,
    Static,
    Temp,
    Pointer,
}

#[derive(Clone, Debug)]
enum CommandType {
    Arithmetic(ArithmeticOperation),
    Pop(Segment, u16),
    Push(Segment, u16),
    Label(String),
    Goto(String),
    IfGoto(String),
    Call(String, u16),
    Function(String, u16),
    Return,
}

fn parse_command(command: &str) -> CommandType {
    let args: Vec<&str> = command.split_whitespace().collect();
    match args[0] {
        "add" => CommandType::Arithmetic(ArithmeticOperation::Add),
        "sub" => CommandType::Arithmetic(ArithmeticOperation::Sub),
        "neg" => CommandType::Arithmetic(ArithmeticOperation::Neg),
        "eq" => CommandType::Arithmetic(ArithmeticOperation::Eq),
        "gt" => CommandType::Arithmetic(ArithmeticOperation::Gt),
        "lt" => CommandType::Arithmetic(ArithmeticOperation::Lt),
        "and" => CommandType::Arithmetic(ArithmeticOperation::And),
        "or" => CommandType::Arithmetic(ArithmeticOperation::Or),
        "not" => CommandType::Arithmetic(ArithmeticOperation::Not),
        "push" => {
            assert!(args.len() == 3, "Wrong number of arguments for push: {}", command);

            let segment = get_segment(args[1]);
            let offset = match args[2].parse::<u16>() {
                Err(_) => panic!("Could not parse integer: {}", args[2]),
                Ok(result) => result
            };

            CommandType::Push(segment, offset)
        }
        "pop" => {
            assert!(args.len() == 3, "Wrong number of arguments for pop: {}", command);

            let segment = get_segment(args[1]);
            let offset = match args[2].parse::<u16>() {
                Err(_) => panic!("Could not parse integer: {}", args[2]),
                Ok(result) => result
            };

            CommandType::Pop(segment, offset)
        }
        "label" => CommandType::Label(args[1].to_string()),
        "goto" => CommandType::Goto(args[1].to_string()),
        "if-goto" => CommandType::IfGoto(args[1].to_string()),
        "call" => {
            assert!(args.len() == 3, "Wrong number of arguments for call: {}", command);

            let offset = match args[2].parse::<u16>() {
                Err(_) => panic!("Could not parse integer: {}", args[2]),
                Ok(result) => result
            };

            CommandType::Call(args[1].to_string(), offset)
        }
        "function" => {
            assert!(args.len() == 3, "Wrong number of arguments for function: {}", command);

            let offset = match args[2].parse::<u16>() {
                Err(_) => panic!("Could not parse integer: {}", args[2]),
                Ok(result) => result
            };

            CommandType::Function(args[1].to_string(), offset)
        }
        "return" => CommandType::Return,
        _ => panic!("Unknown command: {}", command)
    }
}

fn get_segment(segment: &str) -> Segment {
    match segment {
        "local" => Segment::Local,
        "argument" => Segment::Argument,
        "this" => Segment::This,
        "that" => Segment::That,
        "constant" => Segment::Constant,
        "static" => Segment::Static,
        "temp" => Segment::Temp,
        "pointer" => Segment::Pointer,
        _ => panic!("Unknown segment: {}", segment)
    }
}

fn compile_file(file_name: &str, commands: &Vec<CommandType>, output: &mut String){
    let mut command_number = 0;
    let mut current_function = "".to_string();

    for command in commands {
        output.push_str(match command {
            CommandType::Arithmetic(ArithmeticOperation::Add) => "@SP\nAM=M-1\nD=M\nA=A-1\nM=D+M\n".to_string(),
            CommandType::Arithmetic(ArithmeticOperation::Sub) => "@SP\nAM=M-1\nD=M\nA=A-1\nM=M-D\n".to_string(),
            CommandType::Arithmetic(ArithmeticOperation::Neg) => "@SP\nA=M-1\nM=-M\n".to_string(),
            CommandType::Arithmetic(ArithmeticOperation::Eq) => format!("@SP\nAM=M-1\nD=M\nA=A-1\nD=M-D\n@__COND_{0}${1}\nD;JEQ\n@SP\nA=M-1\nM=0\n@__COND_END_{0}${1}\n0;JMP\n(__COND_{0}${1})\n@SP\nA=M-1\nM=-1\n(__COND_END_{0}${1})\n", current_function, command_number),
            CommandType::Arithmetic(ArithmeticOperation::Gt) => format!("@SP\nAM=M-1\nD=M\nA=A-1\nD=M-D\n@__COND_{0}${1}\nD;JGT\n@SP\nA=M-1\nM=0\n@__COND_END_{0}${1}\n0;JMP\n(__COND_{0}${1})\n@SP\nA=M-1\nM=-1\n(__COND_END_{0}${1})\n", current_function, command_number),
            CommandType::Arithmetic(ArithmeticOperation::Lt) => format!("@SP\nAM=M-1\nD=M\nA=A-1\nD=M-D\n@__COND_{0}${1}\nD;JLT\n@SP\nA=M-1\nM=0\n@__COND_END_{0}${1}\n0;JMP\n(__COND_{0}${1})\n@SP\nA=M-1\nM=-1\n(__COND_END_{0}${1})\n", current_function, command_number),
            CommandType::Arithmetic(ArithmeticOperation::And) => "@SP\nAM=M-1\nD=M\nA=A-1\nM=D&M\n".to_string(),
            CommandType::Arithmetic(ArithmeticOperation::Or) => "@SP\nAM=M-1\nD=M\nA=A-1\nM=D|M\n".to_string(),
            CommandType::Arithmetic(ArithmeticOperation::Not) => "@SP\nA=M-1\nM=!M\n".to_string(),
            CommandType::Push(segment, offset) => {
                format!("{}\n@SP\nAM=M+1\nA=A-1\nM=D\n", match segment {
                    Segment::Local => format!("@LCL\nD=M\n@{0}\nA=D+A\nD=M", offset),
                    Segment::Argument => format!("@ARG\nD=M\n@{0}\nA=D+A\nD=M", offset),
                    Segment::This => format!("@THIS\nD=M\n@{0}\nA=D+A\nD=M", offset),
                    Segment::That => format!("@THAT\nD=M\n@{0}\nA=D+A\nD=M", offset),
                    Segment::Constant => {
                        match *offset {
                            0 => "D=0".to_string(),
                            1 => "D=1".to_string(),
                            0xFFFF => "D=-1".to_string(),
                            _ => format!("@{}\nD=A", offset)
                        }
                    }
                    Segment::Static => format!("@{}.{}\nD=M", file_name, offset),
                    Segment::Temp => {
                        assert!(*offset <= 8, "Invalid offset for temp segment: {}", offset);
                        format!("@{}\nD=M", offset + 5)
                    }
                    Segment::Pointer => {
                        match offset {
                            0 => "@THIS\nD=M".to_string(),
                            1 => "@THAT\nD=M".to_string(),
                            _ => panic!("Invalid offset for pointer segment: {}", offset)
                        }
                    }
                })
            }
            CommandType::Pop(segment, offset) => {
                format!("{}\n@R13\nM=D\n@SP\nAM=M-1\nD=M\n@R13\nA=M\nM=D\n", match segment {
                    Segment::Local => format!("@LCL\nD=M\n@{0}\nD=D+A", offset),
                    Segment::Argument => format!("@ARG\nD=M\n@{0}\nD=D+A", offset),
                    Segment::This => format!("@THIS\nD=M\n@{0}\nD=D+A", offset),
                    Segment::That => format!("@THAT\nD=M\n@{0}\nD=D+A", offset),
                    Segment::Constant => panic!("Constant segment is not writable"),
                    Segment::Static => format!("@{}.{}\nD=A", file_name, offset),
                    Segment::Temp => {
                        assert!(*offset <= 8, "Invalid offset for temp segment: {}", offset);
                        format!("@{}\nD=A", offset + 5)
                    }
                    Segment::Pointer => {
                        match offset {
                            0 => "@THIS\nD=A".to_string(),
                            1 => "@THAT\nD=A".to_string(),
                            _ => panic!("Invalid offset for pointer segment: {}", offset)
                        }
                    }
                })
            }
            CommandType::Label(label) => format!("({}${})\n", current_function, label),
            CommandType::Goto(label) => format!("@{}${}\n0;JMP\n", current_function, label),
            CommandType::IfGoto(label) => format!("@SP\nAM=M-1\nD=M\n@{}${}\nD;JNE\n", current_function, label),
            CommandType::Call(label, arguments) => format!("\
@{function}$ret.{id}\nD=A\n@SP\nAM=M\nM=D
@LCL\nD=M\n@SP\nAM=M+1\nM=D
@ARG\nD=M\n@SP\nAM=M+1\nM=D
@THIS\nD=M\n@SP\nAM=M+1\nM=D
@THAT\nD=M\n@SP\nAM=M+1\nM=D
@SP\nMD=M+1\n@LCL\nM=D\n@{arg_offset}\nD=D-A\n@ARG\nM=D
@{label}\n0;JMP
({function}$ret.{id})\n", function = current_function, id = command_number, arg_offset = 5 + arguments, label = label),
            CommandType::Function(label, variables) => {
                let mut tmp = format!("({})\n", label);

                if *variables > 0 {
                    tmp.push_str("@SP\nA=M\n");
                    for _i in 0..*variables {
                        tmp.push_str("M=0\nAD=A+1\n");
                    }
                    tmp.push_str("@SP\nM=D\n");
                }

                current_function = label.to_string();
                tmp
            }
            CommandType::Return => "\
@5\nD=A\n@LCL\nA=M-D\nD=M\n@R14\nM=D
@SP\nAM=M-1\nD=M\n@ARG\nA=M\nM=D
D=A+1\n@SP\nM=D
@LCL\nD=M\n@R13\nM=D
@R13\nAM=M-1\nD=M\n@THAT\nM=D
@R13\nAM=M-1\nD=M\n@THIS\nM=D
@R13\nAM=M-1\nD=M\n@ARG\nM=D
@R13\nAM=M-1\nD=M\n@LCL\nM=D
@R14\nA=M\n0;JMP\n".to_string(),
        }.as_str());
        command_number += 1;
    }
}