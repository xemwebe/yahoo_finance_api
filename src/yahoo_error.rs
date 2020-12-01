use serde::Deserialize;
use std::fmt;

#[derive(Debug, Deserialize)]
pub enum YahooError {
    FetchFailed(String),
    DeserializeFailed(String),
    ConnectionFailed,
    InvalidJson,
    EmptyDataSet,
    DataInconsistency,
}

impl std::error::Error for YahooError {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        None
    }
}

impl fmt::Display for YahooError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FetchFailed(status) => write!(
                f,
                "fetching the data from yahoo! finance failed: with status code {}",
                status
            ),
            Self::DeserializeFailed(s) => write!(
                f,
                "deserializing response from yahoo! finance failed: {}",
                &s
            ),
            Self::ConnectionFailed => write!(f, "connection to yahoo! finance server failed"),
            Self::InvalidJson => write!(f, "yahoo! finance return invalid JSON format"),
            Self::EmptyDataSet => write!(f, "yahoo! finance returned an empty data set"),
            Self::DataInconsistency => write!(f, "yahoo! finance returned inconsistent data"),
        }
    }
}
