use std::collections::HashMap;

use crate::domain_types::{
    AccountStatus, Balance, ClientId, DomainError, PaymentCommand, Transaction, TransactionId,
    TransactionKind,
};

#[cfg(test)]
use crate::domain_types::Money;

#[derive(Debug, PartialEq, Eq)]
pub struct Account {
    client_id: ClientId,
    balance: Balance,
    status: AccountStatus,
    transactions: HashMap<TransactionId, Transaction>,
}

impl Account {
    #[must_use]
    pub fn new(client_id: ClientId) -> Self {
        Self {
            client_id,
            balance: Balance::ZERO,
            status: AccountStatus::Active,
            transactions: HashMap::new(),
        }
    }

    #[must_use]
    pub fn client_id(&self) -> ClientId {
        self.client_id
    }

    #[must_use]
    pub fn balance(&self) -> &Balance {
        &self.balance
    }

    #[must_use]
    pub fn status(&self) -> AccountStatus {
        self.status
    }

    /// Applies a deposit to this account. Other commands are currently ignored.
    ///
    /// # Errors
    ///
    /// Returns an error if a deposit targets a different client, reuses a
    /// transaction ID for this account, or puts the available or total balance
    /// outside the supported money range.
    pub fn apply(&mut self, command: PaymentCommand) -> Result<(), DomainError> {
        if let PaymentCommand::Deposit {
            client_id,
            transaction_id,
            amount,
        } = command
        {
            if client_id != self.client_id {
                return Err(DomainError::ClientMismatch);
            }
            if self.transactions.contains_key(&transaction_id) {
                return Err(DomainError::DuplicateTransaction);
            }

            let available = self.balance.available().checked_add(amount.money())?;

            self.balance = Balance::new(available, self.balance.held())?;
            self.transactions.insert(
                transaction_id,
                Transaction::new(transaction_id, TransactionKind::Deposit, amount),
            );
        }
        // Other payment commands will be implemented in subsequent changes.
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn with_test_state(
        mut self,
        available: Money,
        held: Money,
        status: AccountStatus,
        transactions: HashMap<TransactionId, Transaction>,
    ) -> Self {
        self.balance = Balance::new(available, held).expect("valid test balance");
        self.status = status;
        self.transactions = transactions;
        self
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rust_decimal::Decimal;

    use super::Account;
    use crate::domain_types::{
        AccountStatus, Balance, ClientId, DomainError, Money, PaymentCommand, Transaction,
        TransactionAmount, TransactionId, TransactionKind, TransactionState,
    };

    #[test]
    fn new_account_starts_active_with_a_zero_balance() {
        let account = Account::new(ClientId(42));

        assert_eq!(
            account,
            Account {
                client_id: ClientId(42),
                balance: Balance::ZERO,
                status: AccountStatus::Active,
                transactions: HashMap::new(),
            }
        );
    }

    mod apply {
        use rstest::rstest;

        use super::*;

        fn deposit(transaction_id: u32, amount: Decimal) -> PaymentCommand {
            PaymentCommand::Deposit {
                client_id: ClientId(42),
                transaction_id: TransactionId(transaction_id),
                amount: TransactionAmount::new(amount).expect("valid deposit amount"),
            }
        }

        fn deposit_transaction(transaction_id: u32, amount: Decimal) -> Transaction {
            Transaction {
                id: TransactionId(transaction_id),
                kind: TransactionKind::Deposit,
                amount: Money::new(amount).expect("valid deposit amount"),
                state: TransactionState::Normal,
            }
        }

        #[rstest]
        #[case::new_account(
            Decimal::ZERO,
            Decimal::ZERO,
            AccountStatus::Active,
            &[(u32::MAX, Decimal::new(12_345, 4))],
            Decimal::new(12_345, 4),
        )]
        #[case::unordered_ids_and_exact_accumulation(
            Decimal::ZERO,
            Decimal::ZERO,
            AccountStatus::Active,
            &[
                (9, Decimal::new(1, 1)),
                (0, Decimal::new(2, 1)),
                (3, Decimal::new(1, 4)),
            ],
            Decimal::new(3_001, 4),
        )]
        #[case::negative_available_with_held_funds(
            Decimal::new(-12_345, 4),
            Decimal::new(2, 0),
            AccountStatus::Active,
            &[(1, Decimal::ONE)],
            Decimal::new(-2_345, 4),
        )]
        #[case::locked_account_with_held_funds(
            Decimal::ONE,
            Decimal::new(2, 0),
            AccountStatus::Locked,
            &[(1, Decimal::ONE)],
            Decimal::new(2, 0),
        )]
        #[case::money_limit(
            Decimal::ZERO,
            Decimal::ZERO,
            AccountStatus::Active,
            &[(1, Money::MAX.value())],
            Money::MAX.value(),
        )]
        fn deposits_credit_available_and_record_normal_transactions(
            #[case] available: Decimal,
            #[case] held: Decimal,
            #[case] status: AccountStatus,
            #[case] deposits: &[(u32, Decimal)],
            #[case] expected_available: Decimal,
        ) {
            let mut account = Account::new(ClientId(42)).with_test_state(
                Money::new(available).expect("valid available"),
                Money::new(held).expect("valid held"),
                status,
                HashMap::new(),
            );

            for &(id, amount) in deposits {
                account
                    .apply(deposit(id, amount))
                    .expect("deposit should succeed");
            }

            assert_eq!(
                account,
                Account {
                    client_id: ClientId(42),
                    balance: Balance::new(
                        Money::new(expected_available).expect("valid available"),
                        Money::new(held).expect("valid held"),
                    )
                    .expect("valid balance"),
                    status,
                    transactions: deposits
                        .iter()
                        .map(|&(id, amount)| (TransactionId(id), deposit_transaction(id, amount)))
                        .collect(),
                }
            );
        }

        #[test]
        fn deposit_for_another_client_leaves_the_account_unchanged() {
            let mut account = Account::new(ClientId(7));

            assert_eq!(
                account.apply(deposit(1, Decimal::ONE)),
                Err(DomainError::ClientMismatch)
            );

            assert_eq!(
                account,
                Account {
                    client_id: ClientId(7),
                    balance: Balance::ZERO,
                    status: AccountStatus::Active,
                    transactions: HashMap::new(),
                }
            );
        }

        #[test]
        fn duplicate_deposit_does_not_credit_again_or_overwrite_history() {
            let mut account = Account::new(ClientId(42));
            account
                .apply(deposit(1, Decimal::ONE))
                .expect("first deposit should succeed");

            assert_eq!(
                account.apply(deposit(1, Decimal::new(2, 0))),
                Err(DomainError::DuplicateTransaction)
            );

            assert_eq!(
                account,
                Account {
                    client_id: ClientId(42),
                    balance: Balance::new(
                        Money::new(Decimal::ONE).expect("valid available"),
                        Money::new(Decimal::ZERO).expect("valid held"),
                    )
                    .expect("valid balance"),
                    status: AccountStatus::Active,
                    transactions: HashMap::from([(
                        TransactionId(1),
                        deposit_transaction(1, Decimal::ONE),
                    )]),
                }
            );
        }

        #[rstest]
        #[case::unit_above_available_limit(Money::MAX.value(), Decimal::ZERO, Decimal::ONE)]
        #[case::fraction_above_available_limit(Money::MAX.value(), Decimal::ZERO, Decimal::new(1, 4))]
        #[case::maximum_deposit_exceeds_available_limit(Decimal::new(1, 4), Decimal::ZERO, Money::MAX.value())]
        #[case::held_at_limit(Decimal::ZERO, Money::MAX.value(), Decimal::new(1, 4))]
        #[case::combined_total_at_limit(Money::MAX.value() - Decimal::new(1, 4), Decimal::new(1, 4), Decimal::new(1, 4))]
        fn deposits_exceeding_the_money_limit_leave_balances_and_history_unchanged(
            #[case] available: Decimal,
            #[case] held: Decimal,
            #[case] amount: Decimal,
        ) {
            let mut account = Account::new(ClientId(42)).with_test_state(
                Money::new(available).expect("valid available"),
                Money::new(held).expect("valid held"),
                AccountStatus::Active,
                HashMap::new(),
            );

            assert_eq!(
                account.apply(deposit(1, amount)),
                Err(DomainError::AmountOutOfRange)
            );

            assert_eq!(
                account,
                Account {
                    client_id: ClientId(42),
                    balance: Balance::new(
                        Money::new(available).expect("valid available"),
                        Money::new(held).expect("valid held"),
                    )
                    .expect("valid balance"),
                    status: AccountStatus::Active,
                    transactions: HashMap::new(),
                }
            );
        }
    }
}
