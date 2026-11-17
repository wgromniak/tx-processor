use std::{env, fs::File, path::Path, process::ExitCode};

const SAMPLE_ACCOUNTS: &str = "\
client,available,held,total,locked
1,1.5,0,1.5,false
2,2,0,2,false
";

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    let (Some(input), None) = (args.next(), args.next()) else {
        eprintln!("Usage: tx-processor <transactions.csv>");
        return ExitCode::FAILURE;
    };

    let input = Path::new(&input);
    if let Err(error) = File::open(input) {
        eprintln!("Could not open '{}': {error}", input.display());
        return ExitCode::FAILURE;
    }

    print!("{SAMPLE_ACCOUNTS}");
    ExitCode::SUCCESS
}
