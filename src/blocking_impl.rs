use super::*;
use crate::quotes::{FinancialEvent, YEarningsResponse, YErrorMessage};

impl YahooConnector {
    #[cfg(feature = "governor")]
    fn wait_for_rate_limit_blocking(&self) {
        if let Some(limiter) = &self.rate_limiter {
            use governor::clock::Clock;
            let clock = governor::clock::DefaultClock::default();
            loop {
                match limiter.check() {
                    Ok(_) => break,
                    Err(not_until) => {
                        let wait = not_until.wait_time_from(clock.now());
                        std::thread::sleep(wait);
                    }
                }
            }
        }
    }

    /// Retrieve the quotes of the last month for the given ticker
    pub fn get_latest_quotes(&self, ticker: &str, interval: &str) -> Result<YResponse, YahooError> {
        self.get_quote_range(ticker, interval, "1mo")
    }

    /// Retrieve the quote history for the given ticker from date start to end (inclusive), if available
    pub fn get_quote_history(
        &self,
        ticker: &str,
        start: OffsetDateTime,
        end: OffsetDateTime,
    ) -> Result<YResponse, YahooError> {
        self.get_quote_history_interval(ticker, start, end, "1d")
    }

    /// Retrieve quotes for the given ticker for an arbitrary range
    pub fn get_quote_range(
        &self,
        ticker: &str,
        interval: &str,
        range: &str,
    ) -> Result<YResponse, YahooError> {
        let url: String = format!(
            YCHART_RANGE_QUERY!(),
            url = self.url,
            symbol = crate::percent_encode(ticker),
            interval = interval,
            range = range
        );
        YResponse::from_json(self.send_request_retry(&url)?)?.map_error_msg()
    }

    /// Retrieve the quote history for the given ticker from date start to end (inclusive), if available; specifying the interval of the ticker.
    pub fn get_quote_history_interval(
        &self,
        ticker: &str,
        start: OffsetDateTime,
        end: OffsetDateTime,
        interval: &str,
    ) -> Result<YResponse, YahooError> {
        let url = format!(
            YCHART_PERIOD_QUERY!(),
            url = self.url,
            symbol = crate::percent_encode(ticker),
            start = start.unix_timestamp(),
            end = end.unix_timestamp(),
            interval = interval,
        );
        YResponse::from_json(self.send_request_retry(&url)?)?.map_error_msg()
    }

    /// Retrieve the quote history for the given ticker from date start to end (inclusive) and optionally before and after regular trading hours, if available; specifying the interval of the ticker.
    pub fn get_quote_history_interval_prepost(
        &self,
        ticker: &str,
        start: OffsetDateTime,
        end: OffsetDateTime,
        interval: &str,
        prepost: bool,
    ) -> Result<YResponse, YahooError> {
        let url = format!(
            YCHART_PERIOD_QUERY_PRE_POST!(),
            url = self.url,
            symbol = crate::percent_encode(ticker),
            start = start.unix_timestamp(),
            end = end.unix_timestamp(),
            interval = interval,
            prepost = prepost,
        );
        YResponse::from_json(self.send_request_retry(&url)?)?.map_error_msg()
    }

    /// Retrieve the quote history for the given ticker for a given period and ticker interval and optionally before and after regular trading hours
    pub fn get_quote_period_interval(
        &self,
        ticker: &str,
        range: &str,
        interval: &str,
        prepost: bool,
    ) -> Result<YResponse, YahooError> {
        let url = format!(
            YCHART_PERIOD_INTERVAL_QUERY!(),
            url = self.url,
            symbol = crate::percent_encode(ticker),
            range = range,
            interval = interval,
            prepost = prepost,
        );
        YResponse::from_json(self.send_request_retry(&url)?)?.map_error_msg()
    }

    /// Retrieve the list of quotes found searching a given name
    pub fn search_ticker_opt(&self, name: &str) -> Result<YSearchResultOpt, YahooError> {
        let url = format!(
            YTICKER_QUERY!(),
            url = self.search_url,
            name = crate::percent_encode(name)
        );
        YSearchResultOpt::from_json(self.send_request(&url)?)
    }

    /// Retrieve the list of quotes found searching a given name
    pub fn search_ticker(&self, name: &str) -> Result<YSearchResult, YahooError> {
        let result = self.search_ticker_opt(name)?;
        Ok(YSearchResult::from_opt(&result))
    }

    /// Retrieve the quoteSummary modules (company profile, recommendations,
    /// calendar, holders, financials, ...) for the given symbol. Requires
    /// `&mut self` because the crumb/cookie used for authentication is cached.
    pub fn get_ticker_info(&mut self, symbol: &str) -> Result<YQuoteSummary, YahooError> {
        if symbol.is_empty() {
            return Err(YahooError::FetchFailed(
                "Symbol cannot be empty".to_string(),
            ));
        }
        if self.crumb.is_none() {
            self.crumb = Some(self.get_crumb()?);
        }
        if self.cookie.is_none() {
            self.cookie = Some(self.get_cookie()?);
        }

        let max_retries = 1;
        for i in 0..=max_retries {
            #[cfg(feature = "governor")]
            self.wait_for_rate_limit_blocking();

            // Build URL inside loop to use fresh crumb after refresh
            let url = reqwest::Url::parse(&format!(
                YQUOTE_SUMMARY_QUERY!(),
                url = self.summary_url,
                symbol = crate::percent_encode(symbol),
                crumb = self.crumb.as_deref().ok_or(YahooError::NoResponse)?
            ))
            .map_err(|_| YahooError::InvalidUrl)?;

            let response = self
                .create_client()?
                .get(url)
                .header("Cookie", self.cookie_header_value())
                .send()?;
            let status = response.status();
            let text = response.text()?;

            let result: YQuoteSummary = match crate::response::decode_body(
                text,
                status,
                &format!("get_ticker_info: {}", symbol),
            )
            .and_then(YQuoteSummary::from_json)
            {
                Ok(result) => result,
                // A non-JSON reply (e.g. an HTML error page) usually means the
                // crumb expired; refresh crumb AND cookie (a stale cookie alone
                // would make the retry fail with the same error) and retry once,
                // like get_financial_events does. InvalidCrumb is the same
                // "session expired" signal arriving as a top-level JSON error.
                Err(err) if i < max_retries => match &err {
                    YahooError::EmptyResponse
                    | YahooError::HtmlResponse
                    | YahooError::Unauthorized
                    | YahooError::InvalidCrumb
                    | YahooError::ServerError(_) => {
                        // Refresh the session: drop the cookie so get_crumb
                        // fetches a fresh crumb+cookie pair that Yahoo accepts
                        // together (a separately-refetched cookie may not match
                        // the new crumb and would still be Unauthorized).
                        self.cookie = None;
                        self.crumb = Some(self.get_crumb()?);
                        continue;
                    }
                    _ => return Err(err),
                },
                Err(err) => return Err(err),
            };

            // The v8 API reports errors in `finance.error`, the v10
            // quoteSummary API in `quoteSummary.error`
            let api_error = result
                .finance
                .as_ref()
                .and_then(|f| f.error.as_ref())
                .or_else(|| result.quote_summary.as_ref().and_then(|q| q.error.as_ref()));

            if let Some(error) = api_error {
                if let Some(description) = &error.description {
                    if description.contains("Invalid Crumb") {
                        if i == max_retries {
                            return Err(YahooError::InvalidCrumb);
                        }
                        self.crumb = Some(self.get_crumb()?);
                        continue;
                    }
                }
                if let Some(code) = &error.code {
                    if code.contains("Unauthorized") {
                        if i == max_retries {
                            return Err(YahooError::Unauthorized);
                        }
                        self.crumb = Some(self.get_crumb()?);
                        continue;
                    }
                }
                // Any other API-level error (e.g. unknown symbol) is
                // reported to the caller instead of returning Ok
                return Err(YahooError::ApiError(error.clone()));
            }
            // A successful response without any result block (e.g. `{}` or
            // `"result": []`) means the API had no data for this symbol
            if !result.has_result() {
                return Err(YahooError::NoResult);
            }
            return Ok(result);
        }

        Err(YahooError::NoResponse)
    }

