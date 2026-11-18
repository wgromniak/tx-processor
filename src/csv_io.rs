use std::io::Read;

use rust_decimal::Decimal;
use serde::Deserialize;

use crate::domain_types::{
    Account, ClientId, DomainError, PaymentCommand, TransactionAmount, TransactionId,
};

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
        let client_id = ClientId(self.client);
        let transaction_id = TransactionId(self.tx);

        match (self.r#type.as_str(), self.amount.as_deref()) {
            ("deposit", Some(amount)) => Ok(PaymentCommand::Deposit {
                client_id,
                transaction_id,
                amount: parse_amount(amount)?,
            }),
            ("withdrawal", Some(amount)) => Ok(PaymentCommand::Withdrawal {
                client_id,
                transaction_id,
                amount: parse_amount(amount)?,
            }),
            ("dispute", None) => Ok(PaymentCommand::Dispute {
                client_id,
                transaction_id,
            }),
            ("resolve", None) => Ok(PaymentCommand::Resolve {
                client_id,
                transaction_id,
            }),
            ("chargeback", None) => Ok(PaymentCommand::Chargeback {
                client_id,
                transaction_id,
            }),
            ("deposit" | "withdrawal", None) => Err(CsvError::MissingAmount),
            ("dispute" | "resolve" | "chargeback", Some(_)) => Err(CsvError::UnexpectedAmount),
            _ => Err(CsvError::UnknownTransactionType(self.r#type)),
        }
    }
}

fn parse_amount(amount: &str) -> Result<TransactionAmount, CsvError> {
    let value = Decimal::from_str_exact(amount).map_err(CsvError::InvalidDecimal)?;
    TransactionAmount::new(value).map_err(CsvError::InvalidAmount)
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

#[derive(Debug, PartialEq)]
pub enum CsvError {
    UnknownTransactionType(String),
    MissingAmount,
    UnexpectedAmount,
    InvalidDecimal(rust_decimal::Error),
    InvalidAmount(DomainError),
}
