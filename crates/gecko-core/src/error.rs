use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("parse error in {what}: {msg}")]
    Parse { what: String, msg: String },
    #[error("invalid {what}: {msg}")]
    Invalid { what: String, msg: String },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl Error {
    pub fn invalid(what: impl Into<String>, msg: impl Into<String>) -> Self {
        Error::Invalid { what: what.into(), msg: msg.into() }
    }
    pub fn parse(what: impl Into<String>, msg: impl std::fmt::Display) -> Self {
        Error::Parse { what: what.into(), msg: msg.to_string() }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
