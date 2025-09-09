use anyhow::{Result, anyhow};
use csv::ReaderBuilder;
use std::env;
use std::fs::File;
use std::io::{self};

mod writer;
mod errors;
mod processor;
mod models;
mod ledger;
mod traits;

const DEFAULT_HAS_HEADERS: bool = true;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let input: Box<dyn io::Read> = match args.len() {
        0 | 1 => Box::new(io::stdin()),
        2 => Box::new(File::open(&args[1])?),
        _ => Err(anyhow!("not implemented"))?,
    };

    let reader = ReaderBuilder::new()
        .has_headers(DEFAULT_HAS_HEADERS)
        .from_reader(input);
    let writer = writer::StdOutCSVWriter::new();
    let accountant = ledger::Accountant::new();

    let mut engine = processor::Engine::new(writer, reader, accountant);

    engine.run()?;

    Ok(())
}
