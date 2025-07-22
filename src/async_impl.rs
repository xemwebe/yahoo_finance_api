use crate::quotes::{FinancialEvent, YEarningsResponse, YErrorMessage};

use super::*;

impl YahooConnector {
    /// Retrieve the quotes of the last day for the given ticker
    pub async fn get_latest_quotes(
        &self,
        ticker: &str,
        interval: &str,
    ) -> Result<YResponse, YahooError> {
        self.get_quote_range(ticker, interval, "1mo").await
    }

    /// Retrieve the quote history for the given ticker form date start to end (inclusive), if available
    pub async fn get_quote_history(
        &self,
        ticker: &str,
        start: OffsetDateTime,
        end: OffsetDateTime,
    ) -> Result<YResponse, YahooError> {
        self.get_quote_history_interval(ticker, start, end, "1d")
            .await
    }

    /// Retrieve quotes for the given ticker for an arbitrary range
    pub async fn get_quote_range(
        &self,
        ticker: &str,
        interval: &str,
        range: &str,
    ) -> Result<YResponse, YahooError> {
        let url: String = format!(
            YCHART_RANGE_QUERY!(),
            url = self.url,
            symbol = ticker,
            interval = interval,
            range = range
        );
        YResponse::from_json(self.send_request(&url).await?)?.map_error_msg()
    }

    /// Retrieve the quote history for the given ticker form date start to end (inclusive), if available; specifying the interval of the ticker.
    pub async fn get_quote_history_interval(
        &self,
        ticker: &str,
        start: OffsetDateTime,
        end: OffsetDateTime,
        interval: &str,
    ) -> Result<YResponse, YahooError> {
        let url = format!(
            YCHART_PERIOD_QUERY!(),
            url = self.url,
            symbol = ticker,
            start = start.unix_timestamp(),
            end = end.unix_timestamp(),
            interval = interval,
        );
        YResponse::from_json(self.send_request(&url).await?)?.map_error_msg()
    }

