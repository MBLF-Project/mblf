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

fn instruct(statement: Pair<Rule>, out: &mut Builder) {
    match statement.as_rule() {
        Rule::var => {
            let variable_name = extract_operand(statement);
            println!("Creation of variable '{}'", variable_name);
            out.append("var\n");
        }
        Rule::delvar => {
            let variable_name = extract_operand(statement);
            println!("Deletion of variable '{}'", variable_name);
            out.append("delvar\n");
        }
        Rule::point => {
            let variable_name = extract_operand(statement);
            println!("Pointing to variable '{}'", variable_name);
            out.append("point\n");
        }
        Rule::pointm => {
            let variable_name = extract_operand(statement);
            println!("Pointing back to marker variable {}", variable_name);
            out.append("pointm\n");
        }
        Rule::addv => {
            let variable_name = extract_operand(statement);
            println!("Addition to variable '{}'", variable_name);
            out.append("addv\n");
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
        Rule::instruction => {
            out.append("\n");
        }
        Rule::operator => {
            out.append("operator ");
        }
        Rule::operand => {
            out.append("operand ");
        }

        Rule::loopBlock => {
            for nested_statement in statement.into_inner() {
                instruct(nested_statement, out);
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

    for statement in parsed_file.into_inner() {
        instruct(statement, &mut builder);
    }

    let bf = builder.string().unwrap();

    let mut out = File::create(args.output_file)?;
    out.write(bf.as_bytes())?;
    out.sync_all()?;

    Ok(())
}
