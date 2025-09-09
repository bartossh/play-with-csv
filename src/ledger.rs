use std::{cell::RefCell, io::Read};

use crate::{
    models::Transaction,
    traits::{Accounting, CSVExport, CSVWrite},
};
use anyhow::Result;
use csv::Reader;

pub struct Engine<T, S> {
    writer: T,
    reader: Reader<Box<dyn Read>>,
    accountant: S,
}

impl<T, I> Engine<T, I>
where
    T: CSVWrite + Sync + Send,
    I: CSVExport + Accounting,
{
    pub fn new(writer: T, reader: Reader<Box<dyn Read>>, accountant: I) -> Self {
        Self {
            writer,
            reader,
            accountant,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        for rec in self.reader.deserialize::<Transaction>() {
            let tx: Transaction = rec?;
            self.accountant.apply_bookkeeping(tx)?;
        }

        let writer = RefCell::new(&mut self.writer);

        self.accountant.export(writer)?;

        Ok(())
    }
}
