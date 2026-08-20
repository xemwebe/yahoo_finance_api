use reqwest::StatusCode;

use crate::quotes::YErrorMessage;
use crate::YahooError;

/// Extract `(code, description)` from an `"error"` field, whether it sits at
/// the top level (`{"error": {"code": ...}}`) or inside the chart payload
/// (`{"chart": {"error": {"code": ...}}}`), or is a plain error string.
fn top_error_code(json: &serde_json::Value) -> Option<(String, Option<String>)> {
    let candidates = [
        json.get("error"),
        json.get("chart").and_then(|c| c.get("error")),
    ];
    for candidate in candidates.into_iter().flatten() {
        if let Some(code) = candidate.get("code").and_then(|c| c.as_str()) {
            return Some((
                code.to_string(),
                candidate
                    .get("description")
                    .and_then(|d| d.as_str())
                    .map(str::to_string),
            ));
        }
        if let Some(msg) = candidate.as_str() {
            return Some((msg.to_string(), None));
        }
    }
    None
}

/// Classify a known error code into the specific `YahooError` variant so
/// callers can react (retry on rate limit, refresh crumb on unauthorized).
fn error_from_code(code: &str, description: Option<&str>, url: &str) -> YahooError {
    match code {
        "Too Many Requests" => YahooError::TooManyRequests(url.to_string()),
        "Unauthorized" => YahooError::Unauthorized,
        "Invalid Crumb" => YahooError::InvalidCrumb,
        _ => YahooError::ApiError(YErrorMessage {
            code: Some(code.to_string()),
            description: description.map(str::to_string),
        }),
    }
}

/// Decode a raw HTTP response body into a JSON value with proper error
/// classification so callers can distinguish retriable (empty body, HTML
/// block page, rate limit) from malformed responses.
pub(crate) fn decode_body(
    text: String,
    status: StatusCode,
    url: &str,
) -> Result<serde_json::Value, YahooError> {
    if status == StatusCode::TOO_MANY_REQUESTS {
        return Err(YahooError::TooManyRequests(url.to_string()));
    }
    if status == StatusCode::UNAUTHORIZED {
        return Err(YahooError::Unauthorized);
    }
    if status == StatusCode::FORBIDDEN {
        // Yahoo surfaces a stale crumb/cookie as a 403 (same as 401). Mapping
        // it to Unauthorized lets the retry guard in get_ticker_info refresh
        // the crumb/cookie and retry, matching get_financial_events and
        // get_crumb which already treat 403 as an expired session.
        return Err(YahooError::Unauthorized);
    }
    if status == StatusCode::NOT_FOUND {
        return Err(YahooError::FetchFailed(format!(
            "404, request url: {}",
            url
        )));
    }
    if !status.is_success() {
        if status.is_server_error() {
            return Err(YahooError::ServerError(format!(
                "{} status, request url: {}",
                status.as_u16(),
                url
            )));
        }
        let body = text.trim();
        if !body.is_empty() && body.starts_with('{') {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                if let Some((code, description)) = top_error_code(&json) {
                    return Err(error_from_code(&code, description.as_deref(), url));
                }
            }
        }
        return Err(YahooError::FetchFailed(format!(
            "{} status, request url: {}",
            status.as_u16(),
            url
        )));
    }

    let body = text.trim();
    if body.is_empty() {
        return Err(YahooError::EmptyResponse);
    }
    if body.starts_with('<') {
        return Err(YahooError::HtmlResponse);
    }
    // Yahoo serves a short maintenance page when the API is temporarily down.
    // yfinance detects this exact phrase (history.py "Will be right back").
    // No length cap: a plain-text outage page longer than 4 KB must still be
    // classified as a maintenance page (HTML is caught by starts_with('<')).
    if body.contains("Will be right back") {
        return Err(YahooError::HtmlResponse);
    }
    // A plain-text rate-limit message is definitive (yfinance raises
    // YFRateLimitError for it). Only match non-JSON bodies: a valid JSON
    // error whose description happens to mention "too many requests" (e.g.
    // `{"chart":{"error":{"code":"InvalidPeriod",...}}}`) must not be
    // misclassified as a rate limit.
    if !body.starts_with('{')
        && !body.starts_with('[')
        && body.to_ascii_lowercase().contains("too many requests")
    {
        return Err(YahooError::TooManyRequests(url.to_string()));
    }

    match serde_json::from_str::<serde_json::Value>(body) {
        Ok(json) => {
            // yfinance treats a `{"status_code": ...}` body (200) as a
            // Yahoo-side error rather than a deserialization problem
            // (history.py: "Yahoo status_code = ...").
            if json.get("status_code").is_some() {
                return Err(YahooError::FetchFailed(format!(
                    "Yahoo returned status_code in body, request url: {}",
                    url
                )));
            }
            if let Some((code, description)) = top_error_code(&json) {
                return Err(error_from_code(&code, description.as_deref(), url));
            }
            Ok(json)
        }
        Err(e) => {
            #[cfg(feature = "debug")]
            {
                Err(YahooError::DeserializeFailedDebug(format!(
                    "{} body: {}",
                    e,
                    truncate(body, 4_000)
                )))
            }
            #[cfg(not(feature = "debug"))]
            {
                Err(YahooError::DeserializeFailed(e))
            }
        }
    }
}

