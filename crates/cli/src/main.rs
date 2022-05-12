use eyre::Result;
use frost_snake_lib::{
    parse_csv, parse_from_reader, Ledger, TransactionExecutionError, TransactionExecutor,
};
use std::{env, fs::File, io::BufReader, process::exit};

fn main() -> Result<()> {
    let file_name = env::args()
        .nth(1)
        .ok_or(eyre::eyre!("Missing argument\nUsage: file_name.csv"))?;
    let file = File::open(file_name)?;
    let reader = BufReader::new(file);

    let mut ledger = Ledger::default();

    for trans in parse_csv(reader)? {
        let transaction = if let Ok(trans) = trans {
            trans
        } else {
            continue;
        };

        if let Ok(l) = ledger.clone().execute(transaction) {
            ledger = l;
        };
    }

    println!("{:#?}", ledger);

    Ok(())
}

// [0.00004; 0.00009] => 0.00006