    /// Retrieve the quote history for the given ticker form date start to end (inclusive) and optionally before and after regular trading hours, if available; specifying the interval of the ticker.
    pub async fn get_quote_history_interval_prepost(
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
            symbol = ticker,
            start = start.unix_timestamp(),
            end = end.unix_timestamp(),
            interval = interval,
            prepost = prepost,
        );
        YResponse::from_json(self.send_request(&url).await?)?.map_error_msg()
    }

    /// Retrieve the quote history for the given ticker for a given period and ticker interval and optionally before and after regular trading hours
    pub async fn get_quote_period_interval(
        &self,
        ticker: &str,
        range: &str,
        interval: &str,
        prepost: bool,
    ) -> Result<YResponse, YahooError> {
        let url = format!(
            YCHART_PERIOD_INTERVAL_QUERY!(),
            url = self.url,
            symbol = ticker,
            range = range,
            interval = interval,
            prepost = prepost,
        );
        YResponse::from_json(self.send_request(&url).await?)?.map_error_msg()
    }

    /// Retrieve the list of quotes found searching a given name
    pub async fn search_ticker_opt(&self, name: &str) -> Result<YSearchResultOpt, YahooError> {
        let url = format!(YTICKER_QUERY!(), url = self.search_url, name = name);
        YSearchResultOpt::from_json(self.send_request(&url).await?)
    }

    /// Retrieve the list of quotes found searching a given name
    pub async fn search_ticker(&self, name: &str) -> Result<YSearchResult, YahooError> {
        let result = self.search_ticker_opt(name).await?;
        Ok(YSearchResult::from_opt(&result))
    }

    // Get symbol metadata
    pub async fn get_ticker_info(&mut self, symbol: &str) -> Result<YQuoteSummary, YahooError> {
        if self.crumb.is_none() {
            self.crumb = Some(self.get_crumb().await?);
        }
        if self.cookie.is_none() {
            self.cookie = Some(self.get_cookie().await?);
        }

        let cookie_provider = Arc::new(reqwest::cookie::Jar::default());
        let url = reqwest::Url::parse(
            &(format!(
                YQUOTE_SUMMARY_QUERY!(),
                symbol = symbol,
                crumb = self.crumb.as_ref().unwrap()
            )),
        );

        cookie_provider.add_cookie_str(&self.cookie.clone().unwrap(), &url.clone().unwrap());

        let max_retries = 1;
        for i in 0..=max_retries {
            let text = self
                .create_client(Some(cookie_provider.clone()))
                .await?
                .get(url.clone().unwrap())
                .send()
                .await?
                .text()
                .await?;

            let result: YQuoteSummary = serde_json::from_str(&text)?;

            if let Some(finance) = &result.finance {
                if let Some(error) = &finance.error {
                    if let Some(description) = &error.description {
                        if description.contains("Invalid Crumb") {
                            self.crumb = Some(self.get_crumb().await?);
                            if i == max_retries {
                                return Err(YahooError::InvalidCrumb);
                            } else {
                                continue;
                            }
                        }
                    }
                    if let Some(code) = &error.code {
                        if code.contains("Unauthorized") {
                            self.crumb = Some(self.get_crumb().await?);
                            if i == max_retries {
                                return Err(YahooError::Unauthorized);
                            } else {
                                continue;
                            }
                        }
                    }
                }
            }
            return Ok(result);
        }

        Err(YahooError::NoResponse)
    }

    /// Retrieve financial events(Earnings, Meeting, Call) dates for the given ticker with specified limit (max limit: 250),
    pub async fn get_financial_events(
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
            self.crumb = Some(self.get_crumb().await?);
        }
        if self.cookie.is_none() {
            self.cookie = Some(self.get_cookie().await?);
        }

        let url = format!(
            YEARNINGS_QUERY!(),
            url = Y_EARNINGS_URL,
            lang = "en-US",
            region = "US",
            crumb = self.crumb.as_ref().unwrap()
        );

        // Create request body
        let query_body = serde_json::json!({
            "size": limit,
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
        let cookie_provider = Arc::new(reqwest::cookie::Jar::default());
        let parsed_url = reqwest::Url::parse(&url).map_err(|_| YahooError::InvalidUrl)?;

        if let Some(cookie) = &self.cookie {
            cookie_provider.add_cookie_str(cookie, &parsed_url);
        }

        let max_retries = 1;
        for attempt in 0..=max_retries {
            let client = self.create_client(Some(cookie_provider.clone())).await?;

            let response = client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&query_body)
                .send()
                .await?;

            let status = response.status();

            match status {
                reqwest::StatusCode::TOO_MANY_REQUESTS => {
                    return Err(YahooError::TooManyRequests(format!(
                        "POST {} in get_financial_events for ticker {}",
                        Y_EARNINGS_URL, ticker
                    )));
                }
                reqwest::StatusCode::UNAUTHORIZED => {
                    if attempt < max_retries {
                        self.crumb = Some(self.get_crumb().await?);
                        continue;
                    } else {
                        return Err(YahooError::Unauthorized);
                    }
                }
                reqwest::StatusCode::FORBIDDEN => {
                    return Err(YahooError::Unauthorized);
                }
                reqwest::StatusCode::NOT_FOUND => {
                    return Err(YahooError::FetchFailed(format!(
                        "Ticker {} not found",
                        ticker
                    )));
                }
                _ if !status.is_success() => {
                    return Err(YahooError::FetchFailed(format!("HTTP error: {}", status)));
                }
                _ => {} // Success, continue
            }

            let text = response.text().await?;

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
                        if description.contains("Invalid Crumb") {
                            if attempt < max_retries {
                                self.crumb = Some(self.get_crumb().await?); // Refetch crumb
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

                    return Ok(self.parse_earnings_response(earnings_response)?);
                }
                Err(e) => {
                    // A parsing error is a critical failure unless we are retrying.
                    if attempt < max_retries {
                        // It's possible the session expired, let's try refreshing the crumb and cookie.
                        self.crumb = Some(self.get_crumb().await?);
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

        let result = &response.finance.result[0];
        if result.documents.is_empty() {
            return Ok(earnings_events);
        }

        let document = &result.documents[0];

        if document.columns.is_empty() {
            return Err(YahooError::DataInconsistency);
        }

        // Map column names to indices
        let mut column_map = std::collections::HashMap::new();
        for (index, column) in document.columns.iter().enumerate() {
            column_map.insert(column.label.as_str(), index);
        }

        // Parse each row
        for row in &document.rows {
            let earnings_event = self.parse_earnings_row(row, &column_map)?;
            earnings_events.push(earnings_event);
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

    /// Get only earnings events (filter out meetings)
    pub async fn get_earnings_only(
        &mut self,
        ticker: &str,
        limit: u32,
    ) -> Result<Vec<FinancialEvent>, YahooError> {
        let all_events = self.get_financial_events(ticker, limit).await?;

        Ok(all_events
            .into_iter()
            .filter(|event| event.event_type == "Earnings")
            .collect())
    }

    async fn get_crumb(&mut self) -> Result<String, YahooError> {
        if self.cookie.is_none() {
            self.cookie = Some(self.get_cookie().await?);
        }

        const MAX_RETRIES: usize = 1;
        let crumb_url = reqwest::Url::parse(Y_GET_CRUMB_URL).unwrap();
        let mut last_error = YahooError::NoResponse;

        for _attempt in 0..=MAX_RETRIES {
            let cookie_provider = Arc::new(reqwest::cookie::Jar::default());
            cookie_provider.add_cookie_str(&self.cookie.clone().unwrap(), &crumb_url);

            let response = self
                .create_client(Some(cookie_provider.clone()))
                .await?
                .get(crumb_url.clone())
                .send()
                .await?;

            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                return Err(YahooError::TooManyRequests(format!(
                    "GET {} in get_crumb",
                    Y_GET_CRUMB_URL
                )));
            }
            let crumb = response.text().await?;
            let crumb = crumb.trim();

            if crumb.contains("Invalid Cookie") {
                self.cookie = Some(self.get_cookie().await?);
                last_error = YahooError::InvalidCookie;
                continue;
            }

            if crumb.contains("Too Many Requests") {
                last_error =
                    YahooError::TooManyRequests(format!("GET {} in get_crumb", Y_GET_CRUMB_URL));
                continue;
            }

            if crumb.is_empty() {
                last_error = YahooError::InvalidCrumb;
                continue;
            }

            return Ok(crumb.to_string());
        }

        Err(last_error)
    }

    async fn get_cookie(&mut self) -> Result<String, YahooError> {
        Ok(self
            .client
            .get(Y_GET_COOKIE_URL)
            .send()
            .await?
            .headers()
            .get(Y_COOKIE_REQUEST_HEADER)
            .ok_or(YahooError::NoCookies)?
            .to_str()
            .map_err(|_| YahooError::InvisibleAsciiInCookies)?
            .to_string())
    }

    async fn create_client(
        &self,
        cookie_provider: Option<Arc<reqwest::cookie::Jar>>,
    ) -> Result<Client, reqwest::Error> {
        let mut client_builder = Client::builder();

        if let Some(cookie_provider) = cookie_provider {
            client_builder = client_builder.cookie_provider(cookie_provider);
        }
        if let Some(timeout) = &self.timeout {
            client_builder = client_builder.timeout(*timeout);
        }
        if let Some(user_agent) = &self.user_agent {
            client_builder = client_builder.user_agent(user_agent.clone());
        }
        if let Some(proxy) = &self.proxy {
            client_builder = client_builder.proxy(proxy.clone());
        }

        client_builder.build()
    }

    /// Send request to yahoo! finance server and transform response to JSON value
    async fn send_request(&self, url: &str) -> Result<serde_json::Value, YahooError> {
        let response = self.client.get(url).send().await?.text().await?;

        let json = serde_json::from_str::<serde_json::Value>(&response)
            .map_err(YahooError::DeserializeFailed);

        if json.is_err() {
            let trimmed_response = response.trim();
            if trimmed_response.len() <= 4_000
                && trimmed_response
                    .to_lowercase()
                    .contains("too many requests")
            {
                Err(YahooError::TooManyRequests(format!("request url: {}", url)))?
            } else {
                #[cfg(feature = "debug")]
                Err(YahooError::DeserializeFailedDebug(
                    trimmed_response.to_string(),
                ))?
            }
        }

        json
    }
}

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use super::*;

    #[test]
    fn test_get_single_quote() {
        let provider = YahooConnector::new().unwrap();
        let response = tokio_test::block_on(provider.get_latest_quotes("HNL.DE", "1d")).unwrap();

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

        let response = tokio_test::block_on(provider.get_quote_history("IBM", start, end)).unwrap();
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
        let response = tokio_test::block_on(provider.get_latest_quotes("BF.B", "1m")).unwrap();
        let result = &response.chart.result.as_ref().unwrap();

        assert_eq!(&result[0].meta.symbol, "BF.B");
        let _ = response.last_quote().unwrap();
    }

    #[test]
    fn test_get_quote_history() {
        let provider = YahooConnector::new().unwrap();

        let start = datetime!(2020-01-01 0:00:00.00 UTC);
        let end = datetime!(2020-01-31 23:59:59.99 UTC);

        let response = tokio_test::block_on(provider.get_quote_history("AAPL", start, end));

        if response.is_ok() {
            let response = response.unwrap();
            let result = &response.chart.result.as_ref().unwrap();
            assert_eq!(result[0].timestamp.as_ref().unwrap().len(), 21);

            let quotes = response.quotes().unwrap();
            assert_eq!(quotes.len(), 21);
        }
    }

    #[test]
    fn test_get_quote_range() {
        let provider = YahooConnector::new().unwrap();
        let response =
            tokio_test::block_on(provider.get_quote_range("HNL.DE", "1d", "1mo")).unwrap();
        let result = &response.chart.result.as_ref().unwrap();

        assert_eq!(&result[0].meta.symbol, "HNL.DE");
        assert_eq!(&result[0].meta.range, "1mo");
        assert_eq!(&result[0].meta.data_granularity, "1d");
        let _ = response.last_quote().unwrap();
    }

    #[test]
    fn test_get_metadata() {
        let provider = YahooConnector::new().unwrap();
        let response =
            tokio_test::block_on(provider.get_quote_range("HNL.DE", "1d", "1mo")).unwrap();
        let metadata = response.metadata().unwrap();
        assert_eq!(metadata.symbol, "HNL.DE");
    }

    #[test]
    fn test_get_quote_history_interval() {
        let provider = YahooConnector::new().unwrap();

        let start = datetime!(2019-01-01 0:00:00.00 UTC);
        let end = datetime!(2020-01-31 23:59:59.99 UTC);

        let response =
            tokio_test::block_on(provider.get_quote_history_interval("AAPL", start, end, "1mo"))
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

        let _ = tokio_test::block_on(
            provider.get_quote_history_interval_prepost(ticker, start, end, interval, prepost),
        )
        .unwrap();
    }

    #[test]
    fn test_get_quote_period_interval() {
        let provider = YahooConnector::new().unwrap();

        let range = "5d";
        let interval = "5m";

        let response = tokio_test::block_on(
            provider.get_quote_period_interval("AAPL", &range, &interval, true),
        )
        .unwrap();

        let metadata = response.metadata().unwrap();

        assert_eq!(metadata.data_granularity, interval);
        assert_eq!(metadata.range, range);
    }

    #[test]
    fn test_large_volume() {
        let provider = YahooConnector::new().unwrap();
        let response =
            tokio_test::block_on(provider.get_quote_range("BTC-USD", "1d", "5d")).unwrap();
        let quotes = response.quotes().unwrap();
        assert!(quotes.len() > 0usize);
    }

    #[test]
    fn test_search_ticker() {
        let provider = YahooConnector::new().unwrap();
        let response = tokio_test::block_on(provider.search_ticker("Apple")).unwrap();

        assert_eq!(response.count, 15);
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

        let response = tokio_test::block_on(provider.get_quote_history("VTSAX", start, end));

        if response.is_ok() {
            let response = response.unwrap();
            let result = &response.chart.result.as_ref().unwrap();

            assert_eq!(result[0].timestamp.as_ref().unwrap().len(), 21);

            let quotes = response.quotes().unwrap();
            assert_eq!(quotes.len(), 21);
        }
    }

    #[test]
    fn test_mutual_fund_latest() {
        let provider = YahooConnector::new().unwrap();
        let response = tokio_test::block_on(provider.get_latest_quotes("VTSAX", "1d")).unwrap();
        let result = &response.chart.result.as_ref().unwrap();

        assert_eq!(&result[0].meta.symbol, "VTSAX");
        assert_eq!(&result[0].meta.range, "1mo");
        assert_eq!(&result[0].meta.data_granularity, "1d");
        let _ = response.last_quote().unwrap();
    }

    #[test]
    fn test_mutual_fund_latest_with_null_first_trade_date() {
        let provider = YahooConnector::new().unwrap();
        let response = tokio_test::block_on(provider.get_latest_quotes("SIWA.F", "1d")).unwrap();
        let result = &response.chart.result.as_ref().unwrap();

        assert_eq!(&result[0].meta.symbol, "SIWA.F");
        assert_eq!(&result[0].meta.range, "1mo");
        assert_eq!(&result[0].meta.data_granularity, "1d");
        let _ = response.last_quote().unwrap();
    }

    #[test]
    fn test_mutual_fund_range() {
        let provider = YahooConnector::new().unwrap();
        let response =
            tokio_test::block_on(provider.get_quote_range("VTSAX", "1d", "1mo")).unwrap();
        let result = &response.chart.result.as_ref().unwrap();

        assert_eq!(&result[0].meta.symbol, "VTSAX");
        assert_eq!(&result[0].meta.range, "1mo");
        assert_eq!(&result[0].meta.data_granularity, "1d");
    }

    #[test]
    fn test_mutual_fund_capital_gains() {
        let provider = YahooConnector::new().unwrap();
        let response = tokio_test::block_on(provider.get_quote_range("AMAGX", "1d", "5y")).unwrap();
        let result = &response.chart.result.as_ref().unwrap();

        assert_eq!(&result[0].meta.symbol, "AMAGX");
        assert_eq!(&result[0].meta.range, "5y");
        assert_eq!(&result[0].meta.data_granularity, "1d");
        let capital_gains = response.capital_gains().unwrap();
        assert!(capital_gains.len() > 0usize);
    }

    #[test]
    fn test_get_ticker_info() {
        let mut provider = YahooConnector::new().unwrap();

        let result = tokio_test::block_on(provider.get_ticker_info("AAPL"));

        let quote_summary = result.unwrap().quote_summary.unwrap();
        assert!(
            "Cupertino"
                == quote_summary.result.as_ref().unwrap()[0]
                    .asset_profile
                    .as_ref()
                    .unwrap()
                    .city
                    .as_ref()
                    .unwrap()
        );
    }

    #[tokio::test]
    async fn test_get_crumb() {
        let mut provider = YahooConnector::new().unwrap();
        let crumb = provider.get_crumb().await.unwrap();

        assert!(crumb.len() > 5);
        assert!(crumb.len() < 16);
    }

    #[tokio::test]
    async fn test_get_cookie() {
        let mut provider = YahooConnector::new().unwrap();
        let cookie = provider.get_cookie().await.unwrap();

        assert!(cookie.len() > 30);
        assert!(
            cookie.contains("Expires")
                || cookie.contains("Max-Age")
                || cookie.contains("Domain")
                || cookie.contains("Path")
                || cookie.contains("Secure")
        );
    }

    #[tokio::test]
    async fn test_neg_time_stamp() {
        let start = datetime!(1960-01-01 0:00:00.00 UTC);
        let end = datetime!(2025-04-30 23:59:59.99 UTC);

        let provider = YahooConnector::new().unwrap();
        let response = provider.get_quote_history("XOM", start, end).await.unwrap();
        let quotes = response.quotes();
        assert!(!quotes.is_err());
        let quotes = quotes.unwrap();
        assert_eq!(quotes.len(), 15939);
    }

    #[test]
    fn test_get_financial_events() {
        let mut provider = YahooConnector::new().unwrap();
        let limit = 100;

        let result = tokio_test::block_on(provider.get_financial_events("AAPL", limit));

        if result.is_err() {
            println!("{:?}", result);
        }

        assert!(result.is_ok());
        let earnings = result.unwrap();

        assert_eq!(earnings.len() as u32, limit);
    }

    #[test]
    fn test_get_earnings_only() {
        let mut provider = YahooConnector::new().unwrap();
        let result = tokio_test::block_on(provider.get_earnings_only("AAPL", 100));

        assert!(result.is_ok());
        let earnings = result.unwrap();

        // All events should be earnings type
        for event in &earnings {
            assert_eq!(event.event_type, "Earnings");
        }
    }
}
