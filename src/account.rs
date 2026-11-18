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
    pub fn new(client_id: ClientId) -> Self {
        Self {
            client_id,
            balance: Balance::ZERO,
            status: AccountStatus::Active,
            transactions: HashMap::new(),
        }
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

    pub fn transaction(&self, transaction_id: TransactionId) -> Option<&Transaction> {
        self.transactions.get(&transaction_id)
    }

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

        #[test]
        fn deposit_credits_available_and_records_a_normal_transaction() {
            let mut account = Account::new(ClientId(42));
            let amount = Decimal::new(12_345, 4);

            account
                .apply(deposit(u32::MAX, amount))
                .expect("deposit should succeed");

            assert_eq!(
                account,
                Account {
                    client_id: ClientId(42),
                    balance: Balance::new(
                        Money::new(amount).expect("valid available"),
                        Money::new(Decimal::ZERO).expect("valid held"),
                    )
                    .expect("valid balance"),
                    status: AccountStatus::Active,
                    transactions: HashMap::from([(
                        TransactionId(u32::MAX),
                        deposit_transaction(u32::MAX, amount),
                    )]),
                }
            );
        }

        #[test]
        fn deposits_accumulate_exactly_with_unordered_transaction_ids() {
            let mut account = Account::new(ClientId(42));
            let deposits = [
                (9, Decimal::new(1, 1)),
                (0, Decimal::new(2, 1)),
                (3, Decimal::new(1, 4)),
            ];

            for (id, amount) in deposits {
                account
                    .apply(deposit(id, amount))
                    .expect("deposit should succeed");
            }

            assert_eq!(
                account,
                Account {
                    client_id: ClientId(42),
                    balance: Balance::new(
                        Money::new(Decimal::new(3_001, 4)).expect("valid available"),
                        Money::new(Decimal::ZERO).expect("valid held"),
                    )
                    .expect("valid balance"),
                    status: AccountStatus::Active,
                    transactions: HashMap::from([
                        (TransactionId(9), deposit_transaction(9, Decimal::new(1, 1))),
                        (TransactionId(0), deposit_transaction(0, Decimal::new(2, 1))),
                        (TransactionId(3), deposit_transaction(3, Decimal::new(1, 4))),
                    ]),
                }
            );
        }

        #[test]
        fn deposit_preserves_held_funds_and_can_credit_negative_available_funds() {
            let mut account = Account::new(ClientId(42)).with_test_state(
                Money::new(Decimal::new(-12_345, 4)).expect("valid available"),
                Money::new(Decimal::new(2, 0)).expect("valid held"),
                AccountStatus::Active,
                HashMap::new(),
            );

            account
                .apply(deposit(1, Decimal::ONE))
                .expect("deposit should succeed");

            assert_eq!(
                account,
                Account {
                    client_id: ClientId(42),
                    balance: Balance::new(
                        Money::new(Decimal::new(-2_345, 4)).expect("valid available"),
                        Money::new(Decimal::new(2, 0)).expect("valid held"),
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

        #[test]
        fn deposit_into_a_locked_account_is_credited_without_unlocking_it() {
            let mut account = Account::new(ClientId(42)).with_test_state(
                Money::new(Decimal::ONE).expect("valid available"),
                Money::new(Decimal::new(2, 0)).expect("valid held"),
                AccountStatus::Locked,
                HashMap::new(),
            );
            account
                .apply(deposit(1, Decimal::ONE))
                .expect("deposit into a locked account should succeed");

            assert_eq!(
                account,
                Account {
                    client_id: ClientId(42),
                    balance: Balance::new(
                        Money::new(Decimal::new(2, 0)).expect("valid available"),
                        Money::new(Decimal::new(2, 0)).expect("valid held"),
                    )
                    .expect("valid balance"),
                    status: AccountStatus::Locked,
                    transactions: HashMap::from([(
                        TransactionId(1),
                        deposit_transaction(1, Decimal::ONE),
                    )]),
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

        #[test]
        fn deposit_accepts_the_money_limit() {
            let mut account = Account::new(ClientId(42));

            account
                .apply(deposit(1, Money::MAX.value()))
                .expect("maximum balance is representable");

            assert_eq!(
                account,
                Account {
                    client_id: ClientId(42),
                    balance: Balance::new(
                        Money::new(Money::MAX.value()).expect("valid available"),
                        Money::new(Decimal::ZERO).expect("valid held"),
                    )
                    .expect("valid balance"),
                    status: AccountStatus::Active,
                    transactions: HashMap::from([(
                        TransactionId(1),
                        deposit_transaction(1, Money::MAX.value()),
                    )]),
                }
            );
        }

        #[test]
        fn deposits_exceeding_the_money_limit_leave_balances_and_history_unchanged() {
            let smallest = Decimal::new(1, 4);
            for (available, held, amount) in [
                (Money::MAX.value(), Decimal::ZERO, Decimal::ONE),
                (Money::MAX.value(), Decimal::ZERO, smallest),
                (smallest, Decimal::ZERO, Money::MAX.value()),
                (Decimal::ZERO, Money::MAX.value(), smallest),
                (Money::MAX.value() - smallest, smallest, smallest),
            ] {
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
}
