use eyre::Result;
use frost_snake_lib::execute;
use std::{env, fs::File, io::BufReader};

fn main() -> Result<()> {
    let file_name = env::args()
        .nth(1)
        .ok_or(eyre::eyre!("Missing argument\nUsage: file_name.csv"))?;

    execute(BufReader::new(File::open(file_name)?), std::io::stdout())?;

    Ok(())
}
