use std::collections::HashMap;

use crate::domain_types::{Account, ClientId, DomainError, PaymentCommand};

#[derive(Debug)]
pub struct PaymentProcessor {
    accounts: HashMap<ClientId, Account>,
}

#[allow(clippy::new_without_default)]
impl PaymentProcessor {
    pub fn new() -> Self {
        unimplemented!()
    }

    pub fn process(&mut self, _command: PaymentCommand) -> Result<(), DomainError> {
        unimplemented!()
    }

    pub fn accounts(&self) -> Vec<&Account> {
        unimplemented!()
    }
}
