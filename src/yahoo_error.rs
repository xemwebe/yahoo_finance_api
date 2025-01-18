use thiserror::Error;

#[derive(Error, Debug)]
pub enum YahooError {
    #[error("fetching the data from yahoo! finance failed")]
    FetchFailed(String),
    #[error("deserializing response from yahoo! finance failed")]
    DeserializeFailed(#[from] serde_json::Error),
    #[error("connection to yahoo! finance server failed")]
    ConnectionFailed(#[from] reqwest::Error),
    #[error("yahoo! finance return invalid JSON format")]
    InvalidJson,
    #[error("yahoo! finance returned an empty data set")]
    EmptyDataSet,
    #[error("yahoo! finance returned inconsistent data")]
    DataInconsistency,
    #[error("construcing yahoo! finance client failed")]
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
