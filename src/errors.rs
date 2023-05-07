use std::fmt;

#[derive(Debug)]
pub enum Error {
    StreamDoesNotExistError(String),

    WriteError(String),
    ReadError(String),

    WaitError(String),

    NotAcceptingError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::StreamDoesNotExistError(msg) => write!(f, "StreamDoesNotExistError: {}", msg),
            Error::WriteError(msg) => write!(f, "WriteError: {}", msg),
            Error::ReadError(msg) => write!(f, "ReadError: {}", msg),
            Error::WaitError(msg) => write!(f, "WaitError: {}", msg),
            Error::NotAcceptingError(msg) => write!(f, "NotAcceptingError: {}", msg),
        }
    }
}
