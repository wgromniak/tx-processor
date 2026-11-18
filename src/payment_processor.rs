use std::collections::HashMap;

use crate::{
    account::Account,
    domain_types::{ClientId, DomainError, PaymentCommand},
};

#[derive(Debug, PartialEq, Eq)]
pub struct PaymentProcessor {
    accounts: HashMap<ClientId, Account>,
}

#[allow(clippy::new_without_default)]
impl PaymentProcessor {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
        }
    }

    pub fn process(&mut self, command: PaymentCommand) -> Result<(), DomainError> {
        let client_id = command.client_id();
        self.accounts
            .entry(client_id)
            .or_insert_with(|| Account::new(client_id))
            .apply(command)
    }

    pub fn accounts(&self) -> Vec<&Account> {
        self.accounts.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rust_decimal::Decimal;

    use super::PaymentProcessor;
    use crate::account::Account;
    use crate::domain_types::{
        AccountStatus, ClientId, DomainError, Money, PaymentCommand, Transaction,
        TransactionAmount, TransactionId, TransactionKind, TransactionState,
    };

    #[test]
    fn new_processor_has_no_accounts() {
        assert_eq!(
            PaymentProcessor::new(),
            PaymentProcessor {
                accounts: HashMap::new(),
            }
        );
    }

    mod process {
        use super::*;

        fn deposit(client_id: u16, transaction_id: u32, amount: Decimal) -> PaymentCommand {
            PaymentCommand::Deposit {
                client_id: ClientId(client_id),
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
        fn deposit_creates_and_credits_a_new_client_account() {
            let mut processor = PaymentProcessor::new();
            let amount = Decimal::new(12_345, 4);

            processor
                .process(deposit(42, 7, amount))
                .expect("deposit should succeed");

            assert_eq!(
                processor,
                PaymentProcessor {
                    accounts: HashMap::from([(
                        ClientId(42),
                        Account::new(ClientId(42)).with_test_state(
                            Money::new(amount).expect("valid available"),
                            Money::new(Decimal::ZERO).expect("valid held"),
                            AccountStatus::Active,
                            HashMap::from([(TransactionId(7), deposit_transaction(7, amount))]),
                        ),
                    )]),
                }
            );
        }

        #[test]
        fn interleaved_deposits_update_only_their_client_accounts() {
            let mut processor = PaymentProcessor::new();
            for (client_id, id, amount) in [
                (u16::MAX, u32::MAX, Decimal::new(2, 0)),
                (0, 0, Decimal::ONE),
                (u16::MAX, 7, Decimal::new(1, 4)),
            ] {
                processor
                    .process(deposit(client_id, id, amount))
                    .expect("deposit should succeed");
            }

            assert_eq!(
                processor,
                PaymentProcessor {
                    accounts: HashMap::from([
                        (
                            ClientId(0),
                            Account::new(ClientId(0)).with_test_state(
                                Money::new(Decimal::ONE).expect("valid available"),
                                Money::new(Decimal::ZERO).expect("valid held"),
                                AccountStatus::Active,
                                HashMap::from([(
                                    TransactionId(0),
                                    deposit_transaction(0, Decimal::ONE),
                                )]),
                            ),
                        ),
                        (
                            ClientId(u16::MAX),
                            Account::new(ClientId(u16::MAX)).with_test_state(
                                Money::new(Decimal::new(20_001, 4)).expect("valid available"),
                                Money::new(Decimal::ZERO).expect("valid held"),
                                AccountStatus::Active,
                                HashMap::from([
                                    (
                                        TransactionId(u32::MAX),
                                        deposit_transaction(u32::MAX, Decimal::new(2, 0)),
                                    ),
                                    (TransactionId(7), deposit_transaction(7, Decimal::new(1, 4))),
                                ]),
                            ),
                        ),
                    ]),
                }
            );
        }

        #[test]
        fn deposit_propagates_account_errors_without_partial_updates() {
            let mut processor = PaymentProcessor::new();
            processor
                .process(deposit(42, 1, Money::MAX.value()))
                .expect("first deposit should succeed");

            assert_eq!(
                processor.process(deposit(42, 2, Decimal::ONE)),
                Err(DomainError::AmountOutOfRange)
            );

            assert_eq!(
                processor,
                PaymentProcessor {
                    accounts: HashMap::from([(
                        ClientId(42),
                        Account::new(ClientId(42)).with_test_state(
                            Money::new(Money::MAX.value()).expect("valid available"),
                            Money::new(Decimal::ZERO).expect("valid held"),
                            AccountStatus::Active,
                            HashMap::from([(
                                TransactionId(1),
                                deposit_transaction(1, Money::MAX.value()),
                            )]),
                        ),
                    )]),
                }
            );
        }
    }
}
