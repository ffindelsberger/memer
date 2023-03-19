use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::StripPrefixError;

pub type LoadResult<T> = Result<T, LoadError>;

/// Ignore : Thanks to reddit´s stupid api design they store the post url in the same attribute as the images so have this variant
///         to signal that we do not want to send the file to that was created to discord
/// Rejected: We have a valid image or video but it cant be sent do discord foe i.E. filesize restrictions
#[derive(Debug)]
pub enum LoadError {
    Ignore(String),
    Rejected(String),
    Error(Box<dyn Error + Send + Sync>),
}

impl Display for LoadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Ignore(reason) => write!(f, "{reason}"),
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

impl From<StripPrefixError> for LoadError {
    fn from(value: StripPrefixError) -> Self {
        LoadError::Error(Box::new(value))
    }
}
