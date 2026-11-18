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
        unimplemented!()
    }

    pub fn held(&self) -> Money {
        unimplemented!()
    }

    pub fn total(&self) -> Money {
        unimplemented!()
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
    pub fn new(_client_id: ClientId) -> Self {
        unimplemented!()
    }

    pub fn client_id(&self) -> ClientId {
        unimplemented!()
    }

    pub fn balance(&self) -> &Balance {
        unimplemented!()
    }

    pub fn status(&self) -> AccountStatus {
        unimplemented!()
    }

    pub fn transaction(&self, _transaction_id: TransactionId) -> Option<&Transaction> {
        unimplemented!()
    }

    pub fn apply(&mut self, _command: PaymentCommand) -> Result<(), DomainError> {
        unimplemented!()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum DomainError {
    ExcessivePrecision,
    NonPositiveAmount,
}

#[cfg(test)]
mod tests {
    use super::{DomainError, Money, TransactionAmount};
    use rust_decimal::Decimal;

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
