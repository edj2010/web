use std::{error::Error, fmt::Display, io, str::Utf8Error};

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
        match self {
            Self::Utf8Error(e) => write!(f, "UTF8 Error: {}", e),
            Self::IOError(e) => write!(f, "IO Error: {}", e),
            Self::Other(e) => write!(f, "Error: {}", e),
        }
    }
}

impl Error for WebServerError {}

pub type Result<T> = std::result::Result<T, WebServerError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display() {
        let error = WebServerError::other("test error");
        assert_eq!(error.to_string(), "Error: test error");
    }
}
