use std::{env, fs::File, io, path::Path, process::ExitCode};

use tx_processor::{
    csv_io::{AccountRow, read_transactions, write_accounts},
    domain::payment_processor::PaymentProcessor,
};

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
        .and_then(|commands| {
            let mut processor = PaymentProcessor::new();
            for command in commands {
                let command =
                    command.map_err(|error| format!("Could not read transaction: {error}"))?;
                processor
                    .process(command)
                    .map_err(|error| format!("Could not process transaction: {error:?}"))?;
            }

            let accounts = processor
                .accounts()
                .into_iter()
                .map(AccountRow::from_account);
            write_accounts(io::stdout().lock(), accounts)
                .map_err(|error| format!("Could not write accounts: {error}"))
        });

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
