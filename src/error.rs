use std::{num::ParseIntError, panic::Location};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    message: String,
    location: &'static Location<'static>,
}

impl Error {
    #[allow(dead_code)]
    #[track_caller]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            location: Location::caller(),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} [location={}]", self.message, self.location)
    }
}

impl std::error::Error for Error {}

impl From<Box<dyn std::error::Error>> for Error {
    #[track_caller]
    fn from(error: Box<dyn std::error::Error>) -> Self {
        Self {
            message: format!("dyn error: {error:?}"),
            location: Location::caller(),
        }
    }
}

impl From<std::io::Error> for Error {
    #[track_caller]
    fn from(error: std::io::Error) -> Self {
        Self {
            message: format!("io error: {error:?}"),
            location: Location::caller(),
        }
    }
}

impl From<String> for Error {
    #[track_caller]
    fn from(error: String) -> Self {
        Self {
            message: format!("error: {error}"),
            location: Location::caller(),
        }
    }
}

impl From<&str> for Error {
    #[track_caller]
    fn from(error: &str) -> Self {
        Self {
            message: format!("error: {error}"),
            location: Location::caller(),
        }
    }
}

impl From<ParseIntError> for Error {
    #[track_caller]
    fn from(error: ParseIntError) -> Self {
        Self {
            message: format!("parse int error: {error:?}"),
            location: Location::caller(),
        }
    }
}
