use eyre::{Context, Result};
use frost_snake_lib::{parse_csv, Ledger, TransactionExecutor};
use std::{env, fs::File, io::BufReader};

fn main() -> Result<()> {
    let file_name = env::args()
        .nth(1)
        .ok_or(eyre::eyre!("Missing argument\nUsage: file_name.csv"))?;
    let file = File::open(&file_name).with_context(|| format!("Couldn't find file {file_name}"))?;
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
