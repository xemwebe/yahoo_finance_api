//! # yahoo! finance API
//!
//! This project provides a set of functions to receive data from the
//! the [yahoo! finance](https://finance.yahoo.com) website via their API. This project
//! is licensed under Apache 2.0 or MIT license (see files LICENSE-Apache2.0 and LICENSE-MIT).
//!
//! There is already an existing rust library [yahoo-finance-rs](https://github.com/fbriden/yahoo-finance-rs),
//! which I intended to use for my own projects. However, due some issues in the implementation (the library panics
//! in some cases if yahoo does provide somehow invalid data), I currently can't use it. Once this issue is fixed,
//! I might switch back and drop development of this library.
//!
//! Since version 0.3 and the upgrade to ```reqwest``` 0.10, all requests to the yahoo API return futures, using ```async``` features.
//! Therefore, the functions need to be called from within another ```async``` function with ```.await``` or via funtions like ```block_on```.
//! The examples are based on the ```tokio``` runtime. The examples are based on the ```tokio``` runtime applying the ```tokio-test``` crate.
//! 
//! Get the latest available quote:
//! ```rust
//! use yahoo_finance_api as yahoo;
//! use std::time::{Duration, UNIX_EPOCH};
//! use chrono::prelude::*;
//! use tokio_test;
//! 
//! fn main() {
//!     let provider = yahoo::YahooConnector::new();
//!     // get the latest quotes in 1 minute intervals
//!     let response = tokio_test::block_on(provider.get_latest_quotes("AAPL", "1m")).unwrap();
//!     // extract just the latest valid quote summery
//!     // including timestamp,open,close,high,low,volume
//!     let quote = response.last_quote().unwrap();
//!     let time: DateTime<Utc> =
//!         DateTime::from(UNIX_EPOCH + Duration::from_secs(quote.timestamp));
//!     println!("At {} quote price of Apple was {}", time.to_rfc3339(), quote.close);
//! }
//! ```
//!
//! Get history of quotes for given time period:
//! ```rust
//! use yahoo_finance_api as yahoo;
//! use std::time::{Duration, UNIX_EPOCH};
//! use chrono::{Utc,TimeZone};
//! use tokio_test;
//! 
//! fn main() {
//!     let provider = yahoo::YahooConnector::new();
//!     let start = Utc.ymd(2020, 1, 1).and_hms_milli(0, 0, 0, 0);
//!     let end = Utc.ymd(2020, 1, 31).and_hms_milli(23, 59, 59, 999);
//!     // returns historic quotes with daily interval
//!     let resp = tokio_test::block_on(provider.get_quote_history("AAPL", start, end)).unwrap();
//!     let quotes = resp.quotes().unwrap();
//!     println!("Apple's quotes in January: {:?}", quotes);
//! }
//! ```
//! Another method to retrieve a range of quotes is by
//! requesting the quotes for a given period and lookup frequency. Here is an example retrieving the daily quotes for the last month:
//! ```rust
//! use yahoo_finance_api as yahoo;
//! use std::time::{Duration, UNIX_EPOCH};
//! use chrono::{Utc,TimeZone};
//! use tokio_test;
//! 
//! fn main() {
//!     let provider = yahoo::YahooConnector::new();
//!     let response = tokio_test::block_on(provider.get_quote_range("AAPL", "1d", "1mo")).unwrap();
//!     let quotes = response.quotes().unwrap();
//!     println!("Apple's quotes of the last month: {:?}", quotes);
//! }
//! ```

use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::Value;
use std::fmt;

#[derive(Deserialize, Debug)]
pub struct YResponse {
    pub chart: YChart,
}

impl YResponse {
    fn check_consistency(&self) -> Result<(), YahooError> {
        for stock in &self.chart.result {
            let n = stock.timestamp.len();
            if n == 0 {
                return Err(YahooError::EmptyDataSet);
            }
            let quote = &stock.indicators.quote[0];
            if quote.open.len() != n
                || quote.high.len() != n
                || quote.low.len() != n
                || quote.volume.len() != n
                || quote.close.len() != n
            {
                return Err(YahooError::DataInconsistency);
            }
            if stock.indicators.adjclose.is_some() {
                let adjclose = stock.indicators.adjclose.as_ref().unwrap();
                if adjclose[0].adjclose.len() != n {
                    return Err(YahooError::DataInconsistency);
                }
            }
        }
        Ok(())
    }

    /// Return the latest valid quote
    pub fn last_quote(&self) -> Result<Quote, YahooError> {
        self.check_consistency()?;
        let stock = &self.chart.result[0];
        let n = stock.timestamp.len() - 1;
        for i in (0..n).rev() {
            let quote = stock.indicators.get_ith_quote(stock.timestamp[i], i);
            if quote.is_ok() {
                return quote;
            }
        }
        Err(YahooError::EmptyDataSet)
    }

