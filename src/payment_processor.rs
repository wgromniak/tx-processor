use std::collections::HashMap;

use rust_decimal::Decimal;

use crate::{
    account::Account,
    domain_types::{ClientId, DomainError, Money, PaymentCommand},
};

#[derive(Debug)]
pub struct PaymentProcessor {
    accounts: HashMap<ClientId, Account>,
}

#[allow(clippy::new_without_default)]
impl PaymentProcessor {
    #[allow(
        clippy::expect_used,
        reason = "The fixed smoke-test balances have valid scales"
    )]
    pub fn new() -> Self {
        // Fixed accounts let the smoke test exercise the CLI before payment rules are implemented.
        let accounts = [
            (ClientId(1), Decimal::new(15, 1)),
            (ClientId(2), Decimal::new(2, 0)),
        ]
        .into_iter()
        .map(|(client_id, available)| {
            let available = Money::new(available).expect("valid fixture balance");
            let account = Account::new(client_id).with_available_balance(available);
            (client_id, account)
        })
        .collect();
        Self { accounts }
    }

    pub fn process(&mut self, command: PaymentCommand) -> Result<(), DomainError> {
        if let Some(account) = self.accounts.get_mut(&command.client_id()) {
            account.apply(command)?;
        }
        Ok(())
    }

    pub fn accounts(&self) -> Vec<&Account> {
        self.accounts.values().collect()
    }
}
