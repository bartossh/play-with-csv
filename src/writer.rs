use crate::traits::CSVWrite;
use anyhow::Result;
use csv::{Writer, WriterBuilder};
use serde::Serialize;
use std::io::{self, Stdout};

pub struct StdOutCSVWriter {
    stdout_writer: Writer<Stdout>,
}

impl StdOutCSVWriter {
    pub fn new() -> Self {
        StdOutCSVWriter {
            stdout_writer: WriterBuilder::new().from_writer(io::stdout()),
        }
    }
}

impl CSVWrite for StdOutCSVWriter {
    fn write_record<T: Serialize>(&mut self, record: &T) -> Result<()> {
        Ok(self.stdout_writer.serialize(record)?)
    }
}
