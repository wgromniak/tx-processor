use std::io::{Read, Write};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::domain::{
    account::Account,
    transaction::{TransactionAmount, TransactionId},
    types::{AccountStatus, ClientId, DomainError, PaymentCommand},
};

pub fn read_transactions(
    reader: impl Read,
) -> Result<impl Iterator<Item = Result<TransactionRow, csv::Error>>, csv::Error> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(reader);
    reader.headers()?;

    Ok(reader.into_deserialize())
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct TransactionRow {
    pub r#type: String,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<String>,
}

impl TransactionRow {
    pub fn into_command(self) -> Result<PaymentCommand, CsvError> {
        let client_id = ClientId(self.client);
        let transaction_id = TransactionId(self.tx);

        match (self.r#type.as_str(), self.amount.as_deref()) {
            ("deposit", Some(amount)) => Ok(PaymentCommand::Deposit {
                client_id,
                transaction_id,
                amount: parse_amount(amount)?,
            }),
            ("withdrawal", Some(amount)) => Ok(PaymentCommand::Withdrawal {
                client_id,
                transaction_id,
                amount: parse_amount(amount)?,
            }),
            ("dispute", None) => Ok(PaymentCommand::Dispute {
                client_id,
                transaction_id,
            }),
            ("resolve", None) => Ok(PaymentCommand::Resolve {
                client_id,
                transaction_id,
            }),
            ("chargeback", None) => Ok(PaymentCommand::Chargeback {
                client_id,
                transaction_id,
            }),
            ("deposit" | "withdrawal", None) => Err(CsvError::MissingAmount),
            ("dispute" | "resolve" | "chargeback", Some(_)) => Err(CsvError::UnexpectedAmount),
            _ => Err(CsvError::UnknownTransactionType(self.r#type)),
        }
    }
}

fn parse_amount(amount: &str) -> Result<TransactionAmount, CsvError> {
    let value = Decimal::from_str_exact(amount).map_err(CsvError::InvalidDecimal)?;
    TransactionAmount::new(value).map_err(CsvError::InvalidAmount)
}

pub fn write_accounts(
    writer: impl Write,
    rows: impl IntoIterator<Item = AccountRow>,
) -> Result<(), csv::Error> {
    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(writer);
    writer.write_record(["client", "available", "held", "total", "locked"])?;

    for row in rows {
        writer.serialize(row)?;
    }

    writer.flush()?;
    Ok(())
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct AccountRow {
    pub client: u16,
    pub available: String,
    pub held: String,
    pub total: String,
    pub locked: bool,
}

impl AccountRow {
    pub fn from_account(account: &Account) -> Self {
        let balance = account.balance();
        Self {
            client: account.client_id().0,
            available: balance.available().value().to_string(),
            held: balance.held().value().to_string(),
            total: balance.total().value().to_string(),
            locked: account.status() == AccountStatus::Locked,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum CsvError {
    UnknownTransactionType(String),
    MissingAmount,
    UnexpectedAmount,
    InvalidDecimal(rust_decimal::Error),
    InvalidAmount(DomainError),
}

#[cfg(test)]
mod tests {
    mod from_account {
        use std::collections::HashMap;

        use rstest::rstest;
        use rust_decimal::Decimal;

        use crate::{
            csv_io::AccountRow,
            domain::account::Account,
            domain::types::{AccountStatus, ClientId, Money},
        };

        #[rstest]
        #[case::new_account(
            Account::new(ClientId(42)),
            AccountRow {
                client: 42,
                available: "0".into(),
                held: "0".into(),
                total: "0".into(),
                locked: false,
            },
        )]
        #[case::locked_account_with_fractional_balances(
            Account::new(ClientId(7)).with_test_state(
                Money::new(Decimal::new(-1_234, 4)).expect("valid available balance"),
                Money::new(Decimal::new(20_000, 4)).expect("valid held balance"),
                AccountStatus::Locked,
                HashMap::new(),
            ),
            AccountRow {
                client: 7,
                available: "-0.1234".into(),
                held: "2.0000".into(),
                total: "1.8766".into(),
                locked: true,
            },
        )]
        fn maps_account_balances_and_status_to_a_row(
            #[case] account: Account,
            #[case] expected: AccountRow,
        ) {
            assert_eq!(AccountRow::from_account(&account), expected);
        }
    }

    mod write_accounts {
        use std::io::{self, Write};

        use rstest::rstest;

        use crate::csv_io::{AccountRow, write_accounts};

        struct FlushFails;

        impl Write for FlushFails {
            fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
                Ok(buffer.len())
            }

            fn flush(&mut self) -> io::Result<()> {
                Err(io::Error::other("flush failed"))
            }
        }

        #[rstest]
        #[case::no_accounts(vec![], b"client,available,held,total,locked\n")]
        #[case::accounts_in_input_order(
            vec![
                AccountRow {
                    client: 9,
                    available: "-0.1234".into(),
                    held: "2.0000".into(),
                    total: "1.8766".into(),
                    locked: false,
                },
                AccountRow {
                    client: 1,
                    available: "3.1".into(),
                    held: "0".into(),
                    total: "3.1".into(),
                    locked: true,
                },
            ],
            b"client,available,held,total,locked\n9,-0.1234,2.0000,1.8766,false\n1,3.1,0,3.1,true\n",
        )]
        fn writes_accounts_in_input_order_with_a_single_header(
            #[case] rows: Vec<AccountRow>,
            #[case] expected: &[u8],
        ) {
            let mut output = Vec::new();
            write_accounts(&mut output, rows).expect("accounts should serialize");

            assert_eq!(output, expected);
        }

