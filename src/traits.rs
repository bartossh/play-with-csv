use std::cell::RefCell;

use anyhow::Result;
use serde::Serialize;

use crate::models::Transaction;

/// CSVWrite trait provides a method to write a record to a CSV file.
pub trait CSVWrite {
    /// Writes a record to a CSV file.
    ///
    /// # Arguments
    /// * `record` - The record to write to the CSV file that is Serializable.
    ///
    /// # Returns
    /// A Result indicating success or failure.
    fn write_record<T: Serialize>(&mut self, record: &T) -> Result<()>;
}

/// CSVExport trait provides a method to export a CSV file.
pub trait CSVExport {
    /// Exports a CSV file.
    ///
    /// # Arguments
    /// * `writer` - The writer to write the CSV file to.
    ///
    /// # Returns
    /// A Result indicating success or failure.
    fn export(&self, writer: RefCell<&mut impl CSVWrite>) -> Result<()>;
}

/// Accounting trait provides a method to apply bookkeeping.
pub trait Accounting {
    /// Applies bookkeeping to a transaction.
    ///
    /// # Arguments
    /// * `transaction` - The transaction to apply bookkeeping to.
    ///
    /// # Returns
    /// A Result indicating success or failure.
    fn apply_bookkeeping(&mut self, transaction: Transaction) -> Result<()>;
}
