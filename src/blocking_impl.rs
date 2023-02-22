use super::*;

impl YahooConnector {
    /// Retrieve the quotes of the last day for the given ticker
    pub fn get_latest_quotes(&self, ticker: &str, interval: &str) -> Result<YResponse, YahooError> {
        self.get_quote_range(ticker, interval, "1mo")
    }

    /// Retrieve the quote history for the given ticker form date start to end (inclusive), if available
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
            symbol = ticker,
            interval = interval,
            range = range
        );
        YResponse::from_json(self.send_request(&url)?)
    }

    /// Retrieve the quote history for the given ticker form date start to end (inclusive), if available; specifying the interval of the ticker.
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
            symbol = ticker,
            start = start.timestamp(),
            end = end.timestamp(),
            interval = interval
        );
        YResponse::from_json(self.send_request(&url)?)
    }

    /// Retrieve the list of quotes found searching a given name
    pub fn search_ticker_opt(&self, name: &str) -> Result<YSearchResultOpt, YahooError> {
        let url = format!(YTICKER_QUERY!(), url = self.search_url, name = name);
        YSearchResultOpt::from_json(self.send_request(&url)?)
    }

    /// Retrieve the list of quotes found searching a given name
    pub fn search_ticker(&self, name: &str) -> Result<YSearchResult, YahooError> {
        let result = self.search_ticker_opt(name)?;
        Ok(YSearchResult::from_opt(&result))
    }

    /// Get list for options for a given name
    pub fn search_options(&self, name: &str) -> Result<YOptionResults, YahooError> {
        let url = format!("https://finance.yahoo.com/quote/{name}/options?p={name}");
        let resp = self.client.get(url).send()?.text()?;
        Ok(YOptionResults::scrape(&resp))
    }

    /// Send request to yahoo! finance server and transform response to JSON value
    fn send_request(&self, url: &str) -> Result<serde_json::Value, YahooError> {
        let resp = self.client.get(url).send()?;

        match resp.status() {
            StatusCode::OK => Ok(resp.json()?),
            status => Err(YahooError::FetchFailed(format!("{}", status))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_get_single_quote() {
        let provider = YahooConnector::new();
        let response = provider.get_latest_quotes("HNL.DE", "1d").unwrap();
        assert_eq!(&response.chart.result[0].meta.symbol, "HNL.DE");
        assert_eq!(&response.chart.result[0].meta.range, "1mo");
        assert_eq!(&response.chart.result[0].meta.data_granularity, "1d");
        let _ = response.last_quote().unwrap();
    }

    #[test]
    fn test_strange_api_responses() {
        let provider = YahooConnector::new();
        let start = Utc.ymd(2019, 7, 3).and_hms_milli(0, 0, 0, 0);
        let end = Utc.ymd(2020, 7, 4).and_hms_milli(23, 59, 59, 999);
        let resp = provider.get_quote_history("IBM", start, end).unwrap();

        assert_eq!(&resp.chart.result[0].meta.symbol, "IBM");
        assert_eq!(&resp.chart.result[0].meta.data_granularity, "1d");
        assert_eq!(&resp.chart.result[0].meta.first_trade_date, &-252322200);

        let _ = resp.last_quote().unwrap();
    }

    #[test]
    #[should_panic(expected = "DeserializeFailed")]
    fn test_api_responses_missing_fields() {
        let provider = YahooConnector::new();
        let response = provider.get_latest_quotes("BF.B", "1m").unwrap();

        assert_eq!(&response.chart.result[0].meta.symbol, "BF.B");
        assert_eq!(&response.chart.result[0].meta.range, "1d");
        assert_eq!(&response.chart.result[0].meta.data_granularity, "1m");
        let _ = response.last_quote().unwrap();
    }

    #[test]
    fn test_get_quote_history() {
        let provider = YahooConnector::new();
        let start = Utc.ymd(2020, 1, 1).and_hms_milli(0, 0, 0, 0);
        let end = Utc.ymd(2020, 1, 31).and_hms_milli(23, 59, 59, 999);
        let resp = provider.get_quote_history("AAPL", start, end);
        if resp.is_ok() {
            let resp = resp.unwrap();
            assert_eq!(resp.chart.result[0].timestamp.len(), 21);
            let quotes = resp.quotes().unwrap();
            assert_eq!(quotes.len(), 21);
        }
    }

    #[test]
    fn test_get_quote_range() {
        let provider = YahooConnector::new();
        let response = provider.get_quote_range("HNL.DE", "1d", "1mo").unwrap();
        assert_eq!(&response.chart.result[0].meta.symbol, "HNL.DE");
        assert_eq!(&response.chart.result[0].meta.range, "1mo");
        assert_eq!(&response.chart.result[0].meta.data_granularity, "1d");
        let _ = response.last_quote().unwrap();
    }

    #[test]
    fn test_get() {
        let provider = YahooConnector::new();
        let start = Utc.ymd(2019, 1, 1).and_hms_milli(0, 0, 0, 0);
        let end = Utc.ymd(2020, 1, 31).and_hms_milli(23, 59, 59, 999);
        let response = provider
            .get_quote_history_interval("AAPL", start, end, "1mo")
            .unwrap();
        assert_eq!(&response.chart.result[0].timestamp.len(), &13);
        assert_eq!(&response.chart.result[0].meta.data_granularity, "1mo");
        let quotes = response.quotes().unwrap();
        assert_eq!(quotes.len(), 13usize);
    }

    #[test]
    fn test_large_volume() {
        let provider = YahooConnector::new();
        let response = provider.get_quote_range("BTC-USD", "1d", "5d").unwrap();
        let quotes = response.quotes().unwrap();
        assert!(quotes.len() > 0usize);
    }

    #[test]
    fn test_search_ticker() {
        let provider = YahooConnector::new();
        let resp = provider.search_ticker("Apple").unwrap();

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
        let provider = YahooConnector::new();
        let start = Utc.ymd(2020, 1, 1).and_hms_milli(0, 0, 0, 0);
        let end = Utc.ymd(2020, 1, 31).and_hms_milli(23, 59, 59, 999);
        let resp = provider.get_quote_history("VTSAX", start, end);
        if resp.is_ok() {
            let resp = resp.unwrap();
            assert_eq!(resp.chart.result[0].timestamp.len(), 21);
            let quotes = resp.quotes().unwrap();
            assert_eq!(quotes.len(), 21);
            println!("{:?}", quotes);
        }
    }

    #[test]
    fn test_mutual_fund_latest() {
        let provider = YahooConnector::new();
        let response = provider.get_latest_quotes("VTSAX", "1d").unwrap();

        assert_eq!(&response.chart.result[0].meta.symbol, "VTSAX");
        assert_eq!(&response.chart.result[0].meta.range, "1mo");
        assert_eq!(&response.chart.result[0].meta.data_granularity, "1d");
        let _ = response.last_quote().unwrap();
    }

    #[test]
    fn test_mutual_fund_range() {
        let provider = YahooConnector::new();
        let response = provider.get_quote_range("VTSAX", "1d", "1mo").unwrap();
        assert_eq!(&response.chart.result[0].meta.symbol, "VTSAX");
        assert_eq!(&response.chart.result[0].meta.range, "1mo");
        assert_eq!(&response.chart.result[0].meta.data_granularity, "1d");
    }
}