        #[test]
        fn propagates_flush_errors() {
            let error = write_accounts(FlushFails, []).expect_err("flush error should be returned");

            assert!(matches!(error.kind(), csv::ErrorKind::Io(_)));
        }
    }

    mod read_transactions {
        use std::io::{self, Cursor, Read};

        use rstest::rstest;

        use crate::csv_io::{TransactionRow, read_transactions};

        struct FailingReader;

        impl Read for FailingReader {
            fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::other("input read failed"))
            }
        }

        #[test]
        fn reads_mixed_transactions_in_input_order() {
            let input = "\
type,client,tx,amount
deposit,2,9,12.3400
withdrawal,1,3,0.5
dispute,2,9,
resolve,2,9,
chargeback,2,9,
";

            let rows = read_transactions(input.as_bytes())
                .expect("headers should be readable")
                .collect::<Result<Vec<_>, _>>()
                .expect("rows should deserialize");

            assert_eq!(
                rows,
                [
                    TransactionRow {
                        r#type: "deposit".into(),
                        client: 2,
                        tx: 9,
                        amount: Some("12.3400".into()),
                    },
                    TransactionRow {
                        r#type: "withdrawal".into(),
                        client: 1,
                        tx: 3,
                        amount: Some("0.5".into()),
                    },
                    TransactionRow {
                        r#type: "dispute".into(),
                        client: 2,
                        tx: 9,
                        amount: None,
                    },
                    TransactionRow {
                        r#type: "resolve".into(),
                        client: 2,
                        tx: 9,
                        amount: None,
                    },
                    TransactionRow {
                        r#type: "chargeback".into(),
                        client: 2,
                        tx: 9,
                        amount: None,
                    },
                ]
            );
        }

        #[test]
        fn trims_headers_and_fields() {
            let input = " type , client , tx , amount \n deposit , 7 , 42 , 1.2300 \n";

            let rows = read_transactions(input.as_bytes())
                .expect("headers should be readable")
                .collect::<Result<Vec<_>, _>>()
                .expect("rows should deserialize");

            assert_eq!(
                rows,
                [TransactionRow {
                    r#type: "deposit".into(),
                    client: 7,
                    tx: 42,
                    amount: Some("1.2300".into()),
                }]
            );
        }

        #[rstest]
        #[case::empty("", None)]
        #[case::whitespace("   ", None)]
        #[case::trailing_zeroes("1.2300", Some("1.2300"))]
        fn preserves_amount_text_and_treats_blank_amounts_as_none(
            #[case] amount: &str,
            #[case] expected: Option<&str>,
        ) {
            let input = format!("type,client,tx,amount\ndeposit,1,2,{amount}\n");
            let row = read_transactions(input.as_bytes())
                .expect("headers should be readable")
                .next()
                .expect("input should contain a row")
                .expect("row should deserialize");

            assert_eq!(row.amount.as_deref(), expected);
        }

        #[test]
        fn propagates_header_read_errors() {
            let error = read_transactions(FailingReader)
                .err()
                .expect("header read failure should be returned before iteration");

            assert!(matches!(error.kind(), csv::ErrorKind::Io(_)));
        }

        #[test]
        fn yields_the_first_row_without_reading_the_entire_input() {
            let input = format!(
                "type,client,tx,amount\n{}",
                "deposit,1,2,1\n".repeat(100_000)
            );
            let input_len =
                u64::try_from(input.len()).expect("test input length should fit in u64");
            let mut source = Cursor::new(input);

            {
                let mut rows = read_transactions(&mut source).expect("headers should be readable");
                let row = rows
                    .next()
                    .expect("input should contain a row")
                    .expect("row should deserialize");
                assert_eq!(row.r#type, "deposit");
                assert_eq!(row.tx, 2);
            }

            assert!(source.position() < input_len);
        }
    }

    mod into_command {
        use rstest::rstest;
        use rust_decimal::Decimal;

        use crate::{
            csv_io::{CsvError, TransactionRow, read_transactions},
            domain::{
                transaction::{TransactionAmount, TransactionId},
                types::{ClientId, DomainError, PaymentCommand},
            },
        };

        fn transaction_row(kind: &str, amount: Option<&str>) -> TransactionRow {
            TransactionRow {
                r#type: kind.into(),
                client: 1,
                tx: 2,
                amount: amount.map(str::to_owned),
            }
        }

        #[rstest]
        #[case::deposit(
            " deposit , 65535 , 4294967295 , 12.3400",
            PaymentCommand::Deposit {
                client_id: ClientId(u16::MAX),
                transaction_id: TransactionId(u32::MAX),
                amount: TransactionAmount::new(Decimal::new(123_400, 4))
                    .expect("valid deposit amount"),
            },
        )]
        #[case::withdrawal(
            "withdrawal,1,3,0.5",
            PaymentCommand::Withdrawal {
                client_id: ClientId(1),
                transaction_id: TransactionId(3),
                amount: TransactionAmount::new(Decimal::new(5, 1))
                    .expect("valid withdrawal amount"),
            },
        )]
        #[case::dispute(
            "dispute,65535,4294967295,",
            PaymentCommand::Dispute {
                client_id: ClientId(u16::MAX),
                transaction_id: TransactionId(u32::MAX),
            },
        )]
        #[case::resolve(
            "resolve,65535,4294967295,",
            PaymentCommand::Resolve {
                client_id: ClientId(u16::MAX),
                transaction_id: TransactionId(u32::MAX),
            },
        )]
        #[case::chargeback(
            "chargeback,65535,4294967295,",
            PaymentCommand::Chargeback {
                client_id: ClientId(u16::MAX),
                transaction_id: TransactionId(u32::MAX),
            },
        )]
        fn converts_csv_rows_to_payment_commands(
            #[case] row: &str,
            #[case] expected: PaymentCommand,
        ) {
            let input = format!("type,client,tx,amount\n{row}\n");
            let command = read_transactions(input.as_bytes())
                .expect("headers should be readable")
                .next()
                .expect("input should contain a row")
                .expect("row should deserialize")
                .into_command()
                .expect("row should convert to a payment command");

            assert_eq!(command, expected);
        }

        #[rstest]
        #[case::deposit_without_amount("deposit", None, CsvError::MissingAmount)]
        #[case::withdrawal_without_amount("withdrawal", None, CsvError::MissingAmount)]
        #[case::dispute_with_amount("dispute", Some("1"), CsvError::UnexpectedAmount)]
        #[case::resolve_with_amount("resolve", Some("1"), CsvError::UnexpectedAmount)]
        #[case::chargeback_with_amount("chargeback", Some("1"), CsvError::UnexpectedAmount)]
        fn rejects_amounts_that_do_not_match_the_command(
            #[case] kind: &str,
            #[case] amount: Option<&str>,
            #[case] expected: CsvError,
        ) {
            assert_eq!(transaction_row(kind, amount).into_command(), Err(expected));
        }

        #[test]
        fn rejects_unknown_transaction_types() {
            assert_eq!(
                transaction_row("transfer", Some("1")).into_command(),
                Err(CsvError::UnknownTransactionType("transfer".into()))
            );
        }

        #[rstest]
        #[case::non_numeric("abc")]
        #[case::unrepresentable_without_rounding("12345678901234567890123456789.1234")]
        fn rejects_invalid_decimal_text_without_rounding(#[case] amount: &str) {
            assert!(matches!(
                transaction_row("deposit", Some(amount)).into_command(),
                Err(CsvError::InvalidDecimal(_))
            ));
        }

        #[test]
        fn propagates_domain_validation_errors() {
            assert_eq!(
                transaction_row("withdrawal", Some("0.12345")).into_command(),
                Err(CsvError::InvalidAmount(DomainError::ExcessivePrecision))
            );
        }
    }
}
