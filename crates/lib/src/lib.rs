mod client;
mod error;
mod ledger;
mod parser;
mod transaction;
mod writer;

type Frac = fixed::types::extra::U16;

/// Fixed point integer currency type with 4 decimals of precision
// fixed crate says `Δ = 1/2^f` solving 0.0001 = 1/2^x gives 13.2877 bits required for
// 4 decimals of precision.
// Unfortunately simply rounding that up to 14 isn't enough because that would give
// `Δ = 0.00006` which doesn't cleanly divide 0.0001.
// It isn't until 16 bits of precision we reach `Δ = 0.00002` which cleanly divides 0.0001
// The total width is 64 bits rather than 32 because using 16 of 32 gives us only 16 bits
// to represent the integral part meaning 2^16 = 65536 values.
// Using 64 bit instead we get 2^(64-16) = 2^48 = 281_474_976_710_656 for UCurrency
// and -140_737_488_355_328 to 140_737_488_355_327.9999 for ICurrency which should be enough for a while.
pub type ICurrency = fixed::FixedI64<Frac>;
/// Fixed precision **unsigned** integer currency type with 4 decimals of precision
pub type UCurrency = fixed::FixedU64<Frac>;

pub use fixed_macro::types::I48F16 as icur;
pub use fixed_macro::types::U48F16 as ucur;

pub use client::*;
pub use ledger::*;
pub use parser::{parse_csv, parse_from_reader};
pub use transaction::*;
pub use writer::write_csv;

pub fn execute<R: std::io::Read, W: std::io::Write>(
    reader: R,
    writer: W,
) -> Result<(), error::Error> {
    let transactions = parse_csv(reader)?;
    let mut ledger = Ledger::default();
    for transaction in transactions.flatten() {
        ledger.execute(transaction).ok();
    }

    Ok(write_csv(&ledger, writer)?)
}

#[cfg(test)]
mod tests {

    use fixed_macro::types::U48F16 as ucur;

    use crate::UCurrency;
    #[test]
    fn test_currency() {
        let my_delta = ucur!(0.0001);
        let min_delta = UCurrency::DELTA;
        println!("{min_delta}");

        assert_eq!((min_delta / my_delta) * my_delta, ucur!(0));
        assert_eq!((ucur!(1) + min_delta) / my_delta * my_delta, ucur!(1.0))
    }
}
