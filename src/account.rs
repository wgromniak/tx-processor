use std::collections::HashMap;

use crate::domain_types::{
    AccountStatus, Balance, ClientId, DomainError, PaymentCommand, Transaction, TransactionAmount,
    TransactionId, TransactionKind,
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

    pub fn apply(&mut self, command: PaymentCommand) -> Result<(), DomainError> {
        if command.client_id() != self.client_id {
            return Err(DomainError::ClientMismatch);
        }

        match command {
            PaymentCommand::Deposit {
                transaction_id,
                amount,
                ..
            } => self.deposit(transaction_id, amount),
            PaymentCommand::Withdrawal {
                transaction_id,
                amount,
                ..
            } => self.withdraw(transaction_id, amount),
            PaymentCommand::Dispute { .. }
            | PaymentCommand::Resolve { .. }
            | PaymentCommand::Chargeback { .. } => Ok(()),
        }
    }

    fn deposit(
        &mut self,
        transaction_id: TransactionId,
        amount: TransactionAmount,
    ) -> Result<(), DomainError> {
        if self.transactions.contains_key(&transaction_id) {
            return Err(DomainError::DuplicateTransaction);
        }

        let available = self.balance.available().checked_add(amount.money())?;

        self.balance = Balance::new(available, self.balance.held())?;
        self.transactions.insert(
            transaction_id,
            Transaction::new(transaction_id, TransactionKind::Deposit, amount),
        );
        Ok(())
    }

    fn withdraw(
        &mut self,
        transaction_id: TransactionId,
        amount: TransactionAmount,
    ) -> Result<(), DomainError> {
        if self.transactions.contains_key(&transaction_id) {
            return Err(DomainError::DuplicateTransaction);
        }
        if self.status == AccountStatus::Locked
            || self.balance.available().value() < amount.money().value()
        {
            return Ok(());
        }

        let available = self.balance.available().checked_sub(amount.money())?;

        self.balance = Balance::new(available, self.balance.held())?;
        self.transactions.insert(
            transaction_id,
            Transaction::new(transaction_id, TransactionKind::Withdrawal, amount),
        );
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

        fn withdrawal(transaction_id: u32, amount: Decimal) -> PaymentCommand {
            PaymentCommand::Withdrawal {
                client_id: ClientId(42),
                transaction_id: TransactionId(transaction_id),
                amount: TransactionAmount::new(amount).expect("valid withdrawal amount"),
            }
        }

        fn assert_balance(account: &Account, available: Decimal, held: Decimal) {
            assert_eq!(
                account.balance,
                Balance::new(
                    Money::new(available).expect("valid available"),
                    Money::new(held).expect("valid held"),
                )
                .expect("valid balance"),
            );
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

        #[rstest]
        #[case::deposit(deposit(1, Decimal::ONE))]
        #[case::withdrawal(withdrawal(1, Decimal::ONE))]
        fn command_for_another_client_leaves_the_account_unchanged(
            #[case] command: PaymentCommand,
        ) {
            let mut account = Account::new(ClientId(7));

            assert_eq!(account.apply(command), Err(DomainError::ClientMismatch));

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

        #[rstest]
        #[case::fractional(Decimal::new(12_345, 4), Decimal::new(1, 4))]
        #[case::exact_available(Decimal::new(3, 1), Decimal::new(3, 1))]
        fn withdrawals_debit_only_available_funds_and_record_history(
            #[case] available: Decimal,
            #[case] amount: Decimal,
        ) {
            let mut account = Account::new(ClientId(42)).with_test_state(
                Money::new(available).expect("valid available"),
                Money::new(Decimal::ONE).expect("valid held"),
                AccountStatus::Active,
                HashMap::new(),
            );

            account
                .apply(withdrawal(0, amount))
                .expect("withdrawal succeeds");

            assert_balance(&account, available - amount, Decimal::ONE);
            assert_eq!(account.status, AccountStatus::Active);
            assert_eq!(
                account.transactions,
                HashMap::from([(
                    TransactionId(0),
                    Transaction::new(
                        TransactionId(0),
                        TransactionKind::Withdrawal,
                        TransactionAmount::new(amount).expect("valid amount"),
                    ),
                )])
            );
        }

        #[rstest]
        #[case::empty(Decimal::ZERO, Decimal::ZERO, AccountStatus::Active)]
        #[case::insufficient(Decimal::new(9_999, 4), Decimal::ZERO, AccountStatus::Active)]
        #[case::held_funds(Decimal::ZERO, Decimal::ONE, AccountStatus::Active)]
        #[case::negative_available(-Decimal::ONE, Decimal::ONE, AccountStatus::Active)]
        #[case::locked(Decimal::ONE, Decimal::ZERO, AccountStatus::Locked)]
        fn rejected_withdrawals_leave_balances_and_history_unchanged(
            #[case] available: Decimal,
            #[case] held: Decimal,
            #[case] status: AccountStatus,
        ) {
            let initial = || {
                Account::new(ClientId(42)).with_test_state(
                    Money::new(available).expect("valid available"),
                    Money::new(held).expect("valid held"),
                    status,
                    HashMap::new(),
                )
            };
            let mut account = initial();

            assert_eq!(account.apply(withdrawal(1, Decimal::ONE)), Ok(()));
            assert_eq!(account, initial());
        }

        #[rstest]
        #[case::deposit_then_withdrawal(deposit(1, Decimal::ONE), withdrawal(1, Decimal::ONE))]
        #[case::withdrawal_then_deposit(withdrawal(1, Decimal::ONE), deposit(1, Decimal::ONE))]
        #[case::duplicate_withdrawal(withdrawal(1, Decimal::ONE), withdrawal(1, Decimal::ONE))]
        fn payment_ids_cannot_be_reused(
            #[case] first: PaymentCommand,
            #[case] duplicate: PaymentCommand,
        ) {
            let initial = || {
                let mut account = Account::new(ClientId(42));
                account
                    .apply(deposit(9, Decimal::new(2, 0)))
                    .expect("funding succeeds");
                account.apply(first).expect("first payment succeeds");
                account
            };
            let mut account = initial();

            assert_eq!(
                account.apply(duplicate),
                Err(DomainError::DuplicateTransaction)
            );
            assert_eq!(account, initial());
        }
    }
}