    pub fn quotes(&self) -> Result<Vec<Quote>, YahooError> {
        self.check_consistency()?;
        let stock = &self.chart.result[0];
        let mut quotes = Vec::new();
        let n = stock.timestamp.len();
        for i in 0..n {
            let timestamp = stock.timestamp[i];
            let quote = stock.indicators.get_ith_quote(timestamp, i);
            if quote.is_ok() {
                quotes.push(quote.unwrap());
            }
        }
        Ok(quotes)
    }
}

/// Struct for single quote
#[derive(Debug)]
pub struct Quote {
    pub timestamp: u64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub volume: u32,
    pub close: f64,
    pub adjclose: f64,
}

#[derive(Deserialize, Debug)]
pub struct YChart {
    pub result: Vec<YQuoteBlock>,
    pub error: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct YQuoteBlock {
    pub meta: YMetaData,
    pub timestamp: Vec<u64>,
    pub indicators: QuoteBlock,
}

#[derive(Deserialize, Debug)]
pub struct YMetaData {
    pub currency: String,
    pub symbol: String,
    #[serde(rename = "exchangeName")]
    pub exchange_name: String,
    #[serde(rename = "instrumentType")]
    pub instrument_type: String,
    #[serde(rename = "firstTradeDate")]
    pub first_trade_date: u32,
    #[serde(rename = "regularMarketTime")]
    pub regular_market_time: u32,
    pub gmtoffset: i32,
    pub timezone: String,
    #[serde(rename = "exchangeTimezoneName")]
    pub exchange_timezone_name: String,
    #[serde(rename = "regularMarketPrice")]
    pub regular_market_price: f64,
    #[serde(rename = "chartPreviousClose")]
    pub chart_previous_close: f64,
    #[serde(default)]
    #[serde(rename = "previousClose")]
    pub previous_close: Option<f64>,
    #[serde(default)]
    pub scale: Option<i32>,
    #[serde(rename = "priceHint")]
    pub price_hint: i32,
    #[serde(rename = "currentTradingPeriod")]
    pub current_trading_period: TradingPeriod,
    #[serde(default)]
    #[serde(rename = "tradingPeriods")]
    pub trading_periods: Option<Vec<Vec<PeriodInfo>>>,
    #[serde(rename = "dataGranularity")]
    pub data_granularity: String,
    pub range: String,
    #[serde(rename = "validRanges")]
    pub valid_ranges: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct TradingPeriod {
    pub pre: PeriodInfo,
    pub regular: PeriodInfo,
    pub post: PeriodInfo,
}

#[derive(Deserialize, Debug)]
pub struct PeriodInfo {
    pub timezone: String,
    pub start: u32,
    pub end: u32,
    pub gmtoffset: i32,
}

#[derive(Deserialize, Debug)]
pub struct QuoteBlock {
    quote: Vec<QuoteList>,
    #[serde(default)]
    adjclose: Option<Vec<AdjClose>>,
}

impl QuoteBlock {
    fn get_ith_quote(&self, timestamp: u64, i: usize) -> Result<Quote, YahooError> {
        let adjclose = match &self.adjclose {
            Some(adjclose) => adjclose[0].adjclose[i],
            None => None,
        };
        let quote = &self.quote[0];
        // reject if close is not set
        if quote.close[i].is_none() {
            return Err(YahooError::EmptyDataSet);
        }
        Ok(Quote {
            timestamp: timestamp,
            open: quote.open[i].unwrap_or(0.0),
            high: quote.high[i].unwrap_or(0.0),
            low: quote.low[i].unwrap_or(0.0),
            volume: quote.volume[i].unwrap_or(0),
            close: quote.close[i].unwrap(),
            adjclose: adjclose.unwrap_or(0.0),
        })
    }
}

#[derive(Deserialize, Debug)]
pub struct AdjClose {
    adjclose: Vec<Option<f64>>,
}

#[derive(Deserialize, Debug)]
pub struct QuoteList {
    pub volume: Vec<Option<u32>>,
    pub high: Vec<Option<f64>>,
    pub close: Vec<Option<f64>>,
    pub low: Vec<Option<f64>>,
    pub open: Vec<Option<f64>>,
}

#[derive(Debug)]
pub enum YahooError {
    FetchFailed(StatusCode),
    DeserializeFailed(String),
    ConnectionFailed,
    InvalidStatusCode,
    EmptyDataSet,
    DataInconsistency,
}

impl std::error::Error for YahooError {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        match self {
            _ => None,
        }
    }
}

impl fmt::Display for YahooError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FetchFailed(status) => write!(
                f,
                "fetchin the data from yahoo! finance failed: with status code {}",
                status
            ),
            Self::DeserializeFailed(s) => write!(
                f,
                "deserializing response from yahoo! finance failed: {}",
                &s
            ),
            Self::ConnectionFailed => write!(f, "connection to yahoo! finance server failed"),
            Self::InvalidStatusCode => write!(f, "yahoo! finance return invalid status code"),
            Self::EmptyDataSet => write!(f, "yahoo! finance returned an empty data set"),
            Self::DataInconsistency => write!(f, "yahoo! finance returned inconsistent data"),
        }
    }
}

