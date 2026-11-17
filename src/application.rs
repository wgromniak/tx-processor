//! Use-case orchestration using domain commands and the concrete repository.

use crate::{
    domain::{Account, DomainError, PaymentCommand},
    infrastructure::AccountRepository,
};

#[derive(Debug)]
pub struct PaymentProcessor {
    accounts: AccountRepository,
}

impl PaymentProcessor {
    #[must_use]
    pub fn new(_accounts: AccountRepository) -> Self {
        unimplemented!()
    }

    /// # Errors
    /// Returns a domain error if processing the command fails.
    pub fn process(&mut self, _command: PaymentCommand) -> Result<(), DomainError> {
        unimplemented!()
    }

    #[must_use]
    pub fn accounts(&self) -> Vec<&Account> {
        unimplemented!()
    }
}
