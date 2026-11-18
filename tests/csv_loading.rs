use std::io::{self, Cursor, Read};

use tx_processor::csv_io::{TransactionRow, read_transactions};

#[test]
fn trims_fields_and_maps_reordered_headers() {
    let input = " amount , tx , type , client \r\n 1.2300 , 42 , deposit , 7 \r\n";

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
