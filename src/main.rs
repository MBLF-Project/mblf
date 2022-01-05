use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

use anyhow::{Context, Result};
use string_builder::Builder;
use structopt::StructOpt;

extern crate pest;
#[macro_use]
extern crate pest_derive;

use pest::iterators::Pair;
use pest::Parser;

#[derive(Parser)]
#[grammar = "grammars/mblf.pest"]
struct MblfParser;

struct MemCell {
    address: u32,
    value: u8,
}

impl MemCell {
    pub fn allocate(address: u32) -> Self {
        Self { address, value: 0 }
    }
}

struct State {
    alloc_cnt: u32,
    mem_pointer: u32,
    variables: HashMap<String, MemCell>,
}

#[derive(StructOpt)]
struct Cli {
    #[structopt(parse(from_os_str))]
    input_file: std::path::PathBuf,
    #[structopt(parse(from_os_str))]
    output_file: std::path::PathBuf,
}

fn extract_operand(statement: Pair<Rule>) -> &str {
    let mut line = statement.as_str();

    while let Some(c) = line.chars().next() {
        line = &line[c.len_utf8()..];
        if c == ' ' {
            break;
        }
    }
    line
}

fn parse_constant(text: &str) -> Result<i32, std::num::ParseIntError> {
    if text.starts_with("\"") {
        let c = text.chars().nth(1).unwrap();
        Ok(c as i32)
    } else if text.starts_with("0x") {
        let without_prefix = text.trim_start_matches("0x");
        i32::from_str_radix(without_prefix, 16)
    } else {
        i32::from_str_radix(text, 10)
    }
}

fn instruct(statement: Pair<Rule>, state: &mut State, out: &mut Builder) {
    match statement.as_rule() {
        Rule::include => {
            let file_path = extract_operand(statement);
            println!("Including {} into this src file", file_path);
            out.append("#include\n");
        }
        Rule::var => {
            let variable_name = extract_operand(statement);
            println!("Creation of variable '{}'", variable_name);
            if let Some(_v) = state.variables.insert(
                String::from(variable_name),
                MemCell::allocate(state.alloc_cnt),
            ) {
                panic!("Variable {} already exists", variable_name);
            }
            state.alloc_cnt += 1;
        }
        Rule::delvar => {
            let variable_name = extract_operand(statement);
            println!("Deletion of variable '{}'", variable_name);
            if let None = state.variables.remove(&String::from(variable_name)) {
                panic!("Variable '{}' did not exists", variable_name);
            }
        }
        Rule::point => {
            let variable_name = extract_operand(statement);
            println!("Pointing to variable '{}'", variable_name);
            let address = state
                .variables
                .get(variable_name)
                .unwrap_or_else(|| panic!("Variable '{}' did not exists", variable_name))
                .address;
            if address < state.mem_pointer {
                out.append("<".repeat((state.mem_pointer - address) as usize))
            } else if address > state.mem_pointer {
                out.append(">".repeat((address - state.mem_pointer) as usize))
            }
            state.mem_pointer = address;
        }
        Rule::pointm => {
            let variable_name = extract_operand(statement);
            println!("Pointing back to marker variable {}", variable_name);
            let address = state
                .variables
                .get(variable_name)
                .unwrap_or_else(|| panic!("Marker variable '{}' did not exists", variable_name))
                .address;
            // thank you mixtela
            out.append("<+[-<+]-");
            state.mem_pointer = address;
        }
        Rule::add => {
            let constant = extract_operand(statement);
            let constant_parsed = parse_constant(constant).unwrap();
            println!(
                "Addition of '{}', decimal value is {}",
                constant, constant_parsed
            );
            out.append("+".repeat(constant_parsed as usize));
        }
        Rule::addb => {
            let constant = extract_operand(statement);
            let constant_parsed = parse_constant(constant).unwrap();
            println!(
                "Big Addition of '{}', decimal value is {}",
                constant, constant_parsed
            );
            out.append("addb\n");
        }
        Rule::addv => {
            let variable_name = extract_operand(statement);
            println!("Addition to variable '{}'", variable_name);
            out.append("addv\n");
        }
        Rule::sub => {
            let constant = extract_operand(statement);
            let constant_parsed = parse_constant(constant).unwrap();
            println!(
                "Subtraction of '{}', decimal value is {}",
                constant, constant_parsed
            );
            out.append("-".repeat(constant_parsed as usize));
        }
        Rule::subb => {
            let constant = extract_operand(statement);
            let constant_parsed = parse_constant(constant).unwrap();
            println!(
                "Big Subtraction of '{}', decimal value is {}",
                constant, constant_parsed
            );
            out.append("subb\n");
        }
        Rule::subv => {
            let variable_name = extract_operand(statement);
            println!("Subtraction from variable '{}'", variable_name);
            out.append("subv\n");
        }
        Rule::copy => {
            let variable_name = extract_operand(statement);
            println!("Copy to variable '{}'", variable_name);
            out.append("copy\n");
        }
        Rule::setz => {
            println!("Set current variable to zero");
            out.append("[-]");
        }
        Rule::getchr => {
            println!("Reading char from user input into current variable");
            out.append(",");
        }
        Rule::print => {
            println!("Printing current variable");
            out.append("print\n");
        }
        Rule::instruction => {
            out.append("\n");
        }
        Rule::loopBlock => {
            for nested_statement in statement.into_inner() {
                instruct(nested_statement, state, out);
            }
        }
        Rule::loopBlockStart => {
            println!("Start of loopBlock");
            out.append("loopBlockStart\n");
        }
        Rule::loopBlockEnd => {
            println!("End of loopBlock");
            out.append("loopBlockEnd\n");
        }
        Rule::EOI => {
            println!("End of Input");
            out.append("\n");
        }
        _ => unreachable!(),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::from_args();

    let content = std::fs::read_to_string(&args.input_file)
        .with_context(|| format!("could not read source file {:?}", args.input_file))?;

    let mut builder = Builder::default();

    let parsed_file = MblfParser::parse(Rule::file, &content)
        .expect("Parse Error")
        .next()
        .unwrap();

    let mut state = State {
        alloc_cnt: 0,
        mem_pointer: 0,
        variables: HashMap::new(),
    };
    for statement in parsed_file.into_inner() {
        instruct(statement, &mut state, &mut builder);
    }

    let bf = builder.string().unwrap();

    let mut out = File::create(args.output_file)?;
    out.write(bf.as_bytes())?;
    out.sync_all()?;

    Ok(())
}