    /// Retrieve financial events (Earnings, Meeting, Call) dates for the given ticker with specified limit (max limit: 250)
    pub fn get_financial_events(
        &mut self,
        ticker: &str,
        limit: u32,
    ) -> Result<Vec<FinancialEvent>, YahooError> {
        if ticker.is_empty() {
            return Err(YahooError::FetchFailed(
                "Ticker cannot be empty".to_string(),
            ));
        }

        // Ensure we have crumb for authentication
        if self.crumb.is_none() {
            self.crumb = Some(self.get_crumb()?);
        }
        if self.cookie.is_none() {
            self.cookie = Some(self.get_cookie()?);
        }

        // Create request body
        let query_body = serde_json::json!({
            "size": limit.min(250),
            "query": {
                "operator": "eq",
                "operands": ["ticker", ticker]
            },
            "sortField": "startdatetime",
            "sortType": "DESC",
            "entityIdType": "earnings",
            "includeFields": [
                "startdatetime",
                "timeZoneShortName",
                "epsestimate",
                "epsactual",
                "epssurprisepct",
                "eventtype"
            ]
        });

        // Setup cookie for authenticated request
        let max_retries = 1;
        for attempt in 0..=max_retries {
            #[cfg(feature = "governor")]
            self.wait_for_rate_limit_blocking();

            // Build URL inside loop to use fresh crumb after refresh
            let url = format!(
                YEARNINGS_QUERY!(),
                url = self.earnings_url,
                lang = "en-US",
                region = "US",
                crumb = self.crumb.as_deref().ok_or(YahooError::NoResponse)?
            );

            let client = self.create_client()?;

            let response = client
                .post(&url)
                .header("Cookie", self.cookie_header_value())
                .header("Content-Type", "application/json")
                .json(&query_body)
                .send()?;

            let status = response.status();

            match status {
                reqwest::StatusCode::TOO_MANY_REQUESTS => {
                    return Err(YahooError::TooManyRequests(format!(
                        "POST {} in get_financial_events for ticker {}",
                        Y_EARNINGS_URL, ticker
                    )));
                }
                reqwest::StatusCode::UNAUTHORIZED => {
                    // A stale cookie can surface as 401 too; refresh the whole
                    // session like the 403 branch below.
                    if attempt < max_retries {
                        self.cookie = None;
                        self.crumb = Some(self.get_crumb()?);
                        continue;
                    } else {
                        return Err(YahooError::Unauthorized);
                    }
                }
                reqwest::StatusCode::FORBIDDEN => {
                    // A stale crumb/cookie often surfaces as 403; refresh and retry.
                    if attempt < max_retries {
                        self.cookie = None;
                        self.crumb = Some(self.get_crumb()?);
                        continue;
                    } else {
                        return Err(YahooError::Unauthorized);
                    }
                }
                reqwest::StatusCode::NOT_FOUND => {
                    return Err(YahooError::FetchFailed(format!(
                        "Ticker {} not found",
                        ticker
                    )));
                }
                // 5xx are transient; retry once with a fresh crumb, like yfinance
                // does for any status >= 400.
                status if status.is_server_error() => {
                    if attempt < max_retries {
                        self.crumb = Some(self.get_crumb()?);
                        continue;
                    } else {
                        return Err(YahooError::FetchFailed(format!("HTTP error: {}", status)));
                    }
                }
                _ if !status.is_success() => {
                    return Err(YahooError::FetchFailed(format!("HTTP error: {}", status)));
                }
                _ => {} // Success, continue
            }

            let text = response.text()?;

            // Try to parse response
            match serde_json::from_str::<YEarningsResponse>(&text) {
                Ok(earnings_response) => {
                    // Check for API errors
                    if let Some(error) = &earnings_response.finance.error {
                        let code = error.get("code").and_then(|v| v.as_str()).unwrap_or("");
                        let description = error
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        // If the crumb is invalid, try to refetch it and retry the request
                        if description.contains("Invalid Crumb")
                            || code.contains("Unauthorized")
                            || code.contains("Invalid Crumb")
                        {
                            if attempt < max_retries {
                                self.crumb = Some(self.get_crumb()?); // Refetch crumb
                                continue; // Go to the next iteration
                            } else {
                                return Err(YahooError::InvalidCrumb);
                            }
                        }

                        return Err(YahooError::ApiError(YErrorMessage {
                            code: Some(code.to_string()),
                            description: Some(description.to_string()),
                        }));
                    }

                    return self.parse_earnings_response(earnings_response);
                }
                Err(e) => {
                    // A parsing error is a critical failure unless we are retrying.
                    if attempt < max_retries {
                        // The session may have expired; drop the cookie so
                        // get_crumb fetches a fresh crumb+cookie pair (a stale
                        // cookie alone would make the retry fail the same way).
                        self.cookie = None;
                        self.crumb = Some(self.get_crumb()?);
                        continue;
                    } else {
                        // If parsing fails on the last attempt, return the error.
                        return Err(YahooError::DeserializeFailed(e));
                    }
                }
            }
        }

        Err(YahooError::NoResponse)
    }

    /// Parse earnings response into structured data
    fn parse_earnings_response(
        &self,
        response: YEarningsResponse,
    ) -> Result<Vec<FinancialEvent>, YahooError> {
        let mut earnings_events = Vec::new();

        if response.finance.result.is_empty() {
            return Ok(earnings_events);
        }

        // The schema allows multiple results/documents (one per event type);
        // aggregate events across all of them instead of taking only the first.
        for result in &response.finance.result {
            if result.documents.is_empty() {
                continue;
            }

            for document in &result.documents {
                if document.columns.is_empty() {
                    continue;
                }

                // Map column names to indices
                let mut column_map = std::collections::HashMap::new();
                for (index, column) in document.columns.iter().enumerate() {
                    column_map.insert(column.label.as_str(), index);
                }

                // Parse each row; a single malformed row (e.g. a null date) must not
                // discard the whole response, so skip rows that fail to parse
                for row in &document.rows {
                    if let Ok(earnings_event) = self.parse_earnings_row(row, &column_map) {
                        earnings_events.push(earnings_event);
                    }
                }
            }
        }

        Ok(earnings_events)
    }

    /// Parse individual earnings row
    fn parse_earnings_row(
        &self,
        row: &[serde_json::Value],
        column_map: &std::collections::HashMap<&str, usize>,
    ) -> Result<FinancialEvent, YahooError> {
        // Extract earnings date
        let get_value = |col_name: &str| column_map.get(col_name).and_then(|&idx| row.get(idx));

        let earnings_date = match get_value("Event Start Date").and_then(|v| v.as_str()) {
            Some(date_str) => {
                OffsetDateTime::parse(date_str, &time::format_description::well_known::Rfc3339)
                    .or_else(|_| {
                        OffsetDateTime::parse(
                            date_str,
                            &time::format_description::well_known::Iso8601::DEFAULT,
                        )
                    })
                    .map_err(|_| YahooError::InvalidDateFormat)?
            }
            None => return Err(YahooError::MissingField("Event Start Date".to_string())),
        };

        // Extract event type and convert codes
        let event_type = get_value("Event Type")
            .map(|v| {
                if let Some(s) = v.as_str() {
                    s.to_string()
                } else if let Some(i) = v.as_i64() {
                    i.to_string()
                } else {
                    "Unknown".to_string()
                }
            })
            .unwrap_or_else(|| "Unknown".to_string());

        let event_type = match event_type.as_str() {
            "1" => "Call".to_string(),
            "2" => "Earnings".to_string(),
            "11" => "Meeting".to_string(),
            other => other.to_string(),
        };
        let eps_estimate = get_value("EPS Estimate").and_then(|v| v.as_f64());
        let reported_eps = get_value("Reported EPS").and_then(|v| v.as_f64());
        let surprise_percent = get_value("Surprise (%)").and_then(|v| v.as_f64());
        let timezone = get_value("Timezone short name")
            .and_then(|v| v.as_str())
            .map(String::from);

        Ok(FinancialEvent {
            earnings_date,
            event_type,
            eps_estimate,
            reported_eps,
            surprise_percent,
            timezone,
        })
    }

    /// Get only earnings events (filter out all non-earnings events, e.g. meetings and calls)
    pub fn get_earnings_only(
        &mut self,
        ticker: &str,
        limit: u32,
    ) -> Result<Vec<FinancialEvent>, YahooError> {
        let all_events = self.get_financial_events(ticker, limit)?;
        Ok(all_events
            .into_iter()
            .filter(|event| event.event_type == "Earnings")
            .collect())
    }

    fn get_crumb(&mut self) -> Result<String, YahooError> {
        if self.cookie.is_none() {
            self.cookie = Some(self.get_cookie()?);
        }

        const MAX_RETRIES: usize = 1;
        let crumb_url = reqwest::Url::parse(&self.crumb_url).map_err(|_| YahooError::InvalidUrl)?;
        let mut last_error = YahooError::NoResponse;

        for _attempt in 0..=MAX_RETRIES {
            #[cfg(feature = "governor")]
            self.wait_for_rate_limit_blocking();

            let response = self
                .create_client()?
                .get(crumb_url.clone())
                .header("Cookie", self.cookie_header_value())
                .send()?;

            let status = response.status();
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                return Err(YahooError::TooManyRequests(format!(
                    "GET {} in get_crumb",
                    self.crumb_url
                )));
            }
            if !status.is_success() {
                // Yahoo answers an invalid cookie on the crumb endpoint with a
                // 401 status (not only with an "Invalid Cookie" body): refresh
                // the cookie and retry, like the body check below. A 403/404
                // usually means the session expired too — refresh the cookie
                // and retry once before failing hard (yfinance falls back to
                // another host/strategy in this situation).
                if status == reqwest::StatusCode::UNAUTHORIZED
                    || status == reqwest::StatusCode::FORBIDDEN
                    || status == reqwest::StatusCode::NOT_FOUND
                {
                    if _attempt < MAX_RETRIES {
                        self.cookie = Some(self.get_cookie()?);
                        last_error = YahooError::Unauthorized;
                        continue;
                    }
                    // Same condition on the final attempt: report the session
                    // as invalid (Unauthorized) rather than a generic fetch
                    // failure, matching the first-attempt classification.
                    return Err(YahooError::Unauthorized);
                }
                return Err(YahooError::FetchFailed(format!(
                    "{} status, GET {} in get_crumb",
                    status.as_u16(),
                    self.crumb_url
                )));
            }
            let crumb = response.text()?;
            let crumb = crumb.trim();

            if crumb.contains("Invalid Cookie") {
                // Refresh the cookie before retrying; skip the refresh on the
                // final attempt (the loop is about to terminate anyway).
                if _attempt < MAX_RETRIES {
                    self.cookie = Some(self.get_cookie()?);
                }
                last_error = YahooError::InvalidCookie;
                continue;
            }

            if crumb.contains("Too Many Requests") {
                // A rate limit is definitive (see test_429_is_not_retried):
                // retrying would only add load to an already-limited endpoint.
                return Err(YahooError::TooManyRequests(format!(
                    "GET {} in get_crumb",
                    self.crumb_url
                )));
            }

            // A maintenance/HTML page served with a 200 status is not a valid
            // crumb (yfinance checks `'<html>' in crumb` and falls back to a
            // fresh session); treat it like an empty crumb.
            if crumb.contains("<html>") || crumb.contains("<HTML>") {
                if _attempt < MAX_RETRIES {
                    self.cookie = Some(self.get_cookie()?);
                }
                last_error = YahooError::InvalidCrumb;
                continue;
            }

            if crumb.is_empty() {
                // An empty crumb often means the session/cookie expired;
                // refresh it before retrying, otherwise the retry is doomed.
                if _attempt < MAX_RETRIES {
                    self.cookie = Some(self.get_cookie()?);
                }
                last_error = YahooError::InvalidCrumb;
                continue;
            }

            return Ok(crumb.to_string());
        }

