use std::collections::HashMap;

use crate::domain::{
    transaction::{Transaction, TransactionAmount, TransactionId, TransactionKind},
    types::{AccountStatus, Balance, ClientId, DomainError, PaymentCommand},
};

#[cfg(test)]
use crate::domain::types::Money;

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
            PaymentCommand::Dispute { transaction_id, .. } => self.dispute(transaction_id),
            PaymentCommand::Resolve { transaction_id, .. } => self.resolve(transaction_id),
            PaymentCommand::Chargeback { transaction_id, .. } => self.chargeback(transaction_id),
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

    fn dispute(&mut self, transaction_id: TransactionId) -> Result<(), DomainError> {
        let Some(transaction) = self.transactions.get_mut(&transaction_id) else {
            return Ok(());
        };
        if !transaction.can_be_disputed() {
            return Ok(());
        }

        let available = self
            .balance
            .available()
            .checked_sub(transaction.amount().money())?;
        let held = self
            .balance
            .held()
            .checked_add(transaction.amount().money())?;
        let balance = Balance::new(available, held)?;

        transaction.dispute()?;
        self.balance = balance;
        Ok(())
    }

    fn resolve(&mut self, transaction_id: TransactionId) -> Result<(), DomainError> {
        let Some(transaction) = self.transactions.get_mut(&transaction_id) else {
            return Ok(());
        };
        if !transaction.is_disputed() {
            return Ok(());
        }

        let available = self
            .balance
            .available()
            .checked_add(transaction.amount().money())?;
        let held = self
            .balance
            .held()
            .checked_sub(transaction.amount().money())?;
        let balance = Balance::new(available, held)?;

        transaction.resolve()?;
        self.balance = balance;
        Ok(())
    }

    fn chargeback(&mut self, transaction_id: TransactionId) -> Result<(), DomainError> {
        let Some(transaction) = self.transactions.get_mut(&transaction_id) else {
            return Ok(());
        };
        if !transaction.is_disputed() {
            return Ok(());
        }

        let held = self
            .balance
            .held()
            .checked_sub(transaction.amount().money())?;
        let balance = Balance::new(self.balance.available(), held)?;

        transaction.chargeback()?;
        self.balance = balance;
        self.status = AccountStatus::Locked;
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

    use rstest::rstest;
    use rust_decimal::Decimal;

    use super::Account;
    use crate::domain::{
        transaction::{
            Transaction, TransactionAmount, TransactionId, TransactionKind, TransactionState,
        },
        types::{AccountStatus, Balance, ClientId, DomainError, Money, PaymentCommand},
    };

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

    fn dispute(transaction_id: u32) -> PaymentCommand {
        PaymentCommand::Dispute {
            client_id: ClientId(42),
            transaction_id: TransactionId(transaction_id),
        }
    }

    fn resolve(transaction_id: u32) -> PaymentCommand {
        PaymentCommand::Resolve {
            client_id: ClientId(42),
            transaction_id: TransactionId(transaction_id),
        }
    }

    fn chargeback(transaction_id: u32) -> PaymentCommand {
        PaymentCommand::Chargeback {
            client_id: ClientId(42),
            transaction_id: TransactionId(transaction_id),
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
        Transaction::new(
            TransactionId(transaction_id),
            TransactionKind::Deposit,
            TransactionAmount::new(amount).expect("valid deposit amount"),
        )
    }

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

    #[rstest]
    #[case::deposit(deposit(1, Decimal::ONE))]
    #[case::withdrawal(withdrawal(1, Decimal::ONE))]
    #[case::dispute(dispute(1))]
    #[case::resolve(resolve(1))]
    #[case::chargeback(chargeback(1))]
    fn command_for_another_client_leaves_the_account_unchanged(#[case] command: PaymentCommand) {
        let mut account = Account::new(ClientId(7));

        assert_eq!(account.apply(command), Err(DomainError::ClientMismatch));
        assert_eq!(account, Account::new(ClientId(7)));
    }

    mod deposits {
        use super::*;

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

    mod withdrawals {
        use super::*;

        fn funded_account_after_payment(command: PaymentCommand) -> Account {
            let mut account = Account::new(ClientId(42));
            account
                .apply(deposit(9, Decimal::new(2, 0)))
                .expect("funding succeeds");
            account.apply(command).expect("first payment succeeds");
            account
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
            let mut account = Account::new(ClientId(42)).with_test_state(
                Money::new(available).expect("valid available"),
                Money::new(held).expect("valid held"),
                status,
                HashMap::new(),
            );
            let balance = account.balance;

            assert_eq!(account.apply(withdrawal(1, Decimal::ONE)), Ok(()));
            assert_eq!(
                account,
                Account {
                    client_id: ClientId(42),
                    balance,
                    status,
                    transactions: HashMap::new(),
                }
            );
        }

        #[rstest]
        #[case::deposit_then_withdrawal(deposit(1, Decimal::ONE), withdrawal(1, Decimal::ONE))]
        #[case::withdrawal_then_deposit(withdrawal(1, Decimal::ONE), deposit(1, Decimal::ONE))]
        #[case::duplicate_withdrawal(withdrawal(1, Decimal::ONE), withdrawal(1, Decimal::ONE))]
        fn payment_ids_cannot_be_reused(
            #[case] first: PaymentCommand,
            #[case] duplicate: PaymentCommand,
        ) {
            let mut account = funded_account_after_payment(first);

            assert_eq!(
                account.apply(duplicate),
                Err(DomainError::DuplicateTransaction)
            );
            assert_eq!(account, funded_account_after_payment(first));
        }
    }

    mod disputes {
        use super::*;
        use crate::domain::transaction::TransactionState::{ChargedBack, Disputed, Normal};
        use crate::domain::types::AccountStatus::{Active, Locked};

        fn test_balance(available: Decimal, held: Decimal) -> Balance {
            Balance::new(
                Money::new(available).expect("valid available"),
                Money::new(held).expect("valid held"),
            )
            .expect("valid balance")
        }

        fn withdrawal_transaction(transaction_id: u32, amount: Decimal) -> Transaction {
            Transaction::new(
                TransactionId(transaction_id),
                TransactionKind::Withdrawal,
                TransactionAmount::new(amount).expect("valid withdrawal amount"),
            )
        }

        #[test]
        fn disputes_and_resolutions_preserve_total_and_allow_redisputes() {
            let mut account = Account::new(ClientId(42));
            account
                .apply(deposit(1, Decimal::new(12_345, 4)))
                .expect("deposit succeeds");
            account
                .apply(deposit(2, Decimal::new(1, 4)))
                .expect("deposit succeeds");
            account
                .apply(withdrawal(3, Decimal::ONE))
                .expect("withdrawal succeeds");

            for (command, available, held, first_state, second_state) in [
                (dispute(1), -9_999, 12_345, Disputed, Normal),
                (dispute(2), -10_000, 12_346, Disputed, Disputed),
                (resolve(1), 2_345, 1, Normal, Disputed),
                (dispute(1), -10_000, 12_346, Disputed, Disputed),
                (resolve(2), -9_999, 12_345, Disputed, Normal),
            ] {
                account.apply(command).expect("dispute transition succeeds");
                assert_eq!(
                    account,
                    Account {
                        client_id: ClientId(42),
                        balance: test_balance(Decimal::new(available, 4), Decimal::new(held, 4)),
                        status: Active,
                        transactions: HashMap::from([
                            (
                                TransactionId(1),
                                deposit_transaction(1, Decimal::new(12_345, 4))
                                    .with_test_state(first_state)
                            ),
                            (
                                TransactionId(2),
                                deposit_transaction(2, Decimal::new(1, 4))
                                    .with_test_state(second_state)
                            ),
                            (TransactionId(3), withdrawal_transaction(3, Decimal::ONE)),
                        ]),
                    }
                );
            }
        }

        #[test]
        fn chargeback_freezes_withdrawals_but_allows_deposits_and_dispute_processing() {
            let mut account = Account::new(ClientId(42));
            for command in [
                deposit(1, Decimal::new(2, 0)),
                deposit(2, Decimal::ONE),
                dispute(1),
                dispute(2),
                chargeback(1),
            ] {
                account.apply(command).expect("command succeeds");
            }
            assert_eq!(
                account,
                Account {
                    client_id: ClientId(42),
                    balance: test_balance(Decimal::ZERO, Decimal::ONE),
                    status: Locked,
                    transactions: HashMap::from([
                        (
                            TransactionId(1),
                            deposit_transaction(1, Decimal::new(2, 0)).with_test_state(ChargedBack)
                        ),
                        (
                            TransactionId(2),
                            deposit_transaction(2, Decimal::ONE).with_test_state(Disputed)
                        ),
                    ]),
                }
            );

            for (command, available, held, state) in [
                (deposit(3, Decimal::ONE), 1, 1, Disputed),
                (withdrawal(4, Decimal::ONE), 1, 1, Disputed),
                (resolve(2), 2, 0, Normal),
                (dispute(2), 1, 1, Disputed),
                (chargeback(2), 1, 0, ChargedBack),
            ] {
                account.apply(command).expect("command is handled");
                assert_eq!(
                    account,
                    Account {
                        client_id: ClientId(42),
                        balance: test_balance(Decimal::new(available, 0), Decimal::new(held, 0)),
                        status: Locked,
                        transactions: HashMap::from([
                            (
                                TransactionId(1),
                                deposit_transaction(1, Decimal::new(2, 0))
                                    .with_test_state(ChargedBack)
                            ),
                            (
                                TransactionId(2),
                                deposit_transaction(2, Decimal::ONE).with_test_state(state)
                            ),
                            (TransactionId(3), deposit_transaction(3, Decimal::ONE)),
                        ]),
                    }
                );
            }
        }

        #[rstest]
        #[case::smallest_amount(Decimal::new(1, 4))]
        #[case::maximum_amount(Money::MAX.value())]
        fn a_spent_deposit_can_be_disputed_resolved_and_charged_back(#[case] amount: Decimal) {
            let mut account = Account::new(ClientId(42));
            account
                .apply(deposit(u32::MAX, amount))
                .expect("deposit succeeds");
            account
                .apply(withdrawal(0, amount))
                .expect("withdrawal succeeds");

            for (command, available, held, state, status) in [
                (dispute(u32::MAX), -amount, amount, Disputed, Active),
                (
                    resolve(u32::MAX),
                    Decimal::ZERO,
                    Decimal::ZERO,
                    Normal,
                    Active,
                ),
                (dispute(u32::MAX), -amount, amount, Disputed, Active),
                (
                    chargeback(u32::MAX),
                    -amount,
                    Decimal::ZERO,
                    ChargedBack,
                    Locked,
                ),
            ] {
                account.apply(command).expect("dispute transition succeeds");
                assert_eq!(
                    account,
                    Account {
                        client_id: ClientId(42),
                        balance: test_balance(available, held),
                        status,
                        transactions: HashMap::from([
                            (
                                TransactionId(u32::MAX),
                                deposit_transaction(u32::MAX, amount).with_test_state(state)
                            ),
                            (TransactionId(0), withdrawal_transaction(0, amount)),
                        ]),
                    }
                );
            }
        }

        #[rstest]
        #[case::missing_dispute(Normal, 1, 0, Active, dispute(99))]
        #[case::missing_resolve(Normal, 1, 0, Active, resolve(99))]
        #[case::missing_chargeback(Normal, 1, 0, Active, chargeback(99))]
        #[case::withdrawal_dispute(Normal, 1, 0, Active, dispute(2))]
        #[case::withdrawal_resolve(Normal, 1, 0, Active, resolve(2))]
        #[case::withdrawal_chargeback(Normal, 1, 0, Active, chargeback(2))]
        #[case::repeated_dispute(Disputed, -1, 2, Active, dispute(1))]
        #[case::undisputed_resolve(Normal, 1, 0, Active, resolve(1))]
        #[case::undisputed_chargeback(Normal, 1, 0, Active, chargeback(1))]
        #[case::charged_back_dispute(ChargedBack, -1, 0, Locked, dispute(1))]
        #[case::charged_back_resolve(ChargedBack, -1, 0, Locked, resolve(1))]
        #[case::repeated_chargeback(ChargedBack, -1, 0, Locked, chargeback(1))]
        fn invalid_dispute_transitions_are_ignored(
            #[case] state: TransactionState,
            #[case] available: i64,
            #[case] held: i64,
            #[case] status: AccountStatus,
            #[case] command: PaymentCommand,
        ) {
            let balance = test_balance(Decimal::new(available, 0), Decimal::new(held, 0));
            let mut account = Account {
                client_id: ClientId(42),
                balance,
                status,
                transactions: HashMap::from([
                    (
                        TransactionId(1),
                        deposit_transaction(1, Decimal::new(2, 0)).with_test_state(state),
                    ),
                    (TransactionId(2), withdrawal_transaction(2, Decimal::ONE)),
                ]),
            };

            assert_eq!(account.apply(command), Ok(()));
            assert_eq!(
                account,
                Account {
                    client_id: ClientId(42),
                    balance,
                    status,
                    transactions: HashMap::from([
                        (
                            TransactionId(1),
                            deposit_transaction(1, Decimal::new(2, 0)).with_test_state(state)
                        ),
                        (TransactionId(2), withdrawal_transaction(2, Decimal::ONE)),
                    ]),
                }
            );
        }

        #[rstest]
        #[case::available_underflow(Money::MIN, Money::new(Decimal::ZERO).expect("valid zero"))]
        #[case::held_overflow(Money::new(Decimal::ZERO).expect("valid zero"), Money::MAX)]
        fn out_of_range_disputes_leave_balances_and_history_unchanged(
            #[case] available: Money,
            #[case] held: Money,
        ) {
            let mut account = Account::new(ClientId(42)).with_test_state(
                available,
                held,
                Active,
                HashMap::from([(TransactionId(1), deposit_transaction(1, Decimal::new(1, 4)))]),
            );

            assert_eq!(
                account.apply(dispute(1)),
                Err(DomainError::AmountOutOfRange)
            );
            assert_eq!(
                account,
                Account {
                    client_id: ClientId(42),
                    balance: Balance::new(available, held).expect("valid balance"),
                    status: Active,
                    transactions: HashMap::from([(
                        TransactionId(1),
                        deposit_transaction(1, Decimal::new(1, 4)),
                    )]),
                }
            );
        }
    }
}
