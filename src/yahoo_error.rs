use thiserror::Error;

use crate::quotes::YErrorMessage;

#[derive(Error, Debug)]
pub enum YahooError {
    #[error("fetching the data from yahoo! finance failed: {0}")]
    FetchFailed(String),
    #[error("deserializing response from yahoo! finance failed: {0}")]
    DeserializeFailed(#[from] serde_json::Error),
    #[error("connection to yahoo! finance server failed: {0}")]
    ConnectionFailed(#[from] reqwest::Error),
    #[error("yahoo! finance returned api error: {0:?}")]
    ApiError(YErrorMessage),
    #[error("yahoo! finance returned an empty data set")]
    NoResult,
    #[error("yahoo! finance returned an empty data set")]
    NoQuotes,
    #[error("yahoo! finance returned inconsistent data")]
    DataInconsistency,
    #[error("constructing yahoo! finance client failed")]
    BuilderFailed,
    #[error("No cookies in response headers")]
    NoCookies,
    #[error("Invisible characters in cookies")]
    InvisibleAsciiInCookies,
    #[error("No response")]
    NoResponse,
    #[error("Invalid cookie")]
    InvalidCookie,
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Invalid crumb")]
    InvalidCrumb,
}
