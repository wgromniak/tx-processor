use crate::{
    domain::{Account, DomainError, PaymentCommand},
    infrastructure::AccountRepository,
};

#[derive(Debug)]
pub struct PaymentProcessor {
    accounts: AccountRepository,
}

impl PaymentProcessor {
    pub fn new(_accounts: AccountRepository) -> Self {
        unimplemented!()
    }

    pub fn process(&mut self, _command: PaymentCommand) -> Result<(), DomainError> {
        unimplemented!()
    }

    pub fn accounts(&self) -> Vec<&Account> {
        unimplemented!()
    }
}
