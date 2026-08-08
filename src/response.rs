use reqwest::StatusCode;

use crate::YahooError;

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
    if status == StatusCode::NOT_FOUND {
        return Err(YahooError::FetchFailed(format!("404, request url: {}", url)));
    }
    if !status.is_success() {
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
    if body.len() <= 4_000 && body.to_ascii_lowercase().contains("too many requests") {
        return Err(YahooError::TooManyRequests(url.to_string()));
    }

    match serde_json::from_str::<serde_json::Value>(body) {
        Ok(json) => Ok(json),
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
            Err(YahooError::FetchFailed(e)) if e.contains("500 status") => {}
            other => panic!("expected FetchFailed 500, got {:?}", other),
        }
        match parse("", StatusCode::UNAUTHORIZED) {
            Err(YahooError::Unauthorized) => {}
            other => panic!("expected Unauthorized, got {:?}", other),
        }
    }

    #[test]
    fn test_decode_valid_json() {
        let json = r#"{"chart": {"result": [1, 2, 3], "error": null}}"#;
        let parsed = parse(json, StatusCode::OK).unwrap();
        assert_eq!(parsed["chart"]["result"][0], 1);
    }

    #[test]
    fn test_decode_truncated_json() {
        let body = r#"{"chart": {"result": ["#;
        let result = parse(body, StatusCode::OK);
        match result {
            #[cfg(feature = "debug")]
            Err(YahooError::DeserializeFailedDebug(text)) => {
                assert!(text.contains("EOF") || text.contains("expected value"), "{}", text);
                assert!(text.contains("body:"));
            }
            #[cfg(not(feature = "debug"))]
            Err(YahooError::DeserializeFailed(_)) => {}
            other => panic!("expected DeserializeFailed, got {:?}", other),
        }
    }
}