use rust_decimal::Decimal;

use super::types::{DomainError, Money};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransactionId(pub u32);

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

#[derive(Debug, PartialEq, Eq)]
pub struct Transaction {
    id: TransactionId,
    kind: TransactionKind,
    amount: TransactionAmount,
    state: TransactionState,
}

impl Transaction {
    pub(crate) fn new(id: TransactionId, kind: TransactionKind, amount: TransactionAmount) -> Self {
        Self {
            id,
            kind,
            amount,
            state: TransactionState::Normal,
        }
    }

    pub fn id(&self) -> TransactionId {
        self.id
    }

    pub fn kind(&self) -> TransactionKind {
        self.kind
    }

    pub fn amount(&self) -> TransactionAmount {
        self.amount
    }

    pub fn state(&self) -> TransactionState {
        self.state
    }

    pub fn can_be_disputed(&self) -> bool {
        self.kind == TransactionKind::Deposit && self.state == TransactionState::Normal
    }

    pub fn is_disputed(&self) -> bool {
        self.state == TransactionState::Disputed
    }

    pub(crate) fn dispute(&mut self) -> Result<(), DomainError> {
        if !self.can_be_disputed() {
            return Err(DomainError::InvalidTransactionTransition);
        }
        self.state = TransactionState::Disputed;
        Ok(())
    }

    pub(crate) fn resolve(&mut self) -> Result<(), DomainError> {
        if !self.is_disputed() {
            return Err(DomainError::InvalidTransactionTransition);
        }
        self.state = TransactionState::Normal;
        Ok(())
    }

    pub(crate) fn chargeback(&mut self) -> Result<(), DomainError> {
        if !self.is_disputed() {
            return Err(DomainError::InvalidTransactionTransition);
        }
        self.state = TransactionState::ChargedBack;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn with_test_state(mut self, state: TransactionState) -> Self {
        self.state = state;
        self
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use rust_decimal::Decimal;

    use crate::domain::{
        transaction::{
            Transaction, TransactionAmount, TransactionId, TransactionKind, TransactionState,
        },
        types::DomainError,
    };

    type Transition = fn(&mut Transaction) -> Result<(), DomainError>;

    #[rstest]
    #[case::dispute(&[Transaction::dispute as Transition], TransactionState::Disputed)]
    #[case::resolve(
        &[Transaction::dispute, Transaction::resolve],
        TransactionState::Normal
    )]
    #[case::redispute(
        &[Transaction::dispute, Transaction::resolve, Transaction::dispute],
        TransactionState::Disputed
    )]
    #[case::chargeback(
        &[Transaction::dispute, Transaction::resolve, Transaction::dispute, Transaction::chargeback],
        TransactionState::ChargedBack
    )]
    fn allowed_transitions_preserve_transaction_details(
        #[case] transitions: &[Transition],
        #[case] expected_state: TransactionState,
    ) {
        let amount = TransactionAmount::new(Decimal::new(12_345, 4)).expect("valid amount");
        let mut transaction =
            Transaction::new(TransactionId(u32::MAX), TransactionKind::Deposit, amount);

        assert_eq!(
            transaction,
            Transaction {
                id: TransactionId(u32::MAX),
                kind: TransactionKind::Deposit,
                amount,
                state: TransactionState::Normal,
            }
        );

        for transition in transitions {
            assert_eq!(transition(&mut transaction), Ok(()));
        }

        assert_eq!(
            transaction,
            Transaction {
                id: TransactionId(u32::MAX),
                kind: TransactionKind::Deposit,
                amount,
                state: expected_state,
            }
        );
    }

    #[rstest]
    #[case::withdrawal_dispute(
        TransactionKind::Withdrawal,
        TransactionState::Normal,
        Transaction::dispute
    )]
    #[case::withdrawal_resolve(
        TransactionKind::Withdrawal,
        TransactionState::Normal,
        Transaction::resolve
    )]
    #[case::withdrawal_chargeback(
        TransactionKind::Withdrawal,
        TransactionState::Normal,
        Transaction::chargeback
    )]
    #[case::undisputed_resolve(
        TransactionKind::Deposit,
        TransactionState::Normal,
        Transaction::resolve
    )]
    #[case::undisputed_chargeback(
        TransactionKind::Deposit,
        TransactionState::Normal,
        Transaction::chargeback
    )]
    #[case::repeated_dispute(
        TransactionKind::Deposit,
        TransactionState::Disputed,
        Transaction::dispute
    )]
    #[case::charged_back_dispute(
        TransactionKind::Deposit,
        TransactionState::ChargedBack,
        Transaction::dispute
    )]
    #[case::charged_back_resolve(
        TransactionKind::Deposit,
        TransactionState::ChargedBack,
        Transaction::resolve
    )]
    #[case::repeated_chargeback(
        TransactionKind::Deposit,
        TransactionState::ChargedBack,
        Transaction::chargeback
    )]
    fn invalid_transitions_leave_the_transaction_unchanged(
        #[case] kind: TransactionKind,
        #[case] state: TransactionState,
        #[case] transition: Transition,
    ) {
        let amount = TransactionAmount::new(Decimal::ONE).expect("valid amount");
        let mut transaction = Transaction {
            id: TransactionId(1),
            kind,
            amount,
            state,
        };

        assert_eq!(
            transition(&mut transaction),
            Err(DomainError::InvalidTransactionTransition)
        );
        assert_eq!(
            transaction,
            Transaction {
                id: TransactionId(1),
                kind,
                amount,
                state,
            }
        );
    }

    mod transaction_amount {
        use rstest::rstest;
        use rust_decimal::Decimal;

        use crate::domain::{
            transaction::TransactionAmount,
            types::{DomainError, Money},
        };

        #[rstest]
        #[case::smallest_amount(Decimal::new(1, 4))]
        #[case::maximum_amount(Money::MAX.value())]
        fn accepts_positive_money(#[case] value: Decimal) {
            let amount = TransactionAmount::new(value).expect("positive money should be accepted");

            assert_eq!(
                amount,
                TransactionAmount(Money::new(value).expect("valid money"))
            );
        }

        #[rstest]
        #[case::zero(Decimal::ZERO, DomainError::NonPositiveAmount)]
        #[case::negative(Decimal::new(-1, 4), DomainError::NonPositiveAmount)]
        #[case::excessive_precision(Decimal::new(1, 5), DomainError::ExcessivePrecision)]
        #[case::above_money_limit(
            Money::MAX.value() + Decimal::new(1, 4),
            DomainError::AmountOutOfRange
        )]
        fn rejects_invalid_amounts(#[case] value: Decimal, #[case] expected: DomainError) {
            assert_eq!(TransactionAmount::new(value), Err(expected));
        }
    }
}
