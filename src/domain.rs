//! Domain types and the account aggregate, independent of CSV and persistence.

use std::collections::HashMap;

use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientId(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransactionId(pub u32);

/// A decimal amount with up to four fractional digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Money(Decimal);

impl Money {
    /// # Errors
    /// Returns a domain error if the amount is invalid.
    pub fn new(_value: Decimal) -> Result<Self, DomainError> {
        unimplemented!()
    }

    #[must_use]
    pub fn value(self) -> Decimal {
        unimplemented!()
    }
}

/// The total is derived from available and held funds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Balance {
    available: Money,
    held: Money,
}

impl Balance {
    #[must_use]
    pub fn available(&self) -> Money {
        unimplemented!()
    }

    #[must_use]
    pub fn held(&self) -> Money {
        unimplemented!()
    }

    #[must_use]
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

/// An entity created and mutated only by its owning account.
#[derive(Debug)]
pub struct Transaction {
    id: TransactionId,
    kind: TransactionKind,
    amount: Money,
    state: TransactionState,
}

impl Transaction {
    #[must_use]
    pub fn id(&self) -> TransactionId {
        unimplemented!()
    }

    #[must_use]
    pub fn kind(&self) -> TransactionKind {
        unimplemented!()
    }

    #[must_use]
    pub fn amount(&self) -> Money {
        unimplemented!()
    }

    #[must_use]
    pub fn state(&self) -> TransactionState {
        unimplemented!()
    }
}

/// Typed input to the domain; dispute operations reference an existing transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaymentCommand {
    Deposit {
        client_id: ClientId,
        transaction_id: TransactionId,
        amount: Money,
    },
    Withdrawal {
        client_id: ClientId,
        transaction_id: TransactionId,
        amount: Money,
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
    #[must_use]
    pub fn client_id(&self) -> ClientId {
        unimplemented!()
    }
}

/// The aggregate root: balances and transaction state change through this account.
#[derive(Debug)]
pub struct Account {
    client_id: ClientId,
    balance: Balance,
    status: AccountStatus,
    transactions: HashMap<TransactionId, Transaction>,
}

impl Account {
    #[must_use]
    pub fn new(_client_id: ClientId) -> Self {
        unimplemented!()
    }

    #[must_use]
    pub fn client_id(&self) -> ClientId {
        unimplemented!()
    }

    #[must_use]
    pub fn balance(&self) -> &Balance {
        unimplemented!()
    }

    #[must_use]
    pub fn status(&self) -> AccountStatus {
        unimplemented!()
    }

    #[must_use]
    pub fn transaction(&self, _transaction_id: TransactionId) -> Option<&Transaction> {
        unimplemented!()
    }

    /// # Errors
    /// Returns a domain error if the command cannot be applied to this account.
    pub fn apply(&mut self, _command: PaymentCommand) -> Result<(), DomainError> {
        unimplemented!()
    }
}

/// Placeholder for domain errors; error cases will be defined with the behavior.
#[derive(Debug)]
pub struct DomainError;