        Err(last_error)
    }

    fn get_cookie(&mut self) -> Result<String, YahooError> {
        #[cfg(feature = "governor")]
        self.wait_for_rate_limit_blocking();
        let response = self.client.get(&self.cookie_url).send()?;
        let status = response.status();
        // A 429 (rate limit) is definitive and must win over any cookie the
        // error page happened to set — check it before parsing headers, so a
        // non-UTF-8 cookie on a 429 page cannot surface as a cookie error.
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(YahooError::TooManyRequests(format!(
                "GET {} in get_cookie",
                self.cookie_url
            )));
        }
        // Yahoo may return several Set-Cookie headers; collect all of them so
        // an A3 that is not first is not missed. Each header is stripped to
        // its first `name=value` segment (attributes dropped by
        // cookie_header_value).
        let mut cookies = Vec::new();
        for value in response.headers().get_all(Y_COOKIE_REQUEST_HEADER) {
            let v = value
                .to_str()
                .map_err(|_| YahooError::InvisibleAsciiInCookies)?;
            let first = v.split(';').next().unwrap_or(v).trim();
            if !first.is_empty() {
                cookies.push(first.to_string());
            }
        }
        // Yahoo serves the A3 cookie even on error pages (fc.yahoo.com
        // currently answers 404 with a valid Set-Cookie header), so a
        // non-rate-limit error page still yields a usable cookie.
        if !cookies.is_empty() {
            return Ok(cookies.join("; "));
        }
        if !status.is_success() {
            return Err(YahooError::FetchFailed(format!(
                "{} status, GET {} in get_cookie",
                status.as_u16(),
                self.cookie_url
            )));
        }
        Err(YahooError::NoCookies)
    }

    /// Clone the existing client (already has proxy, timeout, user_agent).
    /// Cookie is added via request-level header instead of client-level cookie_provider.
    fn create_client(&self) -> Result<Client, reqwest::Error> {
        Ok(self.client.clone())
    }

    /// Return the stored `name=value` cookie pairs ready for the Cookie header.
    /// `get_cookie` already strips Set-Cookie attributes (expires, path,
    /// domain, ...), so the stored value is the whole Cookie header.
    fn cookie_header_value(&self) -> String {
        self.cookie.clone().unwrap_or_default()
    }

    /// Send request to yahoo! finance server and transform response to JSON value
    fn send_request(&self, url: &str) -> Result<serde_json::Value, YahooError> {
        #[cfg(feature = "governor")]
        self.wait_for_rate_limit_blocking();
        let response = self.client.get(url).send()?;
        let status = response.status();
        let text = response.text()?;
        crate::response::decode_body(text, status, url)
    }

    /// Send a chart request, retrying once when Yahoo answers with a transient
    /// failure (maintenance HTML page, empty body, or a 5xx). The chart
    /// endpoints need no crumb/cookie, so the retry reuses the same URL.
    fn send_request_retry(&self, url: &str) -> Result<serde_json::Value, YahooError> {
        const MAX_RETRIES: usize = 1;
        let mut last_error = YahooError::NoResponse;
        for _attempt in 0..=MAX_RETRIES {
            match self.send_request(url) {
                Err(
                    err @ (YahooError::EmptyResponse
                    | YahooError::HtmlResponse
                    | YahooError::ServerError(_)),
                ) => last_error = err,
                other => return other,
            }
        }
        Err(last_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn test_get_single_quote() {
        let provider = YahooConnector::new().unwrap();
        let response = provider.get_latest_quotes("HNL.DE", "1d").unwrap();
        let result = &response.chart.result.as_ref().unwrap();

        assert_eq!(&result[0].meta.symbol, "HNL.DE");
        assert_eq!(&result[0].meta.range, "1mo");
        assert_eq!(&result[0].meta.data_granularity, "1d");
        let _ = response.last_quote().unwrap();
    }

    #[test]
    fn test_strange_api_responses() {
        let provider = YahooConnector::new().unwrap();
        let start = datetime!(2019-07-03 0:00:00.00 UTC);
        let end = datetime!(2020-07-04 23:59:59.99 UTC);
        let response = provider.get_quote_history("IBM", start, end).unwrap();
        let result = &response.chart.result.as_ref().unwrap();

        assert_eq!(&result[0].meta.symbol, "IBM");
        assert_eq!(&result[0].meta.data_granularity, "1d");
        assert_eq!(&result[0].meta.first_trade_date, &Some(-252322200));

        let _ = response.last_quote().unwrap();
    }

    #[test]
    #[should_panic(expected = "NoQuotes")]
    fn test_api_responses_missing_fields() {
        let provider = YahooConnector::new().unwrap();
        let response = provider.get_latest_quotes("BF.B", "1m").unwrap();
        let result = &response.chart.result.as_ref().unwrap();

        assert_eq!(&result[0].meta.symbol, "BF.B");
        let _ = response.last_quote().unwrap();
    }

    #[test]
    fn test_get_quote_history() {
        let provider = YahooConnector::new().unwrap();

        let start = datetime!(2020-01-01 0:00:00.00 UTC);
        let end = datetime!(2020-01-31 23:59:59.99 UTC);

        let response = provider.get_quote_history("AAPL", start, end);
        assert!(response.is_ok());

        let response = response.unwrap();
        let result = &response.chart.result.as_ref().unwrap();

        assert_eq!(result[0].timestamp.as_ref().unwrap().len(), 21);
        let quotes = response.quotes().unwrap();
        assert_eq!(quotes.len(), 21);
    }

    #[test]
    fn test_get_quote_range() {
        let provider = YahooConnector::new().unwrap();
        let response = provider.get_quote_range("HNL.DE", "1d", "1mo").unwrap();
        let result = &response.chart.result.as_ref().unwrap();

        assert_eq!(&result[0].meta.symbol, "HNL.DE");
        assert_eq!(&result[0].meta.range, "1mo");
        assert_eq!(&result[0].meta.data_granularity, "1d");
        let _ = response.last_quote().unwrap();
    }

    #[test]
    fn test_get_metadata() {
        let provider = YahooConnector::new().unwrap();
        let response = provider.get_quote_range("HNL.DE", "1d", "1mo").unwrap();
        let metadata = response.metadata().unwrap();
        assert_eq!(metadata.symbol, "HNL.DE");
    }

    #[test]
    fn test_get_quote_history_interval() {
        let provider = YahooConnector::new().unwrap();

        let start = datetime!(2019-01-01 0:00:00.00 UTC);
        let end = datetime!(2020-01-31 23:59:59.99 UTC);

        let response = provider
            .get_quote_history_interval("AAPL", start, end, "1mo")
            .unwrap();
        let result = &response.chart.result.as_ref().unwrap();

        assert_eq!(&result[0].timestamp.as_ref().unwrap().len(), &13);
        assert_eq!(&result[0].meta.data_granularity, "1mo");
        let quotes = response.quotes().unwrap();
        assert_eq!(quotes.len(), 13usize);
    }

    #[test]
    #[should_panic(expected = "ApiError")]
    fn test_wrong_request_get_quote_history_interval() {
        let provider = YahooConnector::new().unwrap();
        let end = OffsetDateTime::now_utc();
        let days = 365;
        let start = end - Duration::from_secs(days * 24 * 60 * 60);
        let interval = "5m";
        let ticker = "AAPL";
        let prepost = true;

        let _ = provider
            .get_quote_history_interval_prepost(ticker, start, end, interval, prepost)
            .unwrap();
    }

    #[test]
    fn test_get_quote_period_interval() {
        let provider = YahooConnector::new().unwrap();

        let range = "5d";
        let interval = "5m";

        let response = provider
            .get_quote_period_interval("AAPL", range, interval, true)
            .unwrap();

        let metadata = response.metadata().unwrap();

        assert_eq!(metadata.data_granularity, interval);
        assert_eq!(metadata.range, range);
    }

    #[test]
    fn test_large_volume() {
        let provider = YahooConnector::new().unwrap();
        let response = provider.get_quote_range("BTC-USD", "1d", "5d").unwrap();
        let quotes = response.quotes().unwrap();
        assert!(!quotes.is_empty());
    }

    #[test]
    fn test_search_ticker() {
        let provider = YahooConnector::new().unwrap();
        let response = provider.search_ticker("Apple").unwrap();

        assert!(response.count > 0);
        let mut apple_found = false;
        for item in response.quotes {
            if item.exchange == "NMS" && item.symbol == "AAPL" && item.short_name == "Apple Inc." {
                apple_found = true;
                break;
            }
        }
        assert!(apple_found)
    }

    #[test]
    fn test_mutual_fund_history() {
        let provider = YahooConnector::new().unwrap();

        let start = datetime!(2020-01-01 0:00:00.00 UTC);
        let end = datetime!(2020-01-31 23:59:59.99 UTC);

        let response = provider.get_quote_history("VTSAX", start, end);
        if let Ok(response) = response {
            let result = &response.chart.result.as_ref().unwrap();

            assert_eq!(result[0].timestamp.as_ref().unwrap().len(), 21);
            let quotes = response.quotes().unwrap();
            assert_eq!(quotes.len(), 21);
        }
    }

    #[test]
    fn test_mutual_fund_latest() {
        let provider = YahooConnector::new().unwrap();
        let response = provider.get_latest_quotes("VTSAX", "1d").unwrap();
        let result = &response.chart.result.as_ref().unwrap();

        assert_eq!(&result[0].meta.symbol, "VTSAX");
        assert_eq!(&result[0].meta.range, "1mo");
        assert_eq!(&result[0].meta.data_granularity, "1d");
        let _ = response.last_quote().unwrap();
    }

    #[test]
    fn test_mutual_fund_latest_with_null_first_trade_date() {
        let provider = YahooConnector::new().unwrap();
        let response = provider.get_latest_quotes("SIWA.F", "1d").unwrap();
        let result = &response.chart.result.as_ref().unwrap();

        assert_eq!(&result[0].meta.symbol, "SIWA.F");
        assert_eq!(&result[0].meta.range, "1mo");
        assert_eq!(&result[0].meta.data_granularity, "1d");
        let _ = response.last_quote().unwrap();
    }

    #[test]
    fn test_mutual_fund_range() {
        let provider = YahooConnector::new().unwrap();
        let response = provider.get_quote_range("VTSAX", "1d", "1mo").unwrap();
        let result = &response.chart.result.as_ref().unwrap();

        assert_eq!(&result[0].meta.symbol, "VTSAX");
        assert_eq!(&result[0].meta.range, "1mo");
        assert_eq!(&result[0].meta.data_granularity, "1d");
    }

    #[ignore]
    #[test]
    fn test_mutual_fund_capital_gains() {
        let provider = YahooConnector::new().unwrap();
        let response = provider.get_quote_range("AMAGX", "1d", "5y").unwrap();
        let result = &response.chart.result.as_ref().unwrap();

        assert_eq!(&result[0].meta.symbol, "AMAGX");
        assert_eq!(&result[0].meta.range, "5y");
        assert_eq!(&result[0].meta.data_granularity, "1d");
        let capital_gains = response.capital_gains().unwrap();
        assert!(!capital_gains.is_empty());
    }

    #[test]
    fn test_get_ticker_info() {
        let mut provider = YahooConnector::new().unwrap();

        let result = provider.get_ticker_info("AAPL");

        let quote_summary = result.unwrap().quote_summary.unwrap();
        // asset_profile is optional and its city may change over time; assert
        // the module parses and the city (if present) is non-empty.
        let profile = quote_summary.result.as_ref().unwrap()[0]
            .asset_profile
            .as_ref()
            .expect("assetProfile module");
        if let Some(city) = &profile.city {
            assert!(!city.is_empty(), "city must not be empty");
        }
    }

    fn fetch_summary(provider: &mut YahooConnector, symbol: &str) -> YSummaryData {
        let result = provider.get_ticker_info(symbol).unwrap();
        result.quote_summary.unwrap().result.unwrap().remove(0)
    }

    #[test]
    fn test_module_recommendation_trend() {
        let mut provider = YahooConnector::new().unwrap();
        let s = fetch_summary(&mut provider, "AAPL");
        let trend = s.recommendation_trend.expect("recommendationTrend module");
        assert!(!trend.trend.is_empty());
        assert!(trend.trend[0].strong_buy.is_some());
    }

    #[test]
    fn test_module_earnings_trend() {
        let mut provider = YahooConnector::new().unwrap();
        let s = fetch_summary(&mut provider, "AAPL");
        let trend = s.earnings_trend.expect("earningsTrend module");
        assert!(!trend.trend.is_empty());
        let item = &trend.trend[0];
        assert!(item.earnings_estimate.is_some() || item.revenue_estimate.is_some());
    }

    #[test]
    fn test_module_earnings_history() {
        let mut provider = YahooConnector::new().unwrap();
        let s = fetch_summary(&mut provider, "AAPL");
        let history = s.earnings_history.expect("earningsHistory module");
        assert!(!history.history.is_empty());
        assert!(history.history[0].period.is_some());
    }

    #[test]
    fn test_module_earnings() {
        let mut provider = YahooConnector::new().unwrap();
        let s = fetch_summary(&mut provider, "AAPL");
        let earnings = s.earnings.expect("earnings module");
        let chart = earnings.earnings_chart.expect("earningsChart");
        assert!(!chart.quarterly.is_empty());
        assert!(earnings.financials_chart.is_some());
    }

    #[test]
    fn test_module_upgrade_downgrade_history() {
        let mut provider = YahooConnector::new().unwrap();
        let s = fetch_summary(&mut provider, "AAPL");
        let history = s
            .upgrade_downgrade_history
            .expect("upgradeDowngradeHistory module");
        assert!(!history.history.is_empty());
        assert!(history.history[0].firm.is_some());
    }

    #[test]
    fn test_module_calendar_events() {
        let mut provider = YahooConnector::new().unwrap();
        let s = fetch_summary(&mut provider, "AAPL");
        let events = s.calendar_events.expect("calendarEvents module");
        assert!(events.earnings.is_some());
        assert!(events.dividend_date.is_some() || events.ex_dividend_date.is_some());
    }

    #[test]
    fn test_module_insider_holders() {
        let mut provider = YahooConnector::new().unwrap();
        let s = fetch_summary(&mut provider, "AAPL");
        let holders = s.insider_holders.expect("insiderHolders module");
        assert!(!holders.holders.is_empty());
        assert!(holders.holders[0].name.is_some());
    }

    #[test]
    fn test_module_insider_transactions() {
        let mut provider = YahooConnector::new().unwrap();
        let s = fetch_summary(&mut provider, "AAPL");
        let transactions = s.insider_transactions.expect("insiderTransactions module");
        assert!(!transactions.transactions.is_empty());
        assert!(transactions.transactions[0].filer_name.is_some());
    }

    #[test]
    fn test_module_major_holders_breakdown() {
        let mut provider = YahooConnector::new().unwrap();
        let s = fetch_summary(&mut provider, "AAPL");
        let breakdown = s
            .major_holders_breakdown
            .expect("majorHoldersBreakdown module");
        assert!(breakdown.institutions_count.is_some());
    }

    #[test]
    fn test_module_institution_ownership() {
        let mut provider = YahooConnector::new().unwrap();
        let s = fetch_summary(&mut provider, "AAPL");
        let ownership = s
            .institution_ownership
            .expect("institutionOwnership module");
        assert!(!ownership.ownership_list.is_empty());
        assert!(ownership.ownership_list[0].organization.is_some());
    }

    #[test]
    fn test_module_fund_ownership() {
        let mut provider = YahooConnector::new().unwrap();
        let s = fetch_summary(&mut provider, "AAPL");
        let ownership = s.fund_ownership.expect("fundOwnership module");
        assert!(!ownership.ownership_list.is_empty());
    }

    #[test]
    fn test_module_net_share_purchase_activity() {
        let mut provider = YahooConnector::new().unwrap();
        let s = fetch_summary(&mut provider, "AAPL");
        let activity = s
            .net_share_purchase_activity
            .expect("netSharePurchaseActivity module");
        assert!(activity.period.is_some());
    }

    #[test]
    fn test_module_sec_filings() {
        let mut provider = YahooConnector::new().unwrap();
        let s = fetch_summary(&mut provider, "AAPL");
        let filings = s.sec_filings.expect("secFilings module");
        assert!(!filings.filings.is_empty());
        assert!(filings.filings[0].filing_type.is_some());
    }

    #[test]
    fn test_module_fund_profile() {
        let mut provider = YahooConnector::new().unwrap();
        let s = fetch_summary(&mut provider, "VOO");
        let profile = s.fund_profile.expect("fundProfile module");
        assert!(profile.family.is_some());
        assert!(profile.fees_expenses_investment.is_some());
    }

    #[test]
    fn test_module_top_holdings() {
        let mut provider = YahooConnector::new().unwrap();
        let s = fetch_summary(&mut provider, "VOO");
        let holdings = s.top_holdings.expect("topHoldings module");
        assert!(!holdings.holdings.is_empty());
        assert!(holdings.holdings[0].symbol.is_some());
        assert!(!holdings.sector_weightings.is_empty());
    }

    #[test]
    fn test_get_crumb() {
        let mut provider = YahooConnector::new().unwrap();
        let crumb = provider.get_crumb().unwrap();

        assert!(crumb.len() > 5);
        assert!(crumb.len() < 16);

        // The crumb must actually authorize a quoteSummary request.
        let result = provider.get_ticker_info("AAPL").unwrap();
        assert!(result.has_result());
    }

    #[test]
    fn test_get_cookie() {
        let mut provider = YahooConnector::new().unwrap();
        let cookie = provider.get_cookie().unwrap();

        // get_cookie now strips Set-Cookie attributes (expires/domain/path...)
        // and returns only `name=value` pairs, so it must contain a '=' but no
        // attribute keywords.
        assert!(cookie.len() > 5);
        assert!(cookie.contains('='));
        assert!(!cookie.contains("Expires"));
        assert!(!cookie.contains("Domain"));
    }

    #[test]
    fn test_neg_time_stamp() {
        let start = datetime!(1960-01-01 0:00:00.00 UTC);
        let end = datetime!(2025-04-30 23:59:59.99 UTC);

        let provider = YahooConnector::new().unwrap();
        let response = provider.get_quote_history("XOM", start, end).unwrap();
        let quotes = response.quotes();
        assert!(!quotes.is_err());
        let quotes = quotes.unwrap();
        // History grows daily; assert a sane lower bound instead of an exact count.
        assert!(quotes.len() >= 15_000);
    }

    #[test]
    fn test_get_financial_events() {
        let mut provider = YahooConnector::new().unwrap();
        let limit = 100;
        let result = provider.get_financial_events("AAPL", limit);

        if result.is_err() {
            println!("{:?}", result);
        }

        assert!(result.is_ok());
        let earnings = result.unwrap();

        // The parser intentionally skips malformed rows, so the returned count
        // may be below the requested limit; assert a sane lower bound.
        assert!(!earnings.is_empty());
        assert!(earnings.len() as u32 <= limit);
    }

    #[test]
    fn test_get_earnings_only() {
        let mut provider = YahooConnector::new().unwrap();
        let result = provider.get_earnings_only("AAPL", 100);

        assert!(result.is_ok());
        let earnings = result.unwrap();

        // All events should be earnings type
        for event in &earnings {
            assert_eq!(event.event_type, "Earnings");
        }

        println!("Earnings-only events: {}", earnings.len());
    }

    #[cfg(feature = "governor")]
    #[test]
    fn test_governor_throttling_blocking() {
        use std::num::NonZeroU32;
        use std::time::Instant;

        // 2 requests per second means ~500ms interval between tokens, burst of 2
        let provider = YahooConnector::builder()
            .rate_limit(Some(NonZeroU32::new(2).unwrap()))
            .build()
            .unwrap();

        let start = Instant::now();
        // First two requests consume the burst capacity of 2
        provider.wait_for_rate_limit_blocking();
        provider.wait_for_rate_limit_blocking();
        let initial_elapsed = start.elapsed();

        // Third request must wait for the next token (~500ms)
        provider.wait_for_rate_limit_blocking();
        let total_elapsed = start.elapsed();

        assert!(initial_elapsed.as_millis() < 100);
        assert!(total_elapsed.as_millis() >= 400);
    }

    #[cfg(feature = "governor")]
    #[test]
    fn test_governor_exact_rate_5_req_per_sec_blocking() {
        use std::num::NonZeroU32;
        use std::time::Instant;

        // 5 req/sec -> 200ms per request after initial burst of 5
        let provider = YahooConnector::builder()
            .rate_limit(Some(NonZeroU32::new(5).unwrap()))
            .build()
            .unwrap();

        let start = Instant::now();
        // Send 8 requests: 5 burst + 3 * 200ms = 600ms total expected delay
        for _ in 0..8 {
            provider.wait_for_rate_limit_blocking();
        }
        let elapsed = start.elapsed();

        println!("5 req/sec blocking test for 8 requests took {:?}", elapsed);
        assert!(
            elapsed.as_millis() >= 500 && elapsed.as_millis() <= 1500,
            "Expected 8 requests at 5 req/sec (blocking) to take ~600ms, but took {}ms",
            elapsed.as_millis()
        );
    }

    #[cfg(feature = "governor")]
    #[test]
    fn test_governor_disabled_no_delay_blocking() {
        use std::time::Instant;

        let provider = YahooConnector::builder().rate_limit(None).build().unwrap();

        let start = Instant::now();
        for _ in 0..20 {
            provider.wait_for_rate_limit_blocking();
        }
        let elapsed = start.elapsed();

        assert!(
            elapsed.as_millis() < 50,
            "Disabled rate limit took {}ms for 20 requests in blocking mode, expected <50ms",
            elapsed.as_millis()
        );
    }

    // ------------------------------------------------------------------
    // Retry logic tests against a local mock HTTP server (no real network)
    // ------------------------------------------------------------------

    fn mock_connector(mock: &crate::mock_server::MockServer) -> YahooConnector {
        YahooConnector {
            client: reqwest::blocking::Client::new(),
            url: mock.chart_url(),
            search_url: crate::YSEARCH_URL,
            summary_url: mock.summary_url(),
            earnings_url: mock.earnings_url(),
            cookie_url: mock.cookie_url(),
            crumb_url: mock.crumb_url(),
            cookie: None,
            crumb: None,
            #[cfg(feature = "governor")]
            rate_limiter: None,
        }
    }

    const MOCK_COOKIE: &str = "A3=mockcookie";

    fn queue_cookie(mock: &crate::mock_server::MockServer) {
        mock.enqueue(200, &[("Set-Cookie", MOCK_COOKIE)], "");
    }

    fn queue_crumb(mock: &crate::mock_server::MockServer, crumb: &str) {
        mock.enqueue_plain(200, crumb);
    }

    fn queue_summary_success(mock: &crate::mock_server::MockServer) {
        mock.enqueue_plain(
            200,
            include_str!("../tests/fixtures/quote_summary_aapl.json"),
        );
    }

    #[test]
    fn test_retry_on_401_refreshes_crumb() {
        let mock = crate::mock_server::MockServer::start();
        queue_cookie(&mock);
        queue_crumb(&mock, "abc");
        mock.enqueue(
            401,
            &[],
            r#"{"finance":{"error":{"code":"Unauthorized","description":"Invalid Crumb"}}}"#,
        );
        queue_cookie(&mock);
        queue_crumb(&mock, "xyz");
        queue_summary_success(&mock);

        let mut conn = mock_connector(&mock);
        let result = conn.get_ticker_info("AAPL");

        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
        assert_eq!(conn.crumb.as_deref(), Some("xyz"));
        let lines = mock.request_lines();
        assert_eq!(lines.len(), 6, "unexpected request sequence: {lines:?}");
        assert_eq!(lines[0], "GET / HTTP/1.1");
        assert_eq!(lines[1], "GET /v1/test/getcrumb HTTP/1.1");
        assert!(lines[2].contains("crumb=abc"), "got: {}", lines[2]);
        assert_eq!(lines[3], "GET / HTTP/1.1");
        assert_eq!(lines[4], "GET /v1/test/getcrumb HTTP/1.1");
        assert!(lines[5].contains("crumb=xyz"), "got: {}", lines[5]);
    }

    #[test]
    fn test_retry_on_403_refreshes_crumb() {
        // A 403 (stale crumb/cookie) is decoded as Unauthorized by
        // decode_body, so the retry guard must refresh the crumb and retry.
        let mock = crate::mock_server::MockServer::start();
        queue_cookie(&mock);
        queue_crumb(&mock, "abc");
        mock.enqueue_plain(403, "forbidden");
        queue_cookie(&mock);
        queue_crumb(&mock, "xyz");
        queue_summary_success(&mock);

        let mut conn = mock_connector(&mock);
        let result = conn.get_ticker_info("AAPL");

        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
        assert_eq!(conn.crumb.as_deref(), Some("xyz"));
        let lines = mock.request_lines();
        assert_eq!(lines.len(), 6, "unexpected request sequence: {lines:?}");
        assert!(lines[2].contains("crumb=abc"), "got: {}", lines[2]);
        assert_eq!(lines[3], "GET / HTTP/1.1");
        assert_eq!(lines[4], "GET /v1/test/getcrumb HTTP/1.1");
        assert!(lines[5].contains("crumb=xyz"), "got: {}", lines[5]);
    }

    #[test]
    fn test_retry_on_empty_response() {
        let mock = crate::mock_server::MockServer::start();
        queue_cookie(&mock);
        queue_crumb(&mock, "abc");
        mock.enqueue_plain(200, "");
        queue_cookie(&mock);
        queue_crumb(&mock, "xyz");
        queue_summary_success(&mock);

        let mut conn = mock_connector(&mock);
        let result = conn.get_ticker_info("AAPL");

        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
        assert_eq!(conn.crumb.as_deref(), Some("xyz"));
        let lines = mock.request_lines();
        assert_eq!(lines.len(), 6);
        assert!(lines[2].contains("crumb=abc"));
        assert_eq!(lines[3], "GET / HTTP/1.1");
        assert_eq!(lines[4], "GET /v1/test/getcrumb HTTP/1.1");
        assert!(lines[5].contains("crumb=xyz"));
    }

    #[test]
    fn test_retry_on_html_response() {
        let mock = crate::mock_server::MockServer::start();
        queue_cookie(&mock);
        queue_crumb(&mock, "abc");
        mock.enqueue_plain(200, "<html><body>blocked</body></html>");
        queue_cookie(&mock);
        queue_crumb(&mock, "xyz");
        queue_summary_success(&mock);

        let mut conn = mock_connector(&mock);
        let result = conn.get_ticker_info("AAPL");

        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
        let lines = mock.request_lines();
        assert_eq!(lines.len(), 6);
    }

    #[test]
    fn test_retry_on_api_error_invalid_crumb() {
        let mock = crate::mock_server::MockServer::start();
        queue_cookie(&mock);
        queue_crumb(&mock, "abc");
        mock.enqueue_plain(
            200,
            r#"{"quoteSummary":{"error":{"code":"Unauthorized","description":"Invalid Crumb"}}}"#,
        );
        queue_crumb(&mock, "xyz");
        queue_summary_success(&mock);

        let mut conn = mock_connector(&mock);
        let result = conn.get_ticker_info("AAPL");

        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
        assert_eq!(conn.crumb.as_deref(), Some("xyz"));
        let lines = mock.request_lines();
        assert_eq!(lines.len(), 5);
    }

    #[test]
    fn test_get_crumb_refreshes_cookie_on_401() {
        // Yahoo rejects an invalid cookie on the crumb endpoint with a 401
        // status; get_crumb must refresh the cookie and retry.
        let mock = crate::mock_server::MockServer::start();
        mock.enqueue(401, &[], "");
        queue_cookie(&mock);
        queue_crumb(&mock, "abc");

        let mut conn = mock_connector(&mock);
        conn.cookie = Some("garbage".to_string());
        conn.crumb = None;

        let crumb = conn.get_crumb().unwrap();
        assert_eq!(crumb, "abc");
        let lines = mock.request_lines();
        assert_eq!(lines.len(), 3, "unexpected request sequence: {lines:?}");
        assert_eq!(lines[0], "GET /v1/test/getcrumb HTTP/1.1");
        assert_eq!(lines[1], "GET / HTTP/1.1");
        assert_eq!(lines[2], "GET /v1/test/getcrumb HTTP/1.1");
    }

    #[test]
    fn test_unauthorized_after_exhausted_retries() {
        let mock = crate::mock_server::MockServer::start();
        queue_cookie(&mock);
        queue_crumb(&mock, "abc");
        mock.enqueue(
            401,
            &[],
            r#"{"finance":{"error":{"code":"Unauthorized","description":"Invalid Crumb"}}}"#,
        );
        queue_cookie(&mock);
        queue_crumb(&mock, "xyz");
        mock.enqueue(
            401,
            &[],
            r#"{"finance":{"error":{"code":"Unauthorized","description":"Invalid Crumb"}}}"#,
        );

        let mut conn = mock_connector(&mock);
        let result = conn.get_ticker_info("AAPL");

        assert!(matches!(result, Err(YahooError::Unauthorized)));
        assert_eq!(conn.crumb.as_deref(), Some("xyz"));
        assert_eq!(mock.request_lines().len(), 6);
    }

    #[test]
    fn test_bad_cookie_and_bad_crumb_recovers() {
        // Worst case: both auth materials are garbage. get_ticker_info must
        // refresh the crumb, then get_crumb must refresh the cookie and only
        // then retry the summary request successfully.
        let mock = crate::mock_server::MockServer::start();
        mock.enqueue(401, &[], ""); // summary with bogus crumb
        queue_cookie(&mock); // get_cookie refresh inside get_crumb
        queue_crumb(&mock, "abc"); // fresh crumb
        queue_summary_success(&mock);

        let mut conn = mock_connector(&mock);
        conn.cookie = Some("garbage".to_string());
        conn.crumb = Some("bogus".to_string());

        let result = conn.get_ticker_info("AAPL");
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
        assert_eq!(conn.crumb.as_deref(), Some("abc"));

        let lines = mock.request_lines();
        assert_eq!(lines.len(), 4, "unexpected request sequence: {lines:?}");
        assert!(lines[0].contains("crumb=bogus"), "got: {}", lines[0]);
        assert_eq!(lines[1], "GET / HTTP/1.1");
        assert_eq!(lines[2], "GET /v1/test/getcrumb HTTP/1.1");
        assert!(lines[3].contains("crumb=abc"), "got: {}", lines[3]);
    }

    #[test]
    fn test_get_crumb_refreshes_cookie_on_invalid_cookie_body() {
        // Yahoo may also answer the crumb endpoint with a 200 body containing
        // "Invalid Cookie" instead of a 401 status; both must refresh.
        let mock = crate::mock_server::MockServer::start();
        mock.enqueue_plain(200, "Invalid Cookie");
        queue_cookie(&mock);
        queue_crumb(&mock, "abc");

        let mut conn = mock_connector(&mock);
        conn.cookie = Some("garbage".to_string());
        conn.crumb = None;

        let crumb = conn.get_crumb().unwrap();
        assert_eq!(crumb, "abc");
        let lines = mock.request_lines();
        assert_eq!(lines.len(), 3, "unexpected request sequence: {lines:?}");
        assert_eq!(lines[0], "GET /v1/test/getcrumb HTTP/1.1");
        assert_eq!(lines[1], "GET / HTTP/1.1");
        assert_eq!(lines[2], "GET /v1/test/getcrumb HTTP/1.1");
    }

    #[test]
    fn test_429_is_not_retried() {
        // A rate limit is definitive for this codebase: crumb must not be
        // refreshed (a fresh crumb does not lift the IP rate limit).
        let mock = crate::mock_server::MockServer::start();
        queue_cookie(&mock);
        queue_crumb(&mock, "abc");
        mock.enqueue(429, &[], "Too Many Requests");

        let mut conn = mock_connector(&mock);
        let result = conn.get_ticker_info("AAPL");

        assert!(matches!(result, Err(YahooError::TooManyRequests(_))));
        assert_eq!(conn.crumb.as_deref(), Some("abc"));
        let lines = mock.request_lines();
        assert_eq!(lines.len(), 3, "unexpected request sequence: {lines:?}");
        assert!(lines[2].contains("crumb=abc"), "got: {}", lines[2]);
    }

    #[test]
    fn test_get_cookie_success_without_status() {
        // Yahoo serves the cookie even on non-2xx pages; the cookie wins.
        let mock = crate::mock_server::MockServer::start();
        mock.enqueue(404, &[("Set-Cookie", MOCK_COOKIE)], "not found");

        let mut conn = mock_connector(&mock);
        let cookie = conn.get_cookie().unwrap();
        assert_eq!(cookie, MOCK_COOKIE);
        assert_eq!(mock.request_lines().len(), 1);
    }

    #[test]
    fn test_get_cookie_no_cookies_error() {
        // 200 without any Set-Cookie header -> NoCookies
        let mock = crate::mock_server::MockServer::start();
        mock.enqueue_plain(200, "");

        let mut conn = mock_connector(&mock);
        let result = conn.get_cookie();
        assert!(matches!(result, Err(YahooError::NoCookies)));
    }

    #[test]
    fn test_get_cookie_rate_limited() {
        // 429 without a cookie -> TooManyRequests
        let mock = crate::mock_server::MockServer::start();
        mock.enqueue(429, &[], "Too Many Requests");

        let mut conn = mock_connector(&mock);
        let result = conn.get_cookie();
        assert!(matches!(result, Err(YahooError::TooManyRequests(_))));
    }

    #[test]
    fn test_get_cookie_joins_multiple_set_cookie() {
        // Multiple Set-Cookie headers must all be collected (an A3 that is
        // not first must not be missed).
        let mock = crate::mock_server::MockServer::start();
        mock.enqueue(
            200,
            &[("Set-Cookie", "B=other"), ("Set-Cookie", "A3=mockcookie")],
            "",
        );

        let mut conn = mock_connector(&mock);
        let cookie = conn.get_cookie().unwrap();
        assert!(cookie.contains("A3=mockcookie"), "got: {}", cookie);
        assert!(cookie.contains("B=other"), "got: {}", cookie);
    }

    #[test]
    fn test_cookie_header_value_keeps_all_cookies() {
        // get_cookie collects every Set-Cookie into the stored value, and
        // cookie_header_value must pass the whole value through as the Cookie
        // header (an A3 that is not first must not be dropped).
        let mock = crate::mock_server::MockServer::start();
        mock.enqueue(
            200,
            &[("Set-Cookie", "B=other"), ("Set-Cookie", "A3=mockcookie")],
            "",
        );

        let mut conn = mock_connector(&mock);
        let cookie = conn.get_cookie().unwrap();
        assert_eq!(cookie, "B=other; A3=mockcookie", "get_cookie joins cookies");
        conn.cookie = Some(cookie);
        let header = conn.cookie_header_value();
        assert_eq!(
            header, "B=other; A3=mockcookie",
            "cookie_header_value keeps all"
        );
    }

    #[test]
    fn test_get_cookie_429_wins_over_set_cookie() {
        // A 429 rate limit is definitive: even if the error page set a
        // cookie, get_cookie must report TooManyRequests.
        let mock = crate::mock_server::MockServer::start();
        mock.enqueue(429, &[("Set-Cookie", "A3=mockcookie")], "Too Many Requests");

        let mut conn = mock_connector(&mock);
        let result = conn.get_cookie();
        assert!(matches!(result, Err(YahooError::TooManyRequests(_))));
    }

    #[test]
    fn test_get_crumb_empty_body_exhausts_retries() {
        // Two empty crumb bodies -> InvalidCrumb after refreshing the cookie
        // in between.
        let mock = crate::mock_server::MockServer::start();
        mock.enqueue_plain(200, "");
        queue_cookie(&mock);
        mock.enqueue_plain(200, "");

        let mut conn = mock_connector(&mock);
        conn.cookie = Some("garbage".to_string());
        conn.crumb = None;

        let result = conn.get_crumb();
        assert!(matches!(result, Err(YahooError::InvalidCrumb)));
        let lines = mock.request_lines();
        assert_eq!(lines.len(), 3, "unexpected request sequence: {lines:?}");
        assert_eq!(lines[0], "GET /v1/test/getcrumb HTTP/1.1");
        assert_eq!(lines[1], "GET / HTTP/1.1");
        assert_eq!(lines[2], "GET /v1/test/getcrumb HTTP/1.1");
    }

    #[test]
    fn test_get_crumb_too_many_requests_body_not_retried() {
        // A "Too Many Requests" body is a definitive rate limit: return
        // immediately, do not retry.
        let mock = crate::mock_server::MockServer::start();
        queue_cookie(&mock);
        mock.enqueue_plain(200, "Too Many Requests");

        let mut conn = mock_connector(&mock);
        let result = conn.get_crumb();
        assert!(matches!(result, Err(YahooError::TooManyRequests(_))));
        assert_eq!(mock.request_lines().len(), 2, "must not retry");
    }

    #[test]
    fn test_get_crumb_html_body_refreshes_cookie() {
        // A maintenance HTML page served with a 200 status is not a valid
        // crumb (yfinance checks `'<html>' in crumb`); it must be treated like
        // an empty crumb: refresh the cookie and retry.
        let mock = crate::mock_server::MockServer::start();
        mock.enqueue_plain(200, "<html><body>Will be right back</body></html>");
        queue_cookie(&mock);
        queue_crumb(&mock, "abc");

        let mut conn = mock_connector(&mock);
        conn.cookie = Some("garbage".to_string());
        conn.crumb = None;

        let crumb = conn.get_crumb().unwrap();
        assert_eq!(crumb, "abc");
        let lines = mock.request_lines();
        assert_eq!(lines.len(), 3, "unexpected request sequence: {lines:?}");
        assert_eq!(lines[0], "GET /v1/test/getcrumb HTTP/1.1");
        assert_eq!(lines[1], "GET / HTTP/1.1");
        assert_eq!(lines[2], "GET /v1/test/getcrumb HTTP/1.1");
    }

    #[test]
    fn test_get_crumb_unauthorized_on_exhausted_401() {
        // Two 401/403/404 responses on the crumb endpoint must surface as
        // Unauthorized (not FetchFailed) so callers can key on the variant.
        let mock = crate::mock_server::MockServer::start();
        mock.enqueue(401, &[], "");
        queue_cookie(&mock);
        mock.enqueue(401, &[], "");

        let mut conn = mock_connector(&mock);
        conn.cookie = Some("garbage".to_string());
        conn.crumb = None;

        let result = conn.get_crumb();
        assert!(matches!(result, Err(YahooError::Unauthorized)));
        let lines = mock.request_lines();
        assert_eq!(lines.len(), 3, "unexpected request sequence: {lines:?}");
        assert_eq!(lines[0], "GET /v1/test/getcrumb HTTP/1.1");
        assert_eq!(lines[1], "GET / HTTP/1.1");
        assert_eq!(lines[2], "GET /v1/test/getcrumb HTTP/1.1");
    }

    #[test]
    fn test_get_crumb_401_no_extra_cookie_on_last_attempt() {
        // "Invalid Cookie" on the final attempt must not issue a wasted
        // get_cookie request (the loop is about to terminate).
        let mock = crate::mock_server::MockServer::start();
        mock.enqueue_plain(200, "Invalid Cookie");
        queue_cookie(&mock);
        mock.enqueue_plain(200, "Invalid Cookie");

        let mut conn = mock_connector(&mock);
        conn.cookie = Some("mock".to_string());
        conn.crumb = None;

        let result = conn.get_crumb();
        assert!(matches!(result, Err(YahooError::InvalidCookie)));
        let lines = mock.request_lines();
        assert_eq!(lines.len(), 3, "unexpected request sequence: {lines:?}");
        assert_eq!(lines[0], "GET /v1/test/getcrumb HTTP/1.1");
        assert_eq!(lines[1], "GET / HTTP/1.1");
        assert_eq!(lines[2], "GET /v1/test/getcrumb HTTP/1.1");
    }

    #[test]
    fn test_mock_server_survives_half_open_connection() {
        // A client that connects and closes without sending a request must not
        // spin the mock's connection handler (EOF must terminate it).
        let mock = crate::mock_server::MockServer::start();
        {
            use std::io::Write;
            use std::net::TcpStream;
            let mut stream = TcpStream::connect(mock.addr()).unwrap();
            // Write a partial request then drop: the header loop sees EOF.
            let _ = stream.write_all(b"GET / HTTP/1.1\r\nHost: x\r\n");
        }
        // Give the handler a moment to (not) spin, then verify the server still
        // answers a normal request.
        std::thread::sleep(std::time::Duration::from_millis(50));
        queue_cookie(&mock);
        let mut conn = mock_connector(&mock);
        let cookie = conn.get_cookie().unwrap();
        assert_eq!(cookie, MOCK_COOKIE);
    }

    #[test]
    fn test_empty_quote_summary_returns_noresult() {
        let mock = crate::mock_server::MockServer::start();
        queue_cookie(&mock);
        queue_crumb(&mock, "abc");
        mock.enqueue_plain(200, "{}");

        let mut conn = mock_connector(&mock);
        let result = conn.get_ticker_info("AAPL");

        assert!(matches!(result, Err(YahooError::NoResult)));
        assert_eq!(mock.request_lines().len(), 3);
    }

    #[test]
    fn test_empty_symbol_rejected_without_requests() {
        let mock = crate::mock_server::MockServer::start();
        let mut conn = mock_connector(&mock);
        let result = conn.get_ticker_info("");

        assert!(matches!(result, Err(YahooError::FetchFailed(_))));
        assert!(mock.request_lines().is_empty());
    }

    // ------------------------------------------------------------------
    // Live error-injection tests against the real Yahoo API. These run in
    // CI like the other live tests; they deliberately send bad crumb/cookie
    // values to verify the recovery paths against Yahoo's actual behavior.
    // ------------------------------------------------------------------

    #[test]
    fn test_invalid_crumb_refreshes_and_succeeds() {
        let mut provider = YahooConnector::new().unwrap();
        // A bogus crumb must be rejected by Yahoo; get_ticker_info refreshes
        // the crumb and retries (Unauthorized / Invalid Crumb / API error).
        provider.crumb = Some("bogus".to_string());

        let result = provider.get_ticker_info("AAPL").unwrap();
        assert!(result.has_result());
    }

    #[test]
    fn test_invalid_cookie_refreshes_and_succeeds() {
        let mut provider = YahooConnector::new().unwrap();
        // A garbage cookie must trigger the "Invalid Cookie" refresh path in
        // get_crumb, which re-fetches the real cookie and succeeds.
        provider.cookie = Some("garbage".to_string());
        provider.crumb = None;

        let crumb = provider.get_crumb().unwrap();
        assert!(crumb.len() > 5);
        assert!(crumb.len() < 16);

        let result = provider.get_ticker_info("AAPL").unwrap();
        assert!(result.has_result());
    }

    #[test]
    fn test_unknown_symbol_error_shape() {
        let mut provider = YahooConnector::new().unwrap();
        let result = provider.get_ticker_info("ZZZZX");

        let err = result.expect_err("unknown symbol must produce an error");
        eprintln!("unknown symbol error: {:?}", err);
        // Any error variant is acceptable; the test documents what Yahoo
        // actually returns for a non-existent symbol.
    }

    #[test]
    fn test_option_symbol_parses() {
        let mut provider = YahooConnector::new().unwrap();
        // Fetch the live options chain and pick the nearest contract. The v7
        // options endpoint requires a valid crumb + cookie (like v10).
        let crumb = provider.get_crumb().unwrap();
        let url = format!(
            "https://query2.finance.yahoo.com/v7/finance/options/AAPL?crumb={}",
            crumb
        );
        let json: serde_json::Value = provider
            .client
            .get(url)
            .header("Cookie", provider.cookie_header_value())
            .send()
            .unwrap()
            .json()
            .unwrap();
        // Pick a contract from a later expiration group (options[0] expires
        // today and would be delisted after market close): scan from the end
        // for a group that still has contracts.
        let groups = json["optionChain"]["result"][0]["options"]
            .as_array()
            .map(|a| a.len())
            .unwrap_or(0);
        let mut index = groups.saturating_sub(1);
        while index > 0
            && json["optionChain"]["result"][0]["options"][index]["calls"]
                .as_array()
                .map_or(true, |c| c.is_empty())
        {
            index -= 1;
        }
        let contract = json["optionChain"]["result"][0]["options"][index]["calls"]
            .as_array()
            .and_then(|calls| calls.first())
            .and_then(|call| call["contractSymbol"].as_str())
            .expect("no option contracts returned for AAPL");
        eprintln!("testing option contract: {}", contract);

        let mut provider = YahooConnector::new().unwrap();
        let result = provider.get_ticker_info(contract).unwrap();
        let summary = result.quote_summary.unwrap();
        let data = summary
            .result
            .expect("option quoteSummary result")
            .remove(0);
        let detail = data.summary_detail.expect("summaryDetail module");
        if let Some(strike) = detail.strike_price {
            assert!(
                strike > 0.0,
                "fractional strike prices (e.g. 212.5) must deserialize as f64, got {}",
                strike
            );
        }
        // open_interest must parse with the tolerant Decimal deserializer.
        let _ = detail.open_interest;
    }

    #[test]
    fn test_crypto_null_trading_periods() {
        let mut provider = YahooConnector::new().unwrap();
        // Crypto symbols typically have null tradingPeriods/currentTradingPeriod;
        // these must fall back to defaults instead of panicking.
        let result = provider.get_ticker_info("BTC-USD").unwrap();
        assert!(result.has_result());
    }

    fn earnings_success_json() -> &'static str {
        r#"{
            "finance": {
                "result": [
                    {
                        "documents": [
                            {
                                "columns": [
                                    {"label": "Event Start Date"},
                                    {"label": "Timezone short name"},
                                    {"label": "EPS Estimate"},
                                    {"label": "Reported EPS"},
                                    {"label": "Surprise (%)"},
                                    {"label": "Event Type"}
                                ],
                                "rows": [
                                    ["2025-05-01T20:30:00.000Z", "EDT", 1.6, 1.7, 6.25, "2"]
                                ]
                            }
                        ]
                    }
                ],
                "error": null
            }
        }"#
    }

    #[test]
    fn test_financial_events_success_mock() {
        let mock = crate::mock_server::MockServer::start();
        queue_cookie(&mock);
        queue_crumb(&mock, "abc");
        mock.enqueue_plain(200, earnings_success_json());

        let mut conn = mock_connector(&mock);
        let events = conn.get_financial_events("AAPL", 25).unwrap();
        assert_eq!(events.len(), 1, "expected one earnings event");
        assert_eq!(events[0].event_type, "Earnings");
        assert_eq!(events[0].eps_estimate, Some(1.6));
        assert_eq!(events[0].reported_eps, Some(1.7));
        assert_eq!(events[0].timezone.as_deref(), Some("EDT"));
        let lines = mock.request_lines();
        assert_eq!(lines.len(), 3, "unexpected request sequence: {lines:?}");
        assert!(
            lines[2].starts_with("POST /v1/finance/visualization"),
            "got: {}",
            lines[2]
        );
    }

    #[test]
    fn test_financial_events_multiple_documents_aggregated() {
        // The schema allows multiple documents/results (one per event type);
        // all of them must be parsed, not just the first.
        let json = r#"{
            "finance": {
                "result": [
                    {
                        "documents": [
                            {
                                "columns": [{"label": "Event Start Date"}],
                                "rows": [["2025-05-01T20:30:00.000Z"]]
                            },
                            {
                                "columns": [{"label": "Event Start Date"}],
                                "rows": [["2025-05-02T20:30:00.000Z"]]
                            }
                        ]
                    },
                    {
                        "documents": [
                            {
                                "columns": [{"label": "Event Start Date"}],
                                "rows": [["2025-05-03T20:30:00.000Z"]]
                            }
                        ]
                    }
                ],
                "error": null
            }
        }"#;
        let mock = crate::mock_server::MockServer::start();
        queue_cookie(&mock);
        queue_crumb(&mock, "abc");
        mock.enqueue_plain(200, json);

        let mut conn = mock_connector(&mock);
        let events = conn.get_financial_events("AAPL", 25).unwrap();
        assert_eq!(events.len(), 3, "expected all documents aggregated");
        let lines = mock.request_lines();
        assert_eq!(lines.len(), 3, "unexpected request sequence: {lines:?}");
    }

    #[test]
    fn test_financial_events_401_retries() {
        // 401 -> refresh crumb and retry once.
        let mock = crate::mock_server::MockServer::start();
        queue_cookie(&mock);
        queue_crumb(&mock, "abc");
        mock.enqueue_plain(401, "unauthorized");
        queue_cookie(&mock);
        queue_crumb(&mock, "xyz");
        mock.enqueue_plain(200, earnings_success_json());

        let mut conn = mock_connector(&mock);
        let events = conn.get_financial_events("AAPL", 25).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(conn.crumb.as_deref(), Some("xyz"));
        let lines = mock.request_lines();
        assert_eq!(lines.len(), 6, "unexpected request sequence: {lines:?}");
    }

    #[test]
    fn test_financial_events_403_refreshes_crumb_and_cookie() {
        // 403 -> refresh crumb (get_crumb) AND cookie, then retry once.
        let mock = crate::mock_server::MockServer::start();
        queue_cookie(&mock);
        queue_crumb(&mock, "abc");
        mock.enqueue_plain(403, "forbidden");
        // On retry get_crumb runs with cookie=None, so it fetches a fresh
        // cookie first and then a fresh crumb.
        queue_cookie(&mock);
        queue_crumb(&mock, "xyz");
        mock.enqueue_plain(200, earnings_success_json());

        let mut conn = mock_connector(&mock);
        let events = conn.get_financial_events("AAPL", 25).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(conn.crumb.as_deref(), Some("xyz"));
        assert_eq!(conn.cookie.as_deref(), Some(MOCK_COOKIE));
        let lines = mock.request_lines();
        assert_eq!(lines.len(), 6, "unexpected request sequence: {lines:?}");
        assert_eq!(lines[0], "GET / HTTP/1.1");
        assert_eq!(lines[1], "GET /v1/test/getcrumb HTTP/1.1");
        assert!(
            lines[2].starts_with("POST /v1/finance/visualization"),
            "got: {}",
            lines[2]
        );
        assert_eq!(lines[3], "GET / HTTP/1.1");
        assert_eq!(lines[4], "GET /v1/test/getcrumb HTTP/1.1");
        assert!(
            lines[5].starts_with("POST /v1/finance/visualization"),
            "got: {}",
            lines[5]
        );
    }

    #[test]
    fn test_financial_events_500_retries_once() {
        // 5xx -> retry once with a fresh crumb, then succeed.
        let mock = crate::mock_server::MockServer::start();
        queue_cookie(&mock);
        queue_crumb(&mock, "abc");
        mock.enqueue_plain(500, "boom");
        queue_crumb(&mock, "xyz");
        mock.enqueue_plain(200, earnings_success_json());

        let mut conn = mock_connector(&mock);
        let events = conn.get_financial_events("AAPL", 25).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(conn.crumb.as_deref(), Some("xyz"));
        let lines = mock.request_lines();
        assert_eq!(lines.len(), 5, "unexpected request sequence: {lines:?}");
    }

    #[test]
    fn test_financial_events_429_not_retried() {
        // Rate limit is definitive for this codebase.
        let mock = crate::mock_server::MockServer::start();
        queue_cookie(&mock);
        queue_crumb(&mock, "abc");
        mock.enqueue_plain(429, "Too Many Requests");

        let mut conn = mock_connector(&mock);
        let result = conn.get_financial_events("AAPL", 25);
        assert!(matches!(result, Err(YahooError::TooManyRequests(_))));
        assert_eq!(mock.request_lines().len(), 3);
    }

    #[test]
    fn test_financial_events_api_error_invalid_crumb_exhausts() {
        // `finance.error` with "Invalid Crumb" -> retry, then InvalidCrumb.
        let mock = crate::mock_server::MockServer::start();
        queue_cookie(&mock);
        queue_crumb(&mock, "abc");
        mock.enqueue_plain(
            200,
            r#"{"finance":{"result":[],"error":{"code":"Invalid Crumb","description":"Invalid Crumb"}}}"#,
        );
        queue_crumb(&mock, "xyz");
        mock.enqueue_plain(
            200,
            r#"{"finance":{"result":[],"error":{"code":"Invalid Crumb","description":"Invalid Crumb"}}}"#,
        );

        let mut conn = mock_connector(&mock);
        let result = conn.get_financial_events("AAPL", 25);
        assert!(matches!(result, Err(YahooError::InvalidCrumb)));
        assert_eq!(conn.crumb.as_deref(), Some("xyz"));
        let lines = mock.request_lines();
        assert_eq!(lines.len(), 5, "unexpected request sequence: {lines:?}");
    }

    #[test]
    fn test_financial_events_api_error_reported() {
        // A non-crumb API error (e.g. unknown symbol) is reported to the caller.
        let mock = crate::mock_server::MockServer::start();
        queue_cookie(&mock);
        queue_crumb(&mock, "abc");
        mock.enqueue_plain(
            200,
            r#"{"finance":{"result":[],"error":{"code":"Not Found","description":"no such ticker"}}}"#,
        );

        let mut conn = mock_connector(&mock);
        let result = conn.get_financial_events("NOPE", 25);
        assert!(matches!(result, Err(YahooError::ApiError(_))));
        assert_eq!(mock.request_lines().len(), 3);
    }

    #[test]
    fn test_financial_events_parse_error_refreshes_crumb_and_cookie() {
        // A parse error (HTML/empty body) refreshes crumb AND cookie, retries once.
        let mock = crate::mock_server::MockServer::start();
        queue_cookie(&mock);
        queue_crumb(&mock, "abc");
        mock.enqueue_plain(200, "<html>not json</html>");
        // On retry get_crumb runs with cookie=None, so it fetches a fresh
        // cookie first and then a fresh crumb.
        queue_cookie(&mock);
        queue_crumb(&mock, "xyz");
        mock.enqueue_plain(200, earnings_success_json());

        let mut conn = mock_connector(&mock);
        let events = conn.get_financial_events("AAPL", 25).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(conn.crumb.as_deref(), Some("xyz"));
        assert_eq!(conn.cookie.as_deref(), Some(MOCK_COOKIE));
        let lines = mock.request_lines();
        assert_eq!(lines.len(), 6, "unexpected request sequence: {lines:?}");
        assert!(
            lines[5].starts_with("POST /v1/finance/visualization"),
            "got: {}",
            lines[5]
        );
    }

    fn chart_success_json() -> &'static str {
        r#"{
            "chart": {
                "result": [
                    {
                        "meta": {
                            "currency": "USD",
                            "symbol": "TEST",
                            "instrumentType": "EQUITY",
                            "exchangeName": "NMS",
                            "fullExchangeName": "Nasdaq",
                            "gmtoffset": -14400,
                            "timezone": "EDT",
                            "exchangeTimezoneName": "America/New_York",
                            "hasPrePostMarketData": false,
                            "priceHint": 2,
                            "currentTradingPeriod": null,
                            "dataGranularity": "1d",
                            "range": "5d",
                            "validRanges": ["1d", "5d", "1mo"]
                        },
                        "timestamp": [1000, 2000],
                        "events": null,
                        "indicators": {
                            "quote": [
                                {
                                    "open": [10.0, 11.0],
                                    "high": [12.0, 13.0],
                                    "low": [9.0, 10.0],
                                    "close": [10.5, 11.5],
                                    "volume": [100, 200]
                                }
                            ]
                        }
                    }
                ],
                "error": null
            }
        }"#
    }

    #[test]
    fn test_chart_retry_on_html_response() {
        // A maintenance HTML page is transient; retry once and succeed.
        let mock = crate::mock_server::MockServer::start();
        mock.enqueue_plain(
            200,
            "<!DOCTYPE html><html><body>Will be right back</body></html>",
        );
        mock.enqueue_plain(200, chart_success_json());

        let conn = mock_connector(&mock);
        let result = conn.get_quote_range("TEST", "1d", "5d");
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
        assert_eq!(mock.request_lines().len(), 2);
    }

    #[test]
    fn test_chart_retry_on_empty_response() {
        // An empty body is transient; retry once and succeed.
        let mock = crate::mock_server::MockServer::start();
        mock.enqueue_plain(200, "");
        mock.enqueue_plain(200, chart_success_json());

        let conn = mock_connector(&mock);
        let result = conn.get_quote_range("TEST", "1d", "5d");
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
        assert_eq!(mock.request_lines().len(), 2);
    }

    #[test]
    fn test_chart_retry_on_server_error() {
        // A 5xx is transient; retry once and succeed.
        let mock = crate::mock_server::MockServer::start();
        mock.enqueue_plain(500, "Internal Server Error");
        mock.enqueue_plain(200, chart_success_json());

        let conn = mock_connector(&mock);
        let result = conn.get_quote_range("TEST", "1d", "5d");
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
        assert_eq!(mock.request_lines().len(), 2);
    }

    #[test]
    fn test_chart_retry_exhausted() {
        // Two consecutive HTML pages exhaust the retry; report the error.
        let mock = crate::mock_server::MockServer::start();
        mock.enqueue_plain(
            200,
            "<!DOCTYPE html><html><body>Will be right back</body></html>",
        );
        mock.enqueue_plain(
            200,
            "<!DOCTYPE html><html><body>Will be right back</body></html>",
        );

        let conn = mock_connector(&mock);
        let result = conn.get_quote_range("TEST", "1d", "5d");
        assert!(matches!(result, Err(YahooError::HtmlResponse)));
        assert_eq!(mock.request_lines().len(), 2);
    }
}
