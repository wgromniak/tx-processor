use std::io::Read;

use serde::Deserialize;

use crate::domain_types::{Account, PaymentCommand};

pub fn read_transactions(
    reader: impl Read,
) -> Result<impl Iterator<Item = Result<TransactionRow, csv::Error>>, csv::Error> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(reader);
    reader.headers()?;

    Ok(reader.into_deserialize())
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct TransactionRow {
    pub r#type: String,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<String>,
}

impl TransactionRow {
    pub fn into_command(self) -> Result<PaymentCommand, CsvError> {
        unimplemented!()
    }
}

#[derive(Debug)]
pub struct AccountRow {
    pub client: u16,
    pub available: String,
    pub held: String,
    pub total: String,
    pub locked: bool,
}

impl AccountRow {
    pub fn from_account(_account: &Account) -> Self {
        unimplemented!()
    }
}

#[derive(Debug)]
pub struct CsvError;
