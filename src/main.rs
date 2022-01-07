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
}

impl MemCell {
    pub fn allocate(address: u32) -> Self {
        Self { address }
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

fn parse_constant(text: &str) -> Result<u32, std::num::ParseIntError> {
    if text.starts_with("\"") {
        let c = text.chars().nth(1).unwrap();
        Ok(c as u32)
    } else if text.starts_with("0x") {
        let without_prefix = text.trim_start_matches("0x");
        u32::from_str_radix(without_prefix, 16)
    } else {
        u32::from_str_radix(text, 10)
    }
}

fn to_bf(rule: Rule, operand: &str, state: &mut State, out: &mut Builder) {
    match rule {
        Rule::var => {
            let variable_name = operand;
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
            let variable_name = operand;
            println!("Deletion of variable '{}'", variable_name);
            if let None = state.variables.remove(&String::from(variable_name)) {
                panic!("Variable '{}' did not exists", variable_name);
            }
        }
        Rule::point => {
            let variable_name = operand;
            println!("Pointing to variable '{}'", variable_name);
            let address = state
                .variables
                .get(variable_name)
                .unwrap_or_else(|| panic!("Variable '{}' did not exists", variable_name))
                .address
                .to_string();
            to_bf(Rule::pointa, &address, state, out)
        }
        Rule::pointa => {
            let address = operand;
            let address_parsed = parse_constant(address).unwrap();
            println!("Pointing to address '{}'", address);
            if address_parsed < state.mem_pointer {
                out.append("<".repeat((state.mem_pointer - address_parsed) as usize))
            } else if address_parsed > state.mem_pointer {
                out.append(">".repeat((address_parsed - state.mem_pointer) as usize))
            }
            state.mem_pointer = address_parsed;
        }
        Rule::pointm => {
            let variable_name = operand;
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
            let constant = operand;
            let constant_parsed = parse_constant(constant).unwrap();
            println!(
                "Addition of '{}', decimal value is {}",
                constant, constant_parsed
            );
            out.append("+".repeat(constant_parsed as usize));
        }
        Rule::addv => {
            let variable_name = operand;
            println!("Addition to variable '{}'", variable_name);
            let source_address = state.mem_pointer.to_string();
            out.append("[");
            to_bf(Rule::sub, "1", state, out);
            to_bf(Rule::point, variable_name, state, out);
            to_bf(Rule::add, "1", state, out);
            to_bf(Rule::pointa, &source_address, state, out);
            out.append("]");
        }
        Rule::sub => {
            let constant = operand;
            let constant_parsed = parse_constant(constant).unwrap();
            println!(
                "Subtraction of '{}', decimal value is {}",
                constant, constant_parsed
            );
            out.append("-".repeat(constant_parsed as usize));
        }
        Rule::subv => {
            let variable_name = operand;
            println!("Subtraction from variable '{}'", variable_name);
            let source_address = state.mem_pointer.to_string();
            out.append("[");
            to_bf(Rule::sub, "1", state, out);
            to_bf(Rule::point, variable_name, state, out);
            to_bf(Rule::sub, "1", state, out);
            to_bf(Rule::pointa, &source_address, state, out);
            out.append("]");
        }
        Rule::copy => {
            let variable_name = operand;
            println!("Copy to variable '{}'", variable_name);
            let source_address = state.mem_pointer.to_string();
            to_bf(Rule::var, "__temp", state, out);
            out.append("[");
            to_bf(Rule::sub, "1", state, out);
            to_bf(Rule::point, variable_name, state, out);
            to_bf(Rule::add, "1", state, out);
            to_bf(Rule::point, "__temp", state, out);
            to_bf(Rule::add, "1", state, out);
            to_bf(Rule::pointa, &source_address, state, out);
            out.append("]");
            to_bf(Rule::point, "__temp", state, out);
            out.append("[");
            to_bf(Rule::sub, "1", state, out);
            to_bf(Rule::pointa, &source_address, state, out);
            to_bf(Rule::add, "1", state, out);
            to_bf(Rule::point, "__temp", state, out);
            out.append("]");
            to_bf(Rule::delvar, "__temp", state, out);
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
            out.append(".");
        }
        Rule::loopBlockStart => {
            println!("Start of loopBlock");
            out.append("[");
        }
        Rule::loopBlockEnd => {
            println!("End of loopBlock");
            out.append("]");
        }
        Rule::EOI => {
            println!("End of Input");
            out.append("\n");
        }
        _ => unreachable!(),
    }
}

fn instruct(statement: Pair<Rule>, state: &mut State, out: &mut Builder) {
    match statement.as_rule() {
        Rule::include => {
            let file_path_raw = extract_operand(statement);
            let file_path = &file_path_raw[1..file_path_raw.len() - 1];
            println!("Including {} into this src file", file_path);
            let content = std::fs::read_to_string(&file_path)
                .with_context(|| format!("could not read source file {:?}", file_path))
                .unwrap();
            let parsed_file = MblfParser::parse(Rule::file, &content)
                .expect("Parse Error")
                .next()
                .unwrap();
            for statement in parsed_file.into_inner() {
                instruct(statement, state, out);
            }
        }
        Rule::loopBlock => {
            for nested_statement in statement.into_inner() {
                instruct(nested_statement, state, out);
            }
        }
        _ => to_bf(statement.as_rule(), extract_operand(statement), state, out),
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
    for stmt in parsed_file.into_inner() {
        instruct(stmt, &mut state, &mut builder);
    }

    let bf = builder.string().unwrap();

    let mut out = File::create(args.output_file)?;
    out.write(bf.as_bytes())?;
    out.sync_all()?;

    Ok(())
}
