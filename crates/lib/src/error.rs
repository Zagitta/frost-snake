use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Parser error: {0}")]
    ParserError(#[from] crate::parser::ParserError),
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),
}
