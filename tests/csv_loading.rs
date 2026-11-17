use std::{
    error::Error,
    io::{self, Cursor, Read},
};

use tx_processor::csv_io::{TransactionRow, read_transactions};

#[test]
fn reads_mixed_transactions_in_input_order() -> Result<(), csv::Error> {
    let input = "\
type,client,tx,amount
deposit,2,9,12.3400
withdrawal,1,3,0.5
dispute,2,9,
resolve,2,9,
chargeback,2,9,
";

    let rows = read_transactions(input.as_bytes()).collect::<Result<Vec<_>, _>>()?;

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
    Ok(())
}

#[test]
fn trims_fields_and_maps_reordered_headers() -> Result<(), csv::Error> {
    let input = " amount , tx , type , client \r\n 1.2300 , 42 , deposit , 7 \r\n";

    let rows = read_transactions(input.as_bytes()).collect::<Result<Vec<_>, _>>()?;

    assert_eq!(
        rows,
        [TransactionRow {
            r#type: "deposit".into(),
            client: 7,
            tx: 42,
            amount: Some("1.2300".into()),
        }]
    );
    Ok(())
}

#[test]
fn preserves_amount_text_and_treats_blank_amounts_as_none() -> Result<(), Box<dyn Error>> {
    for (amount, expected) in [
        ("", None),
        ("   ", None),
        ("\"\"", None),
        ("0", Some("0")),
        ("1.0", Some("1.0")),
        ("1.23", Some("1.23")),
        ("1.234", Some("1.234")),
        ("1.2300", Some("1.2300")),
        ("\"1.2300\"", Some("1.2300")),
    ] {
        let input = format!("type,client,tx,amount\ndeposit,1,2,{amount}\n");
        let row = read_transactions(input.as_bytes())
            .next()
            .ok_or("expected a row")??;

        assert_eq!(row.amount.as_deref(), expected, "input amount: {amount}");
    }
    Ok(())
}

#[test]
fn leaves_transaction_type_and_amount_validation_to_the_domain() -> Result<(), Box<dyn Error>> {
    let input = "type,client,tx,amount\nunknown,1,2,not-a-number\n";
    let row = read_transactions(input.as_bytes())
        .next()
        .ok_or("expected a row")??;

    assert_eq!(row.r#type, "unknown");
    assert_eq!(row.amount.as_deref(), Some("not-a-number"));
    Ok(())
}

#[test]
fn yields_no_rows_for_empty_or_header_only_input() {
    for input in ["", "type,client,tx,amount\n"] {
        assert!(read_transactions(input.as_bytes()).next().is_none());
    }
}

#[test]
fn accepts_identifier_boundaries() -> Result<(), Box<dyn Error>> {
    for (client, tx) in [(0, 0), (u16::MAX, u32::MAX)] {
        let input = format!("type,client,tx,amount\ndeposit,{client},{tx},1\n");
        let row = read_transactions(input.as_bytes())
            .next()
            .ok_or("expected a row")??;

        assert_eq!(row.client, client);
        assert_eq!(row.tx, tx);
    }
    Ok(())
}

#[test]
fn reports_invalid_identifiers_with_row_positions() -> Result<(), Box<dyn Error>> {
    for (client, tx) in [
        ("abc", "2"),
        ("-1", "2"),
        ("65536", "2"),
        ("1", "abc"),
        ("1", "-1"),
        ("1", "4294967296"),
    ] {
        let input = format!("type,client,tx,amount\ndeposit,{client},{tx},1\n");
        let row = read_transactions(input.as_bytes())
            .next()
            .ok_or("expected an error")?;
        let Err(error) = row else {
            panic!("accepted invalid identifiers: {client}, {tx}");
        };

        assert!(matches!(error.kind(), csv::ErrorKind::Deserialize { .. }));
        assert_eq!(error.position().map(csv::Position::line), Some(2));
    }
    Ok(())
}

#[test]
fn reports_inconsistent_field_counts_after_valid_rows() -> Result<(), Box<dyn Error>> {
    for invalid_row in ["dispute,1,2", "deposit,1,3,1,extra"] {
        let input = format!("type,client,tx,amount\ndeposit,1,2,1\n{invalid_row}\n");
        let mut rows = read_transactions(input.as_bytes());
        rows.next().ok_or("expected a valid row")??;
        let Err(error) = rows.next().ok_or("expected an error")? else {
            panic!("accepted inconsistent row: {invalid_row}");
        };

        assert!(matches!(
            error.kind(),
            csv::ErrorKind::UnequalLengths {
                expected_len: 4,
                ..
            }
        ));
        assert_eq!(error.position().map(csv::Position::line), Some(3));
    }
    Ok(())
}

struct FailingReader;

impl Read for FailingReader {
    fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("input read failed"))
    }
}

#[test]
fn propagates_header_read_errors() -> Result<(), Box<dyn Error>> {
    let mut rows = read_transactions(FailingReader);
    let Err(error) = rows.next().ok_or("expected a read error")? else {
        panic!("expected a read error");
    };

    assert!(matches!(error.kind(), csv::ErrorKind::Io(_)));
    assert_eq!(error.to_string(), "input read failed");
    assert!(rows.next().is_none());
    Ok(())
}

#[test]
fn propagates_invalid_utf8_headers() -> Result<(), Box<dyn Error>> {
    let input = b"ty\xffpe,client,tx,amount\ndeposit,1,2,1\n";
    let mut rows = read_transactions(input.as_slice());
    let Err(error) = rows.next().ok_or("expected a header error")? else {
        panic!("expected a header error");
    };

    assert!(matches!(error.kind(), csv::ErrorKind::Utf8 { .. }));
    assert_eq!(error.position().map(csv::Position::line), Some(1));
    assert!(rows.next().is_none());
    Ok(())
}

#[test]
fn propagates_read_errors_after_valid_rows() -> Result<(), Box<dyn Error>> {
    let input = b"type,client,tx,amount\ndeposit,1,2,1\n";
    let mut rows = read_transactions(input.as_slice().chain(FailingReader));
    let row = rows.next().ok_or("expected a valid row")??;
    assert_eq!(row.tx, 2);

    let Err(error) = rows.next().ok_or("expected a read error")? else {
        panic!("expected a read error");
    };
    assert!(matches!(error.kind(), csv::ErrorKind::Io(_)));
    assert_eq!(error.to_string(), "input read failed");
    assert!(rows.next().is_none());
    Ok(())
}

#[test]
fn yields_the_first_row_without_reading_the_entire_input() -> Result<(), Box<dyn Error>> {
    let input = format!(
        "type,client,tx,amount\n{}",
        "deposit,1,2,1\n".repeat(100_000)
    );
    let input_len = u64::try_from(input.len())?;
    let mut source = Cursor::new(input);

    {
        let mut rows = read_transactions(&mut source);
        let row = rows.next().ok_or("expected a valid row")??;
        assert_eq!(row.r#type, "deposit");
        assert_eq!(row.tx, 2);
    }

    assert!(source.position() < input_len);
    Ok(())
}
