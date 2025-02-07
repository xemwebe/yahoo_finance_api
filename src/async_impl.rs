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
        YResponse::from_json(self.send_request(&url).await?)
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
        YResponse::from_json(self.send_request(&url).await?)
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
        YResponse::from_json(self.send_request(&url).await?)
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
            period = range,
            interval = interval,
            prepost = prepost,
        );
        YResponse::from_json(self.send_request(&url).await?)
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

    /// Get list for options for a given name
    pub async fn search_options(&self, name: &str) -> Result<YOptionChain, YahooError> {
        let url = format!("https://query2.finance.yahoo.com/v6/finance/options/{name}");
        let resp = self.client.get(url).send().await?;
        let resp = resp.json::<YOptionChain>().await?;

        Ok(resp)
    }

    // Get symbol metadata
    pub async fn get_ticker_info(&mut self, symbol: &str) -> Result<YQuoteSummary, YahooError> {
        if let None = &self.crumb {
            self.crumb = Some(self.get_crumb().await?);
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

        let mut result: Result<YQuoteSummary, YahooError> = Err(YahooError::NoResponse);

        let max_retries = 1;
        for i in 0..=max_retries {
            result = Ok(self
                .create_client(Some(cookie_provider.clone()))
                .await?
                .get(url.clone().unwrap())
                .send()
                .await?
                .json()
                .await?);

            if let Ok(result) = &result {
                if let Some(finance) = &result.finance {
                    if let Some(error) = &finance.error {
                        if let Some(description) = &error.description {
                            if description.contains("Invalid Crumb") {
                                self.crumb = Some(self.get_crumb().await?);
                                if i == max_retries {
                                    return Err(YahooError::InvalidCrumb);
                                }
                            }
                        }
                        if let Some(code) = &error.code {
                            if code.contains("Unauthorized") {
                                println!("Unauthorized {:?}", i);
                                self.crumb = Some(self.get_crumb().await?);
                                if i == max_retries {
                                    return Err(YahooError::Unauthorized);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Ok(result)
        result
    }

    async fn get_crumb(&mut self) -> Result<String, YahooError> {
        if let None = &self.cookie {
            self.cookie = Some(self.get_cookie().await?);
        }

        let cookie_provider = Arc::new(reqwest::cookie::Jar::default());
        cookie_provider.add_cookie_str(
            &self.cookie.clone().unwrap(),
            &reqwest::Url::parse(Y_GET_CRUMB_URL).unwrap(),
        );

        let mut result = Err(YahooError::NoResponse);

        let max_retries = 1;
        for i in 0..=max_retries {
            result = Ok(self
                .create_client(Some(cookie_provider.clone()))
                .await?
                .get(Y_GET_CRUMB_URL)
                .send()
                .await?
                .text()
                .await?);

            if let Ok(result) = &result {
                if result.contains("Invalid Cookie") {
                    self.cookie = Some(self.get_cookie().await?);
                    if i == max_retries {
                        return Err(YahooError::InvalidCookie);
                    }
                }
            }
        }

        // Ok(result)
        result
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
        &mut self,
        cookie_provider: Option<Arc<reqwest::cookie::Jar>>,
    ) -> Result<Client, reqwest::Error> {
        let mut client_builder = Client::builder();

        if let Some(cookie_provider) = cookie_provider {
            client_builder = client_builder.cookie_provider(cookie_provider);
        }
        if let Some(timeout) = &self.timeout {
            client_builder = client_builder.timeout(timeout.clone());
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
        let resp = self.client.get(url).send().await?;

        match resp.status() {
            StatusCode::OK => Ok(resp.json().await?),
            status => Err(YahooError::FetchFailed(format!("{}", status))),
        }
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
        assert_eq!(&response.chart.result[0].meta.symbol, "HNL.DE");
        assert_eq!(&response.chart.result[0].meta.range, "1mo");
        assert_eq!(&response.chart.result[0].meta.data_granularity, "1d");
        let _ = response.last_quote().unwrap();
    }

    #[test]
    fn test_strange_api_responses() {
        let provider = YahooConnector::new().unwrap();

        let start = datetime!(2019-07-03 0:00:00.00 UTC);
        let end = datetime!(2020-07-04 23:59:59.99 UTC);

        let resp = tokio_test::block_on(provider.get_quote_history("IBM", start, end)).unwrap();

        assert_eq!(&resp.chart.result[0].meta.symbol, "IBM");
        assert_eq!(&resp.chart.result[0].meta.data_granularity, "1d");
        assert_eq!(
            &resp.chart.result[0].meta.first_trade_date,
            &Some(-252322200)
        );

        let _ = resp.last_quote().unwrap();
    }

    #[test]
    #[should_panic(expected = "DeserializeFailed")]
    fn test_api_responses_missing_fields() {
        let provider = YahooConnector::new().unwrap();
        let response = tokio_test::block_on(provider.get_latest_quotes("BF.B", "1m")).unwrap();

        assert_eq!(&response.chart.result[0].meta.symbol, "BF.B");
        assert_eq!(&response.chart.result[0].meta.range, "1d");
        assert_eq!(&response.chart.result[0].meta.data_granularity, "1m");
        let _ = response.last_quote().unwrap();
    }

    #[test]
    fn test_get_quote_history() {
        let provider = YahooConnector::new().unwrap();

        let start = datetime!(2020-01-01 0:00:00.00 UTC);
        let end = datetime!(2020-01-31 23:59:59.99 UTC);

        let resp = tokio_test::block_on(provider.get_quote_history("AAPL", start, end));
        if resp.is_ok() {
            let resp = resp.unwrap();
            assert_eq!(resp.chart.result[0].timestamp.len(), 21);
            let quotes = resp.quotes().unwrap();
            assert_eq!(quotes.len(), 21);
        }
    }

    #[test]
    fn test_get_quote_range() {
        let provider = YahooConnector::new().unwrap();
        let response =
            tokio_test::block_on(provider.get_quote_range("HNL.DE", "1d", "1mo")).unwrap();
        assert_eq!(&response.chart.result[0].meta.symbol, "HNL.DE");
        assert_eq!(&response.chart.result[0].meta.range, "1mo");
        assert_eq!(&response.chart.result[0].meta.data_granularity, "1d");
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
    fn test_get() {
        let provider = YahooConnector::new().unwrap();

        let start = datetime!(2019-01-01 0:00:00.00 UTC);
        let end = datetime!(2020-01-31 23:59:59.99 UTC);

        let response =
            tokio_test::block_on(provider.get_quote_history_interval("AAPL", start, end, "1mo"))
                .unwrap();
        assert_eq!(&response.chart.result[0].timestamp.len(), &13);
        assert_eq!(&response.chart.result[0].meta.data_granularity, "1mo");
        let quotes = response.quotes().unwrap();
        assert_eq!(quotes.len(), 13usize);
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
        let resp = tokio_test::block_on(provider.search_ticker("Apple")).unwrap();

        assert_eq!(resp.count, 15);
        let mut apple_found = false;
        for item in resp.quotes {
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

        let resp = tokio_test::block_on(provider.get_quote_history("VTSAX", start, end));
        if resp.is_ok() {
            let resp = resp.unwrap();
            assert_eq!(resp.chart.result[0].timestamp.len(), 21);
            let quotes = resp.quotes().unwrap();
            assert_eq!(quotes.len(), 21);
        }
    }

    #[test]
    fn test_mutual_fund_latest() {
        let provider = YahooConnector::new().unwrap();
        let response = tokio_test::block_on(provider.get_latest_quotes("VTSAX", "1d")).unwrap();

        assert_eq!(&response.chart.result[0].meta.symbol, "VTSAX");
        assert_eq!(&response.chart.result[0].meta.range, "1mo");
        assert_eq!(&response.chart.result[0].meta.data_granularity, "1d");
        let _ = response.last_quote().unwrap();
    }

    #[test]
    fn test_mutual_fund_latest_with_null_first_trade_date() {
        let provider = YahooConnector::new().unwrap();
        let response = tokio_test::block_on(provider.get_latest_quotes("SIWA.F", "1d")).unwrap();

        assert_eq!(&response.chart.result[0].meta.symbol, "SIWA.F");
        assert_eq!(&response.chart.result[0].meta.range, "1mo");
        assert_eq!(&response.chart.result[0].meta.data_granularity, "1d");
        let _ = response.last_quote().unwrap();
    }

    #[test]
    fn test_mutual_fund_range() {
        let provider = YahooConnector::new().unwrap();
        let response =
            tokio_test::block_on(provider.get_quote_range("VTSAX", "1d", "1mo")).unwrap();
        assert_eq!(&response.chart.result[0].meta.symbol, "VTSAX");
        assert_eq!(&response.chart.result[0].meta.range, "1mo");
        assert_eq!(&response.chart.result[0].meta.data_granularity, "1d");
    }

    #[test]
    fn test_mutual_fund_capital_gains() {
        let provider = YahooConnector::new().unwrap();
        let response = tokio_test::block_on(provider.get_quote_range("AMAGX", "1d", "5y")).unwrap();

        assert_eq!(&response.chart.result[0].meta.symbol, "AMAGX");
        assert_eq!(&response.chart.result[0].meta.range, "5y");
        assert_eq!(&response.chart.result[0].meta.data_granularity, "1d");
        let capital_gains = response.capital_gains().unwrap();
        assert!(capital_gains.len() > 0usize);
    }

    #[test]
    fn search_options() {
        let provider = YahooConnector::new().unwrap();
        let resp = tokio_test::block_on(provider.search_options("AAPL"));

        assert!(resp.is_ok());
    }

    #[test]
    fn test_get_ticker_info() {
        let mut provider = YahooConnector::new().unwrap();

        let result = tokio_test::block_on(provider.get_ticker_info("AAPL"));

        let quote_summary = result.unwrap().quote_summary.unwrap();
        assert!(
            "Cupertino"
                == quote_summary.result[0]
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
}
