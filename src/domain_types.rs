use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientId(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransactionId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Money(Decimal);

impl Money {
    /// Maximum magnitude: `(2^95 - 1) / 10_000`, approximately 3.96e24.
    /// Reserving one coefficient bit makes the sum of any two valid values exact
    /// at four decimal places, before checking whether the result is in range.
    pub const MAX: Self = Self(Decimal::from_parts(
        u32::MAX,
        u32::MAX,
        u32::MAX / 2,
        false,
        4,
    ));
    pub const MIN: Self = Self(Decimal::from_parts(
        u32::MAX,
        u32::MAX,
        u32::MAX / 2,
        true,
        4,
    ));

    pub fn new(value: Decimal) -> Result<Self, DomainError> {
        if value.scale() > 4 {
            return Err(DomainError::ExcessivePrecision);
        }
        if !(Self::MIN.0..=Self::MAX.0).contains(&value) {
            return Err(DomainError::AmountOutOfRange);
        }
        Ok(Self(value))
    }

    pub fn value(self) -> Decimal {
        self.0
    }

    pub(crate) fn checked_add(self, other: Self) -> Result<Self, DomainError> {
        Self::new(self.0 + other.0)
    }

    pub fn checked_sub(self, other: Self) -> Result<Self, DomainError> {
        Self::new(self.0 - other.0)
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
    total: Money,
}

impl Balance {
    pub(crate) const ZERO: Self = Self {
        available: Money(Decimal::ZERO),
        held: Money(Decimal::ZERO),
        total: Money(Decimal::ZERO),
    };

    pub(crate) fn new(available: Money, held: Money) -> Result<Self, DomainError> {
        let total = available.checked_add(held)?;
        Ok(Self {
            available,
            held,
            total,
        })
    }

    pub fn available(&self) -> Money {
        self.available
    }

    pub fn held(&self) -> Money {
        self.held
    }

    pub fn total(&self) -> Money {
        self.total
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

#[derive(Debug, PartialEq, Eq)]
pub struct Transaction {
    id: TransactionId,
    kind: TransactionKind,
    amount: Money,
    state: TransactionState,
}

impl Transaction {
    pub(crate) fn new(id: TransactionId, kind: TransactionKind, amount: TransactionAmount) -> Self {
        Self {
            id,
            kind,
            amount: amount.money(),
            state: TransactionState::Normal,
        }
    }

    pub fn id(&self) -> TransactionId {
        self.id
    }

    pub fn kind(&self) -> TransactionKind {
        self.kind
    }

    pub fn amount(&self) -> Money {
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
        match *self {
            Self::Deposit { client_id, .. }
            | Self::Withdrawal { client_id, .. }
            | Self::Dispute { client_id, .. }
            | Self::Resolve { client_id, .. }
            | Self::Chargeback { client_id, .. } => client_id,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum DomainError {
    ExcessivePrecision,
    NonPositiveAmount,
    ClientMismatch,
    DuplicateTransaction,
    AmountOutOfRange,
    InvalidTransactionTransition,
}

#[cfg(test)]
mod tests {
    mod transaction {
        use rstest::rstest;
        use rust_decimal::Decimal;

        use crate::domain_types::{
            DomainError, Transaction, TransactionAmount, TransactionId, TransactionKind,
            TransactionState,
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
                    amount: amount.money(),
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
                    amount: amount.money(),
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
                amount: amount.money(),
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
                    amount: amount.money(),
                    state,
                }
            );
        }
    }

    mod balance {
        use rust_decimal::Decimal;

        use crate::domain_types::{Balance, DomainError, Money};

        #[test]
        fn new_balance_stores_the_validated_total() {
            let available = Money::new(Decimal::new(-1_234, 4)).expect("valid available balance");
            let held = Money::new(Decimal::new(2, 0)).expect("valid held balance");

            assert_eq!(
                Balance::new(available, held),
                Ok(Balance {
                    available,
                    held,
                    total: Money(Decimal::new(18_766, 4)),
                })
            );
        }

        #[test]
        fn new_balance_rejects_an_out_of_range_total() {
            let balance = Balance::new(
                Money::MAX,
                Money::new(Decimal::new(1, 4)).expect("valid held"),
            );

            assert_eq!(balance, Err(DomainError::AmountOutOfRange));
        }
    }

    mod money {
        use rstest::rstest;
        use rust_decimal::Decimal;

        use crate::domain_types::{DomainError, Money};

        #[rstest]
        #[case::zero(Decimal::ZERO)]
        #[case::smallest_positive(Decimal::new(1, 4))]
        #[case::negative_fraction(Decimal::new(-1_234, 4))]
        #[case::lower_limit(Money::MIN.value())]
        #[case::upper_limit(Money::MAX.value())]
        fn new_preserves_valid_values(#[case] value: Decimal) {
            let money = Money::new(value).expect("valid money");

            assert_eq!(money, Money(value));
            assert_eq!(money.value().scale(), value.scale());
        }

        #[rstest]
        #[case::fraction(Decimal::new(1, 5))]
        #[case::trailing_zero(Decimal::new(12_340, 5))]
        fn new_rejects_excessive_precision(#[case] value: Decimal) {
            assert_eq!(Money::new(value), Err(DomainError::ExcessivePrecision));
        }

        #[rstest]
        #[case::above_limit(Money::MAX.value() + Decimal::new(1, 4))]
        #[case::below_limit(Money::MIN.value() - Decimal::new(1, 4))]
        #[case::decimal_maximum(Decimal::MAX)]
        #[case::decimal_minimum(Decimal::MIN)]
        fn new_rejects_out_of_range_values(#[case] value: Decimal) {
            assert_eq!(Money::new(value), Err(DomainError::AmountOutOfRange));
        }

        #[rstest]
        #[case::fractions(Decimal::new(1, 1), Decimal::new(2, 1), Decimal::new(3, 1))]
        #[case::upper_limit(Money::MAX.value() - Decimal::new(1, 4), Decimal::new(1, 4), Money::MAX.value())]
        #[case::lower_limit(Money::MIN.value() + Decimal::new(1, 4), Decimal::new(-1, 4), Money::MIN.value())]
        #[case::mixed_scales(Money::MAX.value().trunc(), Decimal::new(5_167, 4), Money::MAX.value())]
        #[case::cancellation(Money::MAX.value(), Money::MIN.value(), Decimal::ZERO)]
        fn checked_add_returns_exact_sums(
            #[case] left: Decimal,
            #[case] right: Decimal,
            #[case] expected: Decimal,
        ) {
            let left = Money::new(left).expect("valid left operand");
            let right = Money::new(right).expect("valid right operand");

            assert_eq!(left.checked_add(right), Ok(Money(expected)));
        }

        #[rstest]
        #[case::above_limit(Money::MAX, Money(Decimal::new(1, 4)))]
        #[case::below_limit(Money::MIN, Money(Decimal::new(-1, 4)))]
        #[case::two_maxima(Money::MAX, Money::MAX)]
        #[case::two_minima(Money::MIN, Money::MIN)]
        fn checked_add_rejects_out_of_range_sums(#[case] left: Money, #[case] right: Money) {
            assert_eq!(left.checked_add(right), Err(DomainError::AmountOutOfRange));
        }

        #[rstest]
        #[case::fractions(Decimal::new(3, 1), Decimal::new(2, 1), Decimal::new(1, 1))]
        #[case::negative_result(Decimal::ZERO, Money::MAX.value(), Money::MIN.value())]
        #[case::mixed_scales(Money::MAX.value(), Money::MAX.value().trunc(), Decimal::new(5_167, 4))]
        #[case::cancellation(Money::MAX.value(), Money::MAX.value(), Decimal::ZERO)]
        fn checked_sub_returns_exact_differences(
            #[case] left: Decimal,
            #[case] right: Decimal,
            #[case] expected: Decimal,
        ) {
            let left = Money::new(left).expect("valid left operand");
            let right = Money::new(right).expect("valid right operand");

            assert_eq!(left.checked_sub(right), Ok(Money(expected)));
        }

        #[rstest]
        #[case::above_limit(Money::MAX, Money(Decimal::new(-1, 4)))]
        #[case::below_limit(Money::MIN, Money(Decimal::new(1, 4)))]
        #[case::maximum_minus_minimum(Money::MAX, Money::MIN)]
        #[case::minimum_minus_maximum(Money::MIN, Money::MAX)]
        fn checked_sub_rejects_out_of_range_differences(#[case] left: Money, #[case] right: Money) {
            assert_eq!(left.checked_sub(right), Err(DomainError::AmountOutOfRange));
        }
    }

    mod transaction_amount {
        use rstest::rstest;
        use rust_decimal::Decimal;

        use crate::domain_types::{DomainError, Money, TransactionAmount};

        #[rstest]
        #[case::positive(Decimal::new(1, 4), Ok(TransactionAmount(Money(Decimal::new(1, 4)))))]
        #[case::zero(Decimal::ZERO, Err(DomainError::NonPositiveAmount))]
        #[case::negative(Decimal::new(-1, 4), Err(DomainError::NonPositiveAmount))]
        fn transaction_amount_requires_positive_money(
            #[case] value: Decimal,
            #[case] expected: Result<TransactionAmount, DomainError>,
        ) {
            assert_eq!(TransactionAmount::new(value), expected);
        }

        #[test]
        fn transaction_amount_enforces_money_precision() {
            assert_eq!(
                TransactionAmount::new(Decimal::new(1, 5)),
                Err(DomainError::ExcessivePrecision)
            );
        }

        #[rstest]
        #[case::at_limit(Money::MAX.value(), Ok(TransactionAmount(Money::MAX)))]
        #[case::above_limit(Money::MAX.value() + Decimal::new(1, 4), Err(DomainError::AmountOutOfRange))]
        fn transaction_amount_enforces_the_money_limit(
            #[case] value: Decimal,
            #[case] expected: Result<TransactionAmount, DomainError>,
        ) {
            assert_eq!(TransactionAmount::new(value), expected);
        }
    }
}
