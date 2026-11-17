use crate::domain_types::{Account, PaymentCommand};

#[derive(Debug)]
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
