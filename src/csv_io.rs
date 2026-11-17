use std::io::Read;

use serde::Deserialize;

use crate::domain_types::{Account, PaymentCommand};

pub fn read_transactions(
    reader: impl Read,
) -> impl Iterator<Item = Result<TransactionRow, csv::Error>> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(reader);
    let header_error = reader.headers().err();
    let rows = header_error.is_none().then(|| reader.into_deserialize());

    header_error
        .into_iter()
        .map(Err)
        .chain(rows.into_iter().flatten())
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
