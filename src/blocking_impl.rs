use super::*;

impl YahooConnector {
    /// Retrieve the quotes of the last day for the given ticker
    pub fn get_latest_quotes(&self, ticker: &str, interval: &str) -> Result<YResponse, YahooError> {
        self.get_quote_range(ticker, interval, "1d")
    }

    /// Retrieve the quote history for the given ticker form date start to end (inklusive), if available
    pub fn get_quote_history(
        &self,
        ticker: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
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
        YResponse::from_json(send_request(&url)?)
    }
 
    /// Retrieve the quote history for the given ticker form date start to end (inklusive), if available; specifying the interval of the ticker.
    pub fn get_quote_history_interval(
        &self,
        ticker: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
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
        YResponse::from_json(send_request(&url)?)
    }

    /// Retrieve the list of quotes found searching a given name
    pub fn search_ticker(&self, name: &str) -> Result<YSearchResult, YahooError> {
        let url = format!(YTICKER_QUERY!(), url = self.search_url, name = name);
        YSearchResult::from_json(send_request(&url)?)
    }
}

/// Send request to yahoo! finance server and transform response to JSON value
fn send_request(url: &str) -> Result<serde_json::Value, YahooError> {
    let resp = reqwest::blocking::get(url);
    if resp.is_err() {
        return Err(YahooError::ConnectionFailed);
    }
    let resp = resp.unwrap();
    match resp.status() {
        StatusCode::OK => resp.json().map_err(|_|{YahooError::InvalidJson}),
        status => Err(YahooError::FetchFailed(format!("Status Code: {}", status).to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_get_quote_history() {
        let provider = YahooConnector::new();
        let start = Utc.ymd(2020, 1, 1).and_hms_milli(0, 0, 0, 0);
        let end = Utc.ymd(2020, 1, 31).and_hms_milli(23, 59, 59, 999);
        let resp = provider.get_quote_history("AAPL", start, end).unwrap();

        assert_eq!(resp.chart.result[0].timestamp.len(), 21);
        let quotes = resp.quotes().unwrap();
        assert_eq!(quotes.len(), 21);
    }
}
