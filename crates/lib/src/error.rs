use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Parser error")]
    ParserError(#[from] crate::parser::ParserError),
}
