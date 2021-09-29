use std::fs::File;
use std::io::Write;

use anyhow::{Context, Result};
use structopt::StructOpt;

#[derive(StructOpt)]
struct Cli {
    #[structopt(parse(from_os_str))]
    input_file:  std::path::PathBuf,
    #[structopt(parse(from_os_str))]
    output_file: std::path::PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::from_args();

    let content = std::fs::read_to_string(&args.input_file)
        .with_context(|| format!("could not read source file {:?}", args.input_file))?;

    let mut out = File::create(args.output_file)?;
    out.write_all(&content.as_bytes())?;

    out.sync_all()?;

    Ok(())
}