#[cfg(feature = "debug")]
fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let mut preview: String = s.chars().take(max_chars).collect();
        preview.push_str("...[truncated]");
        preview
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(body: &str, status: StatusCode) -> Result<serde_json::Value, YahooError> {
        decode_body(body.to_string(), status, "https://example.test/url")
    }

    #[test]
    fn test_decode_empty_body() {
        match parse("", StatusCode::OK) {
            Err(YahooError::EmptyResponse) => {}
            other => panic!("expected EmptyResponse, got {:?}", other),
        }
        match parse("   \n\t ", StatusCode::OK) {
            Err(YahooError::EmptyResponse) => {}
            other => panic!("expected EmptyResponse, got {:?}", other),
        }
    }

    #[test]
    fn test_decode_html_body() {
        let html = "<!DOCTYPE html>\n<html lang=\"zh\"><head><title>Yahoo</title></head></html>";
        match parse(html, StatusCode::OK) {
            Err(YahooError::HtmlResponse) => {}
            other => panic!("expected HtmlResponse, got {:?}", other),
        }
    }

    #[test]
    fn test_decode_maintenance_page() {
        // yfinance detects the exact maintenance phrase "Will be right back".
        let body = "Will be right back\n\nThanks for your patience.";
        match parse(body, StatusCode::OK) {
            Err(YahooError::HtmlResponse) => {}
            other => panic!("expected HtmlResponse, got {:?}", other),
        }
    }

    #[test]
    fn test_decode_maintenance_page_long_body() {
        // No length cap: a plain-text outage page longer than 4000 chars (not
        // HTML) must still be classified as a maintenance page.
        let mut body = String::from("Will be right back\n");
        body.push_str(&"x".repeat(5_000));
        match parse(&body, StatusCode::OK) {
            Err(YahooError::HtmlResponse) => {}
            other => panic!("expected HtmlResponse, got {:?}", other),
        }
    }

    #[test]
    fn test_decode_rate_limited() {
        match parse("Too Many Requests", StatusCode::OK) {
            Err(YahooError::TooManyRequests(_)) => {}
            other => panic!("expected TooManyRequests, got {:?}", other),
        }
        match parse("", StatusCode::TOO_MANY_REQUESTS) {
            Err(YahooError::TooManyRequests(_)) => {}
            other => panic!("expected TooManyRequests, got {:?}", other),
        }
    }

    #[test]
    fn test_decode_not_found_status() {
        match parse("", StatusCode::NOT_FOUND) {
            Err(YahooError::FetchFailed(e)) if e.contains("404") => {}
            other => panic!("expected FetchFailed 404, got {:?}", other),
        }
    }

    #[test]
    fn test_decode_server_error_status() {
        match parse("", StatusCode::INTERNAL_SERVER_ERROR) {
            Err(YahooError::ServerError(e)) if e.contains("500 status") => {}
            other => panic!("expected ServerError 500, got {:?}", other),
        }
        match parse("", StatusCode::UNAUTHORIZED) {
            Err(YahooError::Unauthorized) => {}
            other => panic!("expected Unauthorized, got {:?}", other),
        }
        // 403 = stale crumb/cookie, mapped to Unauthorized so the retry guard
        // can refresh the session (same as get_financial_events/get_crumb).
        match parse("", StatusCode::FORBIDDEN) {
            Err(YahooError::Unauthorized) => {}
            other => panic!("expected Unauthorized for 403, got {:?}", other),
        }
    }

    #[test]
    fn test_decode_valid_json() {
        let json = r#"{"chart": {"result": [1, 2, 3], "error": null}}"#;
        let parsed = parse(json, StatusCode::OK).unwrap();
        assert_eq!(parsed["chart"]["result"][0], 1);
    }

    #[test]
    fn test_decode_top_level_error() {
        match parse(
            r#"{"error":{"code":"Too Many Requests","description":"limit reached"}}"#,
            StatusCode::OK,
        ) {
            Err(YahooError::TooManyRequests(_)) => {}
            other => panic!("expected TooManyRequests, got {:?}", other),
        }
        match parse(r#"{"error":{"code":"Unauthorized"}}"#, StatusCode::OK) {
            Err(YahooError::Unauthorized) => {}
            other => panic!("expected Unauthorized, got {:?}", other),
        }
        match parse(r#"{"error":{"code":"Invalid Crumb"}}"#, StatusCode::OK) {
            Err(YahooError::InvalidCrumb) => {}
            other => panic!("expected InvalidCrumb, got {:?}", other),
        }
        match parse(
            r#"{"error":{"code":"Other","description":"boom"}}"#,
            StatusCode::OK,
        ) {
            Err(YahooError::ApiError(err)) => {
                assert_eq!(err.code.as_deref(), Some("Other"));
                assert_eq!(err.description.as_deref(), Some("boom"));
            }
            other => panic!("expected ApiError, got {:?}", other),
        }
        match parse(
            r#"{"error":{"code":"InvalidPeriod","description":"period1 > period2"}}"#,
            StatusCode::UNPROCESSABLE_ENTITY,
        ) {
            Err(YahooError::ApiError(err)) => {
                assert_eq!(err.code.as_deref(), Some("InvalidPeriod"));
            }
            other => panic!("expected ApiError for 422, got {:?}", other),
        }
    }

    #[test]
    fn test_decode_json_mentioning_too_many_requests_not_rate_limit() {
        // A valid JSON error whose description merely mentions "too many
        // requests" must be classified as an ApiError, not a rate limit.
        let json = r#"{"chart":{"error":{"code":"InvalidPeriod","description":"too many requests in period"}}}"#;
        match parse(json, StatusCode::OK) {
            Err(YahooError::ApiError(err)) => {
                assert_eq!(err.code.as_deref(), Some("InvalidPeriod"));
            }
            other => panic!("expected ApiError, got {:?}", other),
        }
        // A plain-text rate-limit body is still definitive.
        match parse("Too Many Requests", StatusCode::OK) {
            Err(YahooError::TooManyRequests(_)) => {}
            other => panic!("expected TooManyRequests, got {:?}", other),
        }
    }

    #[test]
    fn test_decode_truncated_json() {
        let body = r#"{"chart": {"result": ["#;
        let result = parse(body, StatusCode::OK);
        match result {
            #[cfg(feature = "debug")]
            Err(YahooError::DeserializeFailedDebug(text)) => {
                assert!(
                    text.contains("EOF") || text.contains("expected value"),
                    "{}",
                    text
                );
                assert!(text.contains("body:"));
            }
            #[cfg(not(feature = "debug"))]
            Err(YahooError::DeserializeFailed(_)) => {}
            other => panic!("expected DeserializeFailed, got {:?}", other),
        }
    }
}
