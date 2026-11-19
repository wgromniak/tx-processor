use std::{env, fs::File, io, path::Path, process::ExitCode};

use tx_processor::csv_io::{process_transactions, read_transactions};

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    let (Some(input), None) = (args.next(), args.next()) else {
        eprintln!("Usage: tx-processor <transactions.csv>");
        return ExitCode::FAILURE;
    };

    let input = Path::new(&input);
    let result = File::open(input)
        .map_err(|error| format!("Could not open '{}': {error}", input.display()))
        .and_then(|file| {
            read_transactions(file)
                .map_err(|error| format!("Could not read '{}': {error}", input.display()))
        })
        .and_then(|rows| process_transactions(rows, io::stdout().lock()));

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
