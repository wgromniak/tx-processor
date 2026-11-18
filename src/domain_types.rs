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
    pub(crate) const ZERO: Self = Self::new(Money(Decimal::ZERO), Money(Decimal::ZERO));

    pub(crate) const fn new(available: Money, held: Money) -> Self {
        Self { available, held }
    }

    pub fn available(&self) -> Money {
        self.available
    }

    pub fn held(&self) -> Money {
        self.held
    }

    pub fn total(&self) -> Result<Money, DomainError> {
        self.available.checked_add(self.held)
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
    AmountOutOfRange,
}

#[cfg(test)]
mod tests {
    mod balance {
        use rust_decimal::Decimal;

        use crate::domain_types::{Balance, DomainError, Money};

        #[test]
        fn balance_total_sums_available_and_held() {
            let balance = Balance {
                available: Money::new(Decimal::new(-1_234, 4)).expect("valid available balance"),
                held: Money::new(Decimal::new(2, 0)).expect("valid held balance"),
            };

            assert_eq!(balance.total(), Ok(Money(Decimal::new(18_766, 4))));
        }

        #[test]
        fn balance_total_rejects_amounts_outside_the_money_range() {
            let balance = Balance::new(
                Money::MAX,
                Money::new(Decimal::new(1, 4)).expect("valid held"),
            );

            assert_eq!(balance.total(), Err(DomainError::AmountOutOfRange));
        }
    }

    mod money {
        use rust_decimal::Decimal;

        use crate::domain_types::{DomainError, Money};

        #[test]
        fn money_preserves_signed_values_at_supported_precision() {
            for value in [Decimal::ZERO, Decimal::new(1, 4), Decimal::new(-1234, 4)] {
                let stored = Money::new(value).expect("money should support this value");
                assert_eq!(stored, Money(value));
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
        fn money_accepts_both_inclusive_limits() {
            for value in [Money::MIN.value(), Money::MAX.value()] {
                assert_eq!(Money::new(value), Ok(Money(value)));
            }
        }

        #[test]
        fn money_rejects_values_outside_the_range() {
            let smallest = Decimal::new(1, 4);
            for value in [
                Money::MAX.value() + smallest,
                Money::MIN.value() - smallest,
                Decimal::MAX,
                Decimal::MIN,
            ] {
                assert_eq!(Money::new(value), Err(DomainError::AmountOutOfRange));
            }
        }

        #[test]
        fn checked_add_preserves_four_decimal_places_at_both_limits() {
            let smallest = Decimal::new(1, 4);
            for (limit, step) in [(Money::MAX, smallest), (Money::MIN, -smallest)] {
                let near_limit = Money::new(limit.value() - step).expect("within range");
                let step = Money::new(step).expect("valid step");

                assert_eq!(near_limit.checked_add(step), Ok(limit));
                assert_eq!(step.checked_add(near_limit), Ok(limit));
            }
        }

        #[test]
        fn checked_add_rejects_results_outside_the_money_range() {
            let smallest = Money::new(Decimal::new(1, 4)).expect("valid step");
            let negative_smallest = Money::new(Decimal::new(-1, 4)).expect("valid step");
            for (left, right) in [
                (Money::MAX, smallest),
                (Money::MIN, negative_smallest),
                (Money::MAX, Money::MAX),
                (Money::MIN, Money::MIN),
            ] {
                assert_eq!(left.checked_add(right), Err(DomainError::AmountOutOfRange));
                assert_eq!(right.checked_add(left), Err(DomainError::AmountOutOfRange));
            }
        }

        #[test]
        fn checked_add_handles_cancellation_and_mixed_scales_exactly() {
            assert_eq!(Money::MAX.checked_add(Money::MIN), Ok(Money(Decimal::ZERO)));

            let integer = Money::new(Money::MAX.value().trunc()).expect("within range");
            let fraction = Money::new(Decimal::new(5_167, 4)).expect("valid fraction");
            assert_eq!(integer.checked_add(fraction), Ok(Money::MAX));
            assert_eq!(fraction.checked_add(integer), Ok(Money::MAX));
        }

        #[test]
        fn money_limits_leave_room_for_exact_four_decimal_addition() {
            let maximum_coefficient = (1_i128 << 95) - 1;
            assert_eq!(Money::MAX.value().scale(), 4);
            assert_eq!(Money::MAX.value().mantissa(), maximum_coefficient);
            assert_eq!(Money::MIN.value().mantissa(), -maximum_coefficient);

            let doubled = Money::MAX
                .value()
                .checked_add(Money::MAX.value())
                .expect("sum fits Decimal");
            assert_eq!(doubled.scale(), 4);
            assert_eq!(doubled.mantissa(), 2 * maximum_coefficient);
        }
    }

    mod transaction_amount {
        use rust_decimal::Decimal;

        use crate::domain_types::{DomainError, Money, TransactionAmount};

        #[test]
        fn transaction_amount_requires_positive_money() {
            let value = Decimal::new(1, 4);
            let amount = TransactionAmount::new(value).expect("positive money should be accepted");
            assert_eq!(amount, TransactionAmount(Money(value)));

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

        #[test]
        fn transaction_amount_enforces_the_money_limit() {
            assert_eq!(
                TransactionAmount::new(Money::MAX.value()),
                Ok(TransactionAmount(Money::MAX))
            );
            assert_eq!(
                TransactionAmount::new(Money::MAX.value() + Decimal::new(1, 4)),
                Err(DomainError::AmountOutOfRange)
            );
        }
    }
}
