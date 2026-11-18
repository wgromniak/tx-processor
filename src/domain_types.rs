use std::collections::HashMap;

use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientId(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransactionId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Money(Decimal);

impl Money {
    pub fn new(value: Decimal) -> Result<Self, DomainError> {
        if value.scale() > 4 {
            return Err(DomainError::ExcessivePrecision);
        }
        Ok(Self(value))
    }

    pub fn value(self) -> Decimal {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransactionAmount(Money);

impl TransactionAmount {
    pub fn new(value: Decimal) -> Result<Self, DomainError> {
        let money = Money::new(value)?;
        if value <= Decimal::ZERO {
            return Err(DomainError::NonPositiveAmount);
        }
        Ok(Self(money))
    }

    pub fn money(self) -> Money {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Balance {
    available: Money,
    held: Money,
}

impl Balance {
    pub fn available(&self) -> Money {
        self.available
    }

    pub fn held(&self) -> Money {
        self.held
    }

    pub fn total(&self) -> Money {
        Money(self.available.value() + self.held.value())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountStatus {
    Active,
    Locked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionKind {
    Deposit,
    Withdrawal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionState {
    Normal,
    Disputed,
    ChargedBack,
}

#[derive(Debug)]
pub struct Transaction {
    id: TransactionId,
    kind: TransactionKind,
    amount: Money,
    state: TransactionState,
}

impl Transaction {
    pub fn id(&self) -> TransactionId {
        unimplemented!()
    }

    pub fn kind(&self) -> TransactionKind {
        unimplemented!()
    }

    pub fn amount(&self) -> Money {
        unimplemented!()
    }

    pub fn state(&self) -> TransactionState {
        unimplemented!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaymentCommand {
    Deposit {
        client_id: ClientId,
        transaction_id: TransactionId,
        amount: TransactionAmount,
    },
    Withdrawal {
        client_id: ClientId,
        transaction_id: TransactionId,
        amount: TransactionAmount,
    },
    Dispute {
        client_id: ClientId,
        transaction_id: TransactionId,
    },
    Resolve {
        client_id: ClientId,
        transaction_id: TransactionId,
    },
    Chargeback {
        client_id: ClientId,
        transaction_id: TransactionId,
    },
}

impl PaymentCommand {
    pub fn client_id(&self) -> ClientId {
        unimplemented!()
    }
}

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
            balance: Balance {
                available: Money(Decimal::ZERO),
                held: Money(Decimal::ZERO),
            },
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

    pub fn transaction(&self, _transaction_id: TransactionId) -> Option<&Transaction> {
        unimplemented!()
    }

    pub fn apply(&mut self, _command: PaymentCommand) -> Result<(), DomainError> {
        unimplemented!()
    }

    #[cfg(test)]
    pub(crate) fn with_test_state(
        mut self,
        available: Money,
        held: Money,
        status: AccountStatus,
    ) -> Self {
        self.balance = Balance { available, held };
        self.status = status;
        self
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum DomainError {
    ExcessivePrecision,
    NonPositiveAmount,
}

#[cfg(test)]
mod tests {
    use super::{Account, AccountStatus, Balance, ClientId, DomainError, Money, TransactionAmount};
    use rust_decimal::Decimal;

    #[test]
    fn new_account_starts_active_with_a_zero_balance() {
        let account = Account::new(ClientId(42));

        assert_eq!(account.client_id(), ClientId(42));
        assert_eq!(account.status(), AccountStatus::Active);
        assert_eq!(account.balance().available().value(), Decimal::ZERO);
        assert_eq!(account.balance().held().value(), Decimal::ZERO);
        assert_eq!(account.balance().total().value(), Decimal::ZERO);
    }

    #[test]
    fn balance_total_sums_available_and_held() {
        let balance = Balance {
            available: Money::new(Decimal::new(-1_234, 4)).expect("valid available balance"),
            held: Money::new(Decimal::new(2, 0)).expect("valid held balance"),
        };

        assert_eq!(balance.total().value(), Decimal::new(18_766, 4));
    }

    #[test]
    fn money_preserves_signed_values_at_supported_precision() {
        for value in [Decimal::ZERO, Decimal::new(1, 4), Decimal::new(-1234, 4)] {
            let stored = Money::new(value).expect("money should support this value");
            assert_eq!(stored.value(), value);
            assert_eq!(stored.value().scale(), value.scale());
        }
    }

    #[test]
    fn money_rejects_more_than_four_decimal_places() {
        assert_eq!(
            Money::new(Decimal::new(12_340, 5)),
            Err(DomainError::ExcessivePrecision)
        );
    }

    #[test]
    fn transaction_amount_requires_positive_money() {
        let value = Decimal::new(1, 4);
        let amount = TransactionAmount::new(value).expect("positive money should be accepted");
        assert_eq!(amount.money().value(), value);

        for value in [Decimal::ZERO, Decimal::new(-1, 4)] {
            assert_eq!(
                TransactionAmount::new(value),
                Err(DomainError::NonPositiveAmount)
            );
        }
    }

    #[test]
    fn transaction_amount_enforces_money_precision() {
        assert_eq!(
            TransactionAmount::new(Decimal::new(1, 5)),
            Err(DomainError::ExcessivePrecision)
        );
    }
}
