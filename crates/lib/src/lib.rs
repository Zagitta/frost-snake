mod client;
mod error;
mod ledger;
mod parser;
mod transaction;

type FRAC = fixed::types::extra::U14;

/// Fixed point integer currency type with 4 decimals of precision
// fixed crate says `Δ = 1/2^f` solving 0.0001 = 1/2^x gives 13.2877 bits required for
// 4 decimals of precision. Unfortunately we can't split bits so round it up to 14 :^)
// The total width is 64 bits rather than 32 because using 14 of 32 gives us only 18 bits
// to represent the integral part meaning 2^18 = 262144 values.
// Using 64 bit instead we get 2^(64-14) = 2^50 = 1_125_899_906_842_624  for UCurrency
// and -562_949_953_421_311 to 562_949_953_421_312 for ICurrency which should be enough for a while.
pub type ICurrency = fixed::FixedI64<FRAC>;
/// Fixed precision **unsigned** integer currency type with 4 decimals of precision
pub type UCurrency = fixed::FixedU64<FRAC>;

pub type Result<T> = std::result::Result<T, error::Error>;

pub use client::*;
pub use ledger::*;
pub use parser::parse_from_reader;
pub use transaction::*;
