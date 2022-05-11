mod client;
mod error;
mod ledger;
mod parser;
mod transaction;

/// Fixed precision integer currency type with 4 decimals of precision
// fixed crate says `Δ = 1/2^f` solving 0.0001 = 1/2^x gives 13.2877 bits required for
// 4 decimals of precision. Unfortunately we can't split bits so round it up to 14 :^)
// The total width is 64 bits rather than 32 because using 14 of 32 gives us only 18 bits
// to represent the integral part meaning 2^18 = 262144 values. Further using 1 bit for
// the sign only gives us a range of -131071 to 131072 which is a very small amount.
// With 64 bits we get a range of -562_949_953_421_311 to 562_949_953_421_312 which should be
// enough for a while.
pub type ICurrency = fixed::FixedI64<fixed::types::extra::U14>;
/// Fixed precision **unsigned** integer currency type with 4 decimals of precision
pub type UCurrency = fixed::FixedU64<fixed::types::extra::U14>;

pub type Result<T> = std::result::Result<T, error::Error>;

pub use parser::parse_from_reader;
