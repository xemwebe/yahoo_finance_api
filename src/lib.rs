//! # yahoo! finance API
//!
//! This project provides a set of functions to receive data from the
//! the [yahoo! finance](https://finance.yahoo.com) website via their API. This project
//! is licensed under Apache 2.0 or MIT license (see files LICENSE-Apache2.0 and LICENSE-MIT).
//!
//! Since version 0.3 and the upgrade to ```reqwest``` 0.10, all requests to the yahoo API return futures, using ```async``` features.
//! Therefore, the functions need to be called from within another ```async``` function with ```.await``` or via functions like ```block_on```.
//! The examples are based on the ```tokio``` runtime applying the ```tokio-test``` crate.
//!
//! Use the `blocking` feature to get the previous behavior back: i.e. `yahoo_finance_api = {"version": "1.0", features = ["blocking"]}`. 
//!
#![cfg_attr(not(feature="blocking"), doc = "
# Get the latest available quote:
```rust
use yahoo_finance_api as yahoo;
use std::time::{Duration, UNIX_EPOCH};
use chrono::prelude::*;
use tokio_test;

fn main() {
    let provider = yahoo::YahooConnector::new();
    // get the latest quotes in 1 minute intervals
    let response = tokio_test::block_on(provider.get_latest_quotes(\"AAPL\", \"1m\")).unwrap();
    // extract just the latest valid quote summery
    // including timestamp,open,close,high,low,volume
    let quote = response.last_quote().unwrap();
    let time: DateTime<Utc> =
        DateTime::from(UNIX_EPOCH + Duration::from_secs(quote.timestamp));
    println!(\"At {} quote price of Apple was {}\", time.to_rfc3339(), quote.close);
}
```
# Get history of quotes for given time period:
```rust
use yahoo_finance_api as yahoo;
use std::time::{Duration, UNIX_EPOCH};
use chrono::{Utc,TimeZone};
use tokio_test;

fn main() {
    let provider = yahoo::YahooConnector::new();
    let start = Utc.ymd(2020, 1, 1).and_hms_milli(0, 0, 0, 0);
    let end = Utc.ymd(2020, 1, 31).and_hms_milli(23, 59, 59, 999);
    // returns historic quotes with daily interval
    let resp = tokio_test::block_on(provider.get_quote_history(\"AAPL\", start, end)).unwrap();
    let quotes = resp.quotes().unwrap();
    println!(\"Apple's quotes in January: {:?}\", quotes);
}
```
# Another method to retrieve a range of quotes is by
# requesting the quotes for a given period and lookup frequency. Here is an example retrieving the daily quotes for the last month:
```rust
use yahoo_finance_api as yahoo;
use std::time::{Duration, UNIX_EPOCH};
use chrono::{Utc,TimeZone};
use tokio_test;

fn main() {
    let provider = yahoo::YahooConnector::new();
    let response = tokio_test::block_on(provider.get_quote_range(\"AAPL\", \"1d\", \"1mo\")).unwrap();
    let quotes = response.quotes().unwrap();
    println!(\"Apple's quotes of the last month: {:?}\", quotes);
}
```
")]
#![cfg_attr(feature="blocking", doc = "
# Get the latest available quote (with blocking feature enabled):
```rust
use yahoo_finance_api as yahoo;
use std::time::{Duration, UNIX_EPOCH};
use chrono::prelude::*;
use tokio_test;

fn main() {
    let provider = yahoo::YahooConnector::new();
    // get the latest quotes in 1 minute intervals
    let response = provider.get_latest_quotes(\"AAPL\", \"1m\").unwrap();
    // extract just the latest valid quote summery
    // including timestamp,open,close,high,low,volume
    let quote = response.last_quote().unwrap();
    let time: DateTime<Utc> =
        DateTime::from(UNIX_EPOCH + Duration::from_secs(quote.timestamp));
    println!(\"At {} quote price of Apple was {}\", time.to_rfc3339(), quote.close);
}
```
//!
Get history of quotes for given time period:
```rust
use yahoo_finance_api as yahoo;
use std::time::{Duration, UNIX_EPOCH};
use chrono::{Utc,TimeZone};
use tokio_test;

fn main() {
    let provider = yahoo::YahooConnector::new();
    let start = Utc.ymd(2020, 1, 1).and_hms_milli(0, 0, 0, 0);
    let end = Utc.ymd(2020, 1, 31).and_hms_milli(23, 59, 59, 999);
    // returns historic quotes with daily interval
    let resp = provider.get_quote_history(\"AAPL\", start, end).unwrap();
    let quotes = resp.quotes().unwrap();
    println!(\"Apple's quotes in January: {:?}\", quotes);
}

```
Another method to retrieve a range of quotes is by
requesting the quotes for a given period and lookup frequency. Here is an example retrieving the daily quotes for the last month:

```rust
use yahoo_finance_api as yahoo;
use tokio_test;

fn main() {
    let provider = yahoo::YahooConnector::new();
    let response = provider.get_quote_range(\"AAPL\", \"1d\", \"1mo\").unwrap();
    let quotes = response.quotes().unwrap();
    println!(\"Apple's quotes of the last month: {:?}\", quotes);
}
```
")]

use chrono::{DateTime, Utc};
use reqwest::StatusCode;

#[cfg(not(feature = "blocking"))]
use tokio_compat_02::FutureExt;

mod quotes;
mod search_result;
mod yahoo_error;
use quotes::YResponse;
use search_result::YSearchResult;
use yahoo_error::YahooError;

const YCHART_URL: &str = "https://query1.finance.yahoo.com/v8/finance/chart";
const YSEARCH_URL: &str = "https://query2.finance.yahoo.com/v1/finance/search";

// Macros instead of constants, 
macro_rules! YCHART_PERIOD_QUERY {
    () => {"{url}/{symbol}?symbol={symbol}&period1={start}&period2={end}&interval={interval}"};
}
macro_rules! YCHART_RANGE_QUERY {
    () => {"{url}/{symbol}?symbol={symbol}&interval={interval}&range={range}"};
} 
macro_rules! YTICKER_QUERY {
    () => {"{url}?q={name}"};
} 

/// Container for connection parameters to yahoo! finance server
#[derive(Default)]
pub struct YahooConnector {
    url: &'static str,
    search_url: &'static str,
}


impl YahooConnector {
    /// Constructor for a new instance of the yahoo  connector.
    pub fn new() -> YahooConnector {
        YahooConnector {
            url: YCHART_URL,
            search_url: YSEARCH_URL,
        }
    }
}

#[cfg(not(feature = "blocking"))]
impl YahooConnector {

    /// Retrieve the quotes of the last day for the given ticker
    pub async fn get_latest_quotes(
        &self,
        ticker: &str,
        interval: &str,
    ) -> Result<YResponse, YahooError> {
        self.get_quote_range(ticker, interval, "1d").await
    }

    /// Retrieve the quote history for the given ticker form date start to end (inklusive), if available
    pub async fn get_quote_history(
        &self,
        ticker: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
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
        YResponse::from_json(send_request(&url).await?)
    }
    /// Retrieve the quote history for the given ticker form date start to end (inklusive), if available; specifying the interval of the ticker.
    pub async fn get_quote_history_interval(
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
        YResponse::from_json(send_request(&url).await?)
    }

    /// Retrieve the list of quotes found searching a given name
    pub async fn search_ticker(&self, name: &str) -> Result<YSearchResult, YahooError> {
        let url = format!(YTICKER_QUERY!(), url = self.search_url, name = name);
        YSearchResult::from_json(send_request(&url).await?)
    }

}

#[cfg(feature = "blocking")]
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

#[cfg(not(feature = "blocking"))]
/// Send request to yahoo! finance server and transform response to JSON value
async fn send_request(url: &str) -> Result<serde_json::Value, YahooError> {
    let resp = reqwest::get(url).compat().await;
    if resp.is_err() {
        return Err(YahooError::ConnectionFailed);
    }
    let resp = resp.unwrap();
    match resp.status() {
        StatusCode::OK => resp.json().await.map_err(|_|{YahooError::InvalidJson}),
        status => Err(YahooError::FetchFailed(format!("Status Code: {}", status).to_string())),
    }
}

