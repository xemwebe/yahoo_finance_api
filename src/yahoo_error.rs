use thiserror::Error;

use crate::quotes::YErrorMessage;

/// Errors returned by the yahoo! finance connector.
#[derive(Error, Debug)]
pub enum YahooError {
    /// The request itself or its setup failed (e.g. empty ticker).
    #[error("fetching the data from yahoo! finance failed: {0}")]
    FetchFailed(String),
    /// The request itself or its setup failed (e.g. empty ticker).
    #[error("yahoo! finance returned a server error (5xx): {0}")]
    ServerError(String),
    /// The response could not be deserialized into the expected type.
    #[error("deserializing response from yahoo! finance failed: {0}")]
    DeserializeFailed(#[from] serde_json::Error),

    /// The response could not be deserialized; the full response body
    /// (truncated) is attached. Only compiled with the `debug` feature.
    #[error("deserializing response from yahoo! finance failed, full response body: {0}")]
    DeserializeFailedDebug(String),

    /// yahoo! finance returned an empty response body.
    #[error("yahoo! finance returned an empty response body")]
    EmptyResponse,

    /// yahoo! finance returned a non-JSON (HTML) response body.
    #[error("yahoo! finance returned a non-JSON (HTML) response body")]
    HtmlResponse,

    /// A request to yahoo! finance failed (network, TLS, timeout, ...).
    #[error("connection to yahoo! finance server failed: {0}")]
    ConnectionFailed(#[from] reqwest::Error),
    /// yahoo! finance returned an API-level error.
    #[error("yahoo! finance returned api error: {0:?}")]
    ApiError(YErrorMessage),
    /// The response contained no result data.
    #[error("yahoo! finance returned an empty result set")]
    NoResult,
    /// The response contained no valid quotes.
    #[error("yahoo! finance returned no quotes")]
    NoQuotes,
    /// The response data was structurally inconsistent (e.g. mismatched array lengths).
    #[error("yahoo! finance returned inconsistent data")]
    DataInconsistency,
    /// Building the HTTP client failed (reserved, currently not constructed).
    #[error("constructing yahoo! finance client failed")]
    BuilderFailed,
    /// No cookies were found in the response headers.
    #[error("No cookies in response headers")]
    NoCookies,
    /// The cookie value contained invisible characters.
    #[error("Invisible characters in cookies")]
    InvisibleAsciiInCookies,
    /// The request did not return a response.
    #[error("No response")]
    NoResponse,
    /// The cookie used for authentication was invalid.
    #[error("Invalid cookie")]
    InvalidCookie,
    /// The request was rejected as unauthorized.
    #[error("Unauthorized")]
    Unauthorized,
    /// The crumb used for authentication was invalid.
    #[error("Invalid crumb")]
    InvalidCrumb,
    /// yahoo! finance rate-limited the request.
    #[error("Too many requests (rate limited by Yahoo) during: {0}")]
    TooManyRequests(String),

    /// The URL could not be parsed.
    #[error("Invalid URL format")]
    InvalidUrl,

    /// A date could not be parsed.
    #[error("Invalid date format")]
    InvalidDateFormat,

    /// A required field was missing in the response.
    #[error("Missing required field: {0}")]
    MissingField(String),
}
