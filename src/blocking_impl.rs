use super::*;

/// Container for connection parameters to yahoo! finance server
#[derive(Default)]
pub struct YahooConnectorBlocking {
    url: &'static str,
    search_url: &'static str,
}

impl YahooConnectorBlocking {
    /// Constructor for a new instance of the yahoo  connector.
    pub fn new() -> Self {
        Self {
            url: YCHART_URL,
            search_url: YSEARCH_URL,
        }
    }
}

impl YahooConnectorBlocking {
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
    pub fn search_ticker_opt(&self, name: &str) -> Result<YSearchResultOpt, YahooError> {
        let url = format!(YTICKER_QUERY!(), url = self.search_url, name = name);
        YSearchResultOpt::from_json(send_request(&url)?)
    }

    /// Retrieve the list of quotes found searching a given name
    pub fn search_ticker(&self, name: &str) -> Result<YSearchResult, YahooError> {
        let result = self.search_ticker_opt(name)?;
        Ok(YSearchResult::from_opt(&result))
    }
}

/// Send request to yahoo! finance server and transform response to JSON value
fn send_request(url: &str) -> Result<serde_json::Value, YahooError> {
    let resp = ureq::get(url).call();
    if let Ok(resp) = resp {
        match resp.status() {
            200 => resp.into_json().map_err(|_| YahooError::InvalidJson),
            status => Err(YahooError::FetchFailed(format!("Status Code: {}", status))),
        }
    } else {
        Err(YahooError::ConnectionFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_get_quote_history() {
        let provider = YahooConnectorBlocking::new();
        let start = Utc.ymd(2020, 1, 1).and_hms_milli(0, 0, 0, 0);
        let end = Utc.ymd(2020, 1, 31).and_hms_milli(23, 59, 59, 999);
        let resp = provider.get_quote_history("AAPL", start, end).unwrap();

        assert_eq!(resp.chart.result[0].timestamp.len(), 21);
        let quotes = resp.quotes().unwrap();
        assert_eq!(quotes.len(), 21);
    }
}
