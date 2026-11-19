use std::{env, fs::File, io, path::Path, process::ExitCode};

use tx_processor::{
    csv_io::{AccountRow, TransactionRow, read_transactions, write_accounts},
    domain::payment_processor::PaymentProcessor,
};

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    let (Some(input), None) = (args.next(), args.next()) else {
        eprintln!("Usage: tx-processor <transactions.csv>");
        return ExitCode::FAILURE;
    };

    match run(Path::new(&input)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run(input: &Path) -> Result<(), String> {
    let file = File::open(input)
        .map_err(|error| format!("Could not open '{}': {error}", input.display()))?;
    let rows = read_transactions(file)
        .map_err(|error| format!("Could not read '{}': {error}", input.display()))?;
    process_transactions(rows, io::stdout().lock())
}

fn process_transactions(
    rows: impl Iterator<Item = Result<TransactionRow, csv::Error>>,
    output: impl io::Write,
) -> Result<(), String> {
    let mut processor = PaymentProcessor::new();

    for row in rows {
        let row = match row {
            Ok(row) => row,
            Err(error) => {
                let message = format!("Could not read transaction: {error}");
                if error.is_io_error() {
                    return Err(message);
                }
                eprintln!("{message}");
                continue;
            }
        };

        let command = match row.into_command() {
            Ok(command) => command,
            Err(error) => {
                eprintln!("Invalid transaction: {error:?}");
                continue;
            }
        };

        processor
            .process(command)
            .map_err(|error| format!("Could not process transaction: {error:?}"))?;
    }

    let accounts = processor
        .accounts()
        .into_iter()
        .map(AccountRow::from_account);
    write_accounts(output, accounts).map_err(|error| format!("Could not write accounts: {error}"))
}

#[cfg(test)]
mod tests {
    use std::io::{self, Read};

    use super::{process_transactions, read_transactions};

    struct FailingReader;

    impl Read for FailingReader {
        fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("input read failed"))
        }
    }

    #[test]
    fn fails_without_writing_accounts_after_input_read_error() {
        let input = b"type,client,tx,amount\ndeposit,1,1,1\n";
        let rows = read_transactions(input.as_slice().chain(FailingReader))
            .expect("headers should be readable");
        let mut output = Vec::new();

        let result = process_transactions(rows, &mut output);

        assert_eq!(output, b"");
        assert_eq!(
            result,
            Err("Could not read transaction: input read failed".into())
        );
    }
}
