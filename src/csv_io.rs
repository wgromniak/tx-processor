//! CSV DTOs, mappings, and streaming IO, separate from domain types.

use std::io::{Read, Write};

use crate::domain::{Account, PaymentCommand};

/// An input row before domain conversion; amount text preserves decimal precision.
#[derive(Debug)]
pub struct TransactionRow {
    pub r#type: String,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<String>,
}

impl TransactionRow {
    /// # Errors
    /// Returns a CSV error if the row cannot be converted into a domain command.
    pub fn into_command(self) -> Result<PaymentCommand, CsvError> {
        unimplemented!()
    }
}

/// An output row whose amounts are formatted for CSV.
#[derive(Debug)]
pub struct AccountRow {
    pub client: u16,
    pub available: String,
    pub held: String,
    pub total: String,
    pub locked: bool,
}

impl AccountRow {
    #[must_use]
    pub fn from_account(_account: &Account) -> Self {
        unimplemented!()
    }
}

/// Reads individual rows from a file or an in-memory input.
#[derive(Debug)]
pub struct CsvReader<R: Read> {
    reader: R,
}

impl<R: Read> CsvReader<R> {
    #[must_use]
    pub fn new(_reader: R) -> Self {
        unimplemented!()
    }

    /// # Errors
    /// Returns a CSV error if reading or parsing the next row fails.
    pub fn read_next(&mut self) -> Result<Option<TransactionRow>, CsvError> {
        unimplemented!()
    }
}

/// Writes rows to stdout, a file, or an in-memory output.
#[derive(Debug)]
pub struct CsvWriter<W: Write> {
    writer: W,
}

impl<W: Write> CsvWriter<W> {
    #[must_use]
    pub fn new(_writer: W) -> Self {
        unimplemented!()
    }

    /// # Errors
    /// Returns a CSV error if writing the row fails.
    pub fn write_row(&mut self, _row: &AccountRow) -> Result<(), CsvError> {
        unimplemented!()
    }

    /// # Errors
    /// Returns a CSV error if flushing the output fails.
    pub fn flush(&mut self) -> Result<(), CsvError> {
        unimplemented!()
    }
}

/// Placeholder for CSV parsing, domain conversion, and IO errors.
#[derive(Debug)]
pub struct CsvError;
