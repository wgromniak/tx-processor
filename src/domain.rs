use std::collections::HashMap;

use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientId(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransactionId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Money(Decimal);

impl Money {
    pub fn new(_value: Decimal) -> Result<Self, DomainError> {
        unimplemented!()
    }

    pub fn value(self) -> Decimal {
        unimplemented!()
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

#[derive(Debug)]
pub struct DomainError;
