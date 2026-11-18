use std::io::{self, Cursor, Read};

use rust_decimal::Decimal;
use tx_processor::{
    csv_io::{CsvError, TransactionRow, read_transactions},
    domain_types::{ClientId, DomainError, PaymentCommand, TransactionAmount, TransactionId},
};

#[test]
fn converts_csv_rows_to_payment_commands() {
    let input = "\
type,client,tx,amount
 deposit , 65535 , 4294967295 , 12.3400
withdrawal,1,3,0.5
dispute,65535,4294967295,
resolve,65535,4294967295,
chargeback,65535,4294967295,
";
    let commands = read_transactions(input.as_bytes())
        .expect("headers should be readable")
        .map(|row| row.expect("row should deserialize").into_command())
        .collect::<Result<Vec<_>, _>>()
        .expect("rows should convert to payment commands");

    let client_id = ClientId(u16::MAX);
    let transaction_id = TransactionId(u32::MAX);
    assert_eq!(
        commands,
        [
            PaymentCommand::Deposit {
                client_id,
                transaction_id,
                amount: TransactionAmount::new(Decimal::new(123_400, 4))
                    .expect("valid deposit amount"),
            },
            PaymentCommand::Withdrawal {
                client_id: ClientId(1),
                transaction_id: TransactionId(3),
                amount: TransactionAmount::new(Decimal::new(5, 1))
                    .expect("valid withdrawal amount"),
            },
            PaymentCommand::Dispute {
                client_id,
                transaction_id,
            },
            PaymentCommand::Resolve {
                client_id,
                transaction_id,
            },
            PaymentCommand::Chargeback {
                client_id,
                transaction_id,
            },
        ]
    );
}

#[test]
fn rejects_amounts_that_do_not_match_the_command() {
    for kind in ["deposit", "withdrawal"] {
        assert_eq!(
            transaction_row(kind, None).into_command(),
            Err(CsvError::MissingAmount)
        );
    }
    for kind in ["dispute", "resolve", "chargeback"] {
        assert_eq!(
            transaction_row(kind, Some("1")).into_command(),
            Err(CsvError::UnexpectedAmount)
        );
    }
}

#[test]
fn rejects_unknown_transaction_types() {
    assert_eq!(
        transaction_row("transfer", Some("1")).into_command(),
        Err(CsvError::UnknownTransactionType("transfer".into()))
    );
}

#[test]
fn rejects_invalid_decimal_text_without_rounding() {
    for amount in ["abc", "12345678901234567890123456789.1234"] {
        assert!(matches!(
            transaction_row("deposit", Some(amount)).into_command(),
            Err(CsvError::InvalidDecimal(_))
        ));
    }
}

#[test]
fn propagates_domain_validation_errors() {
    assert_eq!(
        transaction_row("withdrawal", Some("0.12345")).into_command(),
        Err(CsvError::InvalidAmount(DomainError::ExcessivePrecision))
    );
}

fn transaction_row(kind: &str, amount: Option<&str>) -> TransactionRow {
    TransactionRow {
        r#type: kind.into(),
        client: 1,
        tx: 2,
        amount: amount.map(str::to_owned),
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

#[test]
fn preserves_amount_text_and_treats_blank_amounts_as_none() {
    for (amount, expected) in [("", None), ("   ", None), ("1.2300", Some("1.2300"))] {
        let input = format!("type,client,tx,amount\ndeposit,1,2,{amount}\n");
        let row = read_transactions(input.as_bytes())
            .expect("headers should be readable")
            .next()
            .expect("input should contain a row")
            .expect("row should deserialize");

        assert_eq!(row.amount.as_deref(), expected, "input amount: {amount}");
    }
}

struct FailingReader;

impl Read for FailingReader {
    fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("input read failed"))
    }
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
    let input_len = u64::try_from(input.len()).expect("test input length should fit in u64");
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
