use std::fs::File;
use std::io::Write;

use anyhow::{Context, Result};
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
    input_file:  std::path::PathBuf,
    #[structopt(parse(from_os_str))]
    output_file: std::path::PathBuf,
}

fn instruct(statement: Pair<Rule>, mut out: &File) {
    match statement.as_rule() {
        Rule::instruction => {
            for nested_statement in statement.into_inner() {
                instruct(nested_statement, out);
            }
            out.write(b"\n");
        }
        Rule::operator => {
            out.write(b"operator ");
        }
        Rule::operand => {
            out.write(b"operand ");
        }

        Rule::loopBlock => {
            for nested_statement in statement.into_inner() {
                instruct(nested_statement, out);
            }
        }
        Rule::loopBlockStart => {
            out.write(b"loopBlockStart\n");
        }
        Rule::loopBlockEnd => {
            out.write(b"loopBlockEnd\n");
        }
        Rule::EOI => {
            out.write(b"\n");
        }
        _ => unreachable!(),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::from_args();

    let content = std::fs::read_to_string(&args.input_file)
        .with_context(|| format!("could not read source file {:?}", args.input_file))?;

    let out = File::create(args.output_file)?;

    let parsed_file = MblfParser::parse(Rule::file, &content)
        .expect("Parse Error")
        .next()
        .unwrap();

    for statement in parsed_file.into_inner() {
        instruct(statement, &out);
    }

    out.sync_all()?;

    Ok(())
}
