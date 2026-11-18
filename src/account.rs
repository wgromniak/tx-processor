use std::collections::HashMap;

use crate::domain_types::{
    AccountStatus, Balance, ClientId, DomainError, Money, PaymentCommand, Transaction,
    TransactionId,
};

#[derive(Debug)]
pub struct Account {
    client_id: ClientId,
    balance: Balance,
    status: AccountStatus,
    transactions: HashMap<TransactionId, Transaction>,
}

impl Account {
    pub fn new(client_id: ClientId) -> Self {
        Self {
            client_id,
            balance: Balance::ZERO,
            status: AccountStatus::Active,
            transactions: HashMap::new(),
        }
    }

    pub(crate) fn with_available_balance(mut self, available: Money) -> Self {
        self.balance = Balance::new(available, self.balance.held());
        self
    }

    pub fn client_id(&self) -> ClientId {
        self.client_id
    }

    pub fn balance(&self) -> &Balance {
        &self.balance
    }

    pub fn status(&self) -> AccountStatus {
        self.status
    }

    pub fn transaction(&self, _transaction_id: TransactionId) -> Option<&Transaction> {
        unimplemented!()
    }

    pub fn apply(&mut self, _command: PaymentCommand) -> Result<(), DomainError> {
        // Account changes will be implemented with the payment rules.
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn with_test_state(
        mut self,
        available: Money,
        held: Money,
        status: AccountStatus,
    ) -> Self {
        self.balance = Balance::new(available, held);
        self.status = status;
        self
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    use super::Account;
    use crate::domain_types::{AccountStatus, ClientId};

    #[test]
    fn new_account_starts_active_with_a_zero_balance() {
        let account = Account::new(ClientId(42));

        assert_eq!(account.client_id(), ClientId(42));
        assert_eq!(account.status(), AccountStatus::Active);
        assert_eq!(account.balance().available().value(), Decimal::ZERO);
        assert_eq!(account.balance().held().value(), Decimal::ZERO);
        assert_eq!(account.balance().total().value(), Decimal::ZERO);
    }
}