/// Container for connection parameters to yahoo! finance server
pub struct YahooConnector {
    url: &'static str,
}

impl YahooConnector {
    /// Constructor for a new instance of the yahoo  connector.
    pub fn new() -> YahooConnector {
        YahooConnector {
            url: "https://query1.finance.yahoo.com/v8/finance/chart",
        }
    }

    /// Retrieve the quotes of the last day for the given ticker
    pub async fn get_latest_quotes(&self, ticker: &str, interval: &str) -> Result<YResponse, YahooError> {
        self.get_quote_range(ticker, interval, "1d").await
    }

    /// Retrieve the quote history for the given ticker form date start to end (inklusive), if available
    pub async fn get_quote_history(
        &self,
        ticker: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<YResponse, YahooError> {
        self.get_quote_history_interval(ticker, start, end, "1d").await
    }

     /// Retrieve quotes for the given ticker for an arbitrary range
     pub async fn get_quote_range(&self, ticker: &str, interval: &str, range: &str) -> Result<YResponse, YahooError> {
        let url: String = format!(
            "{url}/{symbol}?symbol={symbol}&interval={interval}&range={range}",
            url = self.url,
            symbol = ticker,
            interval = interval, 
            range = range
        );
        let resp = self.send_request(&url).await?;
        let response: YResponse = serde_json::from_value(resp)
            .map_err(|e| YahooError::DeserializeFailed(e.to_string()))?;
        Ok(response)
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
            "{url}/{symbol}?symbol={symbol}&period1={start}&period2={end}&interval={interval}",
            url = self.url,
            symbol = ticker,
            start = start.timestamp(),
            end = end.timestamp(),
            interval = interval
        );
        let resp = self.send_request(&url).await?;
        let response: YResponse = serde_json::from_value(resp)
            .map_err(|err| YahooError::DeserializeFailed(err.to_string()))?;
        Ok(response)
    }

    /// Send request to yahoo! finance server and transform response to JSON value
    async fn send_request(&self, url: &str) -> Result<Value, YahooError> {
        let resp = reqwest::get(url).await;
        if resp.is_err() {
            return Err(YahooError::ConnectionFailed);
        }
        let resp = resp.unwrap();
        match resp.status() {
            StatusCode::OK => match resp.json().await {
                Ok(json) => Ok(json),
                _ => Err(YahooError::InvalidStatusCode),
            },

            status => Err(YahooError::FetchFailed(status)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use tokio_test;

    #[test]
    fn test_get_single_quote() {
        let provider = YahooConnector::new();
        let response = tokio_test::block_on(provider.get_latest_quotes("HNL.DE", "1m")).unwrap();

        assert_eq!(&response.chart.result[0].meta.symbol, "HNL.DE");
        assert_eq!(&response.chart.result[0].meta.range, "1d");
        assert_eq!(&response.chart.result[0].meta.data_granularity, "1m");
        let _ = response.last_quote().unwrap();
    }

    #[test]
    fn test_get_quote_history() {
        let provider = YahooConnector::new();
        let start = Utc.ymd(2020, 1, 1).and_hms_milli(0, 0, 0, 0);
        let end = Utc.ymd(2020, 1, 31).and_hms_milli(23, 59, 59, 999);
        let resp = tokio_test::block_on(provider.get_quote_history("AAPL", start, end)).unwrap();

        assert_eq!(resp.chart.result[0].timestamp.len(), 21);
        let quotes = resp.quotes().unwrap();
        assert_eq!(quotes.len(), 21);
    }


    #[test]
    fn test_get_quote_range() {
        let provider = YahooConnector::new();
        let response = tokio_test::block_on(provider.get_quote_range("HNL.DE", "1d", "1mo")).unwrap();
        assert_eq!(&response.chart.result[0].meta.symbol, "HNL.DE");
        assert_eq!(&response.chart.result[0].meta.range, "1mo");
        assert_eq!(&response.chart.result[0].meta.data_granularity, "1d");
        let _ = response.last_quote().unwrap();
    }

    #[test]
    fn test_get_() {
        let provider = YahooConnector::new();
        let start = Utc.ymd(2019, 1, 1).and_hms_milli(0, 0, 0, 0);
        let end = Utc.ymd(2020, 1, 31).and_hms_milli(23, 59, 59, 999);
        let response = tokio_test::block_on(provider.get_quote_history_interval("AAPL", start, end, "1mo")).unwrap();
        assert_eq!(&response.chart.result[0].timestamp.len(), &13);
        assert_eq!(&response.chart.result[0].meta.data_granularity, "1mo");
        let quotes = response.quotes().unwrap();
        assert_eq!(quotes.len(), 13usize);
    }
}
