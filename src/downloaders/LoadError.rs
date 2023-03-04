use std::error::Error;
use std::fmt::{Display, Formatter};

pub type LoadResult<T> = Result<T, LoadError>;

#[derive(Debug)]
pub enum LoadError {
    Rejected(String),
    Error(Box<dyn Error + Send + Sync>),
}

impl Display for LoadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Rejected(reason) => write!(f, "{reason}"),
            LoadError::Error(e) => write!(f, "{e}"),
        }
    }
}

impl From<String> for LoadError {
    fn from(value: String) -> Self {
        LoadError::Rejected(value)
    }
}

impl From<&str> for LoadError {
    fn from(value: &str) -> Self {
        value.to_string().into()
    }
}

impl From<std::io::Error> for LoadError {
    fn from(value: std::io::Error) -> Self {
        LoadError::Error(Box::new(value))
    }
}

impl From<reqwest::Error> for LoadError {
    fn from(value: reqwest::Error) -> Self {
        LoadError::Error(Box::new(value))
    }
}
