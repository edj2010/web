use std::fmt::Display;
use std::io;
use std::str::Utf8Error;

#[derive(Debug)]
pub enum WebServerError {
    Utf8Error(Utf8Error),
    IOError(io::Error),
    Other(String),
}

impl WebServerError {
    pub fn other(e: &str) -> Self {
        WebServerError::Other(String::from(e))
    }
}

impl From<Utf8Error> for WebServerError {
    fn from(e: Utf8Error) -> Self {
        WebServerError::Utf8Error(e)
    }
}

impl From<io::Error> for WebServerError {
    fn from(e: io::Error) -> Self {
        WebServerError::IOError(e)
    }
}

impl Display for WebServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            Self::Utf8Error(e) => e.to_string(),
            Self::IOError(e) => e.to_string(),
            Self::Other(e) => e.to_string(),
        };
        write!(f, "{}", string)
    }
}

pub type Result<T> = std::result::Result<T, WebServerError>;