#[cfg(feature = "blocking")]
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

    #[cfg(feature = "blocking")]
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

    #[cfg(not(feature = "blocking"))]
    #[test]
    fn test_get_single_quote() {
        let provider = YahooConnector::new();
        let response = tokio_test::block_on(provider.get_latest_quotes("HNL.DE", "1m")).unwrap();

        assert_eq!(&response.chart.result[0].meta.symbol, "HNL.DE");
        assert_eq!(&response.chart.result[0].meta.range, "1d");
        assert_eq!(&response.chart.result[0].meta.data_granularity, "1m");
        let _ = response.last_quote().unwrap();
    }

    #[cfg(not(feature = "blocking"))]
    #[test]
    fn test_strange_api_responses() {
        let provider = YahooConnector::new();
        let start = Utc.ymd(2019, 7, 3).and_hms_milli(0, 0, 0, 0);
        let end = Utc.ymd(2020, 7, 4).and_hms_milli(23, 59, 59, 999);
        let resp = tokio_test::block_on(provider.get_quote_history("IBM", start, end)).unwrap();

        assert_eq!(&resp.chart.result[0].meta.symbol, "IBM");
        assert_eq!(&resp.chart.result[0].meta.data_granularity, "1d");
        assert_eq!(&resp.chart.result[0].meta.first_trade_date, &-252322200);

        let _ = resp.last_quote().unwrap();
    }

    #[cfg(not(feature = "blocking"))]
    #[test]
    #[should_panic(expected = "DeserializeFailed(\"missing field `adjclose`\")")]
    fn test_api_responses_missing_fields() {
        let provider = YahooConnector::new();
        let response = tokio_test::block_on(provider.get_latest_quotes("BF.B", "1m")).unwrap();

        assert_eq!(&response.chart.result[0].meta.symbol, "BF.B");
        assert_eq!(&response.chart.result[0].meta.range, "1d");
        assert_eq!(&response.chart.result[0].meta.data_granularity, "1m");
        let _ = response.last_quote().unwrap();
    }

    #[cfg(not(feature = "blocking"))]
    #[test]
    fn test_get_quote_history() {
        let provider = YahooConnector::new();
        let start = Utc.ymd(2020, 1, 1).and_hms_milli(0, 0, 0, 0);
        let end = Utc.ymd(2020, 1, 31).and_hms_milli(23, 59, 59, 999);
        let resp = tokio_test::block_on(provider.get_quote_history("AAPL", start, end));
        if resp.is_ok() {
            let resp = resp.unwrap();
            assert_eq!(resp.chart.result[0].timestamp.len(), 21);
            let quotes = resp.quotes().unwrap();
            assert_eq!(quotes.len(), 21);
        }
    }

    #[cfg(not(feature = "blocking"))]
    #[test]
    fn test_get_quote_range() {
        let provider = YahooConnector::new();
        let response =
            tokio_test::block_on(provider.get_quote_range("HNL.DE", "1d", "1mo")).unwrap();
        assert_eq!(&response.chart.result[0].meta.symbol, "HNL.DE");
        assert_eq!(&response.chart.result[0].meta.range, "1mo");
        assert_eq!(&response.chart.result[0].meta.data_granularity, "1d");
        let _ = response.last_quote().unwrap();
    }

    #[cfg(not(feature = "blocking"))]
    #[test]
    fn test_get() {
        let provider = YahooConnector::new();
        let start = Utc.ymd(2019, 1, 1).and_hms_milli(0, 0, 0, 0);
        let end = Utc.ymd(2020, 1, 31).and_hms_milli(23, 59, 59, 999);
        let response =
            tokio_test::block_on(provider.get_quote_history_interval("AAPL", start, end, "1mo"))
                .unwrap();
        assert_eq!(&response.chart.result[0].timestamp.len(), &13);
        assert_eq!(&response.chart.result[0].meta.data_granularity, "1mo");
        let quotes = response.quotes().unwrap();
        assert_eq!(quotes.len(), 13usize);
    }

    #[cfg(not(feature = "blocking"))]
    #[test]
    fn test_large_volume() {
        let provider = YahooConnector::new();
        let response = tokio_test::block_on(provider.get_quote_range("BTC-USD", "1d", "5d")).unwrap();
        let quotes = response.quotes().unwrap();
        assert_eq!(quotes.len(), 5usize);
    }

    #[cfg(not(feature = "blocking"))]
    #[test]
    fn test_search_ticker() {
        let provider = YahooConnector::new();
        let resp = tokio_test::block_on(provider.search_ticker("Apple")).unwrap();

        assert_eq!(resp.count, 15);
        let mut apple_found = false;
        for item in resp.quotes 
        {
            if item.exchange == "NMS" && item.symbol == "AAPL" && item.short_name == "Apple Inc." {
                apple_found = true;
                break;
            }
        }
        assert!(apple_found)
    }
}
