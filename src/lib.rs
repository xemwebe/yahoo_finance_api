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
#![cfg_attr(
    not(feature = "blocking"),
    doc = "
# Get the latest available quote:
```rust
use yahoo_finance_api as yahoo;
use std::time::{Duration, UNIX_EPOCH};
use time::OffsetDateTime;
use tokio_test;

fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    // get the latest quotes in 1 minute intervals
    let response = tokio_test::block_on(provider.get_latest_quotes(\"AAPL\", \"1d\")).unwrap();
    // extract just the latest valid quote summery
    // including timestamp,open,close,high,low,volume
    let quote = response.last_quote().unwrap();
    let time: OffsetDateTime =
        OffsetDateTime::from(UNIX_EPOCH + Duration::from_secs(quote.timestamp));
    println!(\"At {} quote price of Apple was {}\", time, quote.close);
}
```
# Get history of quotes for given time period:
```rust
use yahoo_finance_api as yahoo;
use std::time::{Duration, UNIX_EPOCH};
use time::{macros::datetime, OffsetDateTime};
use tokio_test;

fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    let start = datetime!(2020-1-1 0:00:00.00 UTC);
    let end = datetime!(2020-1-31 23:59:59.99 UTC);
    // returns historic quotes with daily interval
    let resp = tokio_test::block_on(provider.get_quote_history(\"AAPL\", start, end)).unwrap();
    let quotes = resp.quotes().unwrap();
    println!(\"Apple's quotes in January: {:?}\", quotes);
}
```
# Get the history of quotes for time range
Another method to retrieve a range of quotes is by requesting the quotes for a given period and
lookup frequency. Here is an example retrieving the daily quotes for the last month:
```rust
use yahoo_finance_api as yahoo;
use std::time::{Duration, UNIX_EPOCH};
use tokio_test;

fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    let response = tokio_test::block_on(provider.get_quote_range(\"AAPL\", \"1d\", \"1mo\")).unwrap();
    let quotes = response.quotes().unwrap();
    println!(\"Apple's quotes of the last month: {:?}\", quotes);
}
```

# Search for a ticker given a search string (e.g. company name):
```rust
use yahoo_finance_api as yahoo;
use tokio_test;

fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    let resp = tokio_test::block_on(provider.search_ticker(\"Apple\")).unwrap();

    let mut apple_found = false;
    println!(\"All tickers found while searching for 'Apple':\");
    for item in resp.quotes
    {
        println!(\"{}\", item.symbol)
    }
}
```
Some fields like `longname` are only optional and will be replaced by default
values if missing (e.g. empty string). If you do not like this behavior,
use `search_ticker_opt` instead which contains `Option<String>` fields,
returning `None` if the field found missing in the response.
"
)]
//!
#![cfg_attr(
    feature = "blocking",
    doc = "
# Get the latest available quote (with blocking feature enabled):
```rust
use yahoo_finance_api as yahoo;
use std::time::{Duration, UNIX_EPOCH};
use time::OffsetDateTime;

fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    // get the latest quotes in 1 minute intervals
    let response = provider.get_latest_quotes(\"AAPL\", \"1d\").unwrap();
    // extract just the latest valid quote summery
    // including timestamp,open,close,high,low,volume
    let quote = response.last_quote().unwrap();
    let time: OffsetDateTime =
        OffsetDateTime::from(UNIX_EPOCH + Duration::from_secs(quote.timestamp));
    println!(\"At {} quote price of Apple was {}\", time, quote.close);
}
```
# Get history of quotes for given time period:
```rust
use yahoo_finance_api as yahoo;
use std::time::{Duration, UNIX_EPOCH};
use time::{macros::datetime, OffsetDateTime};

fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    let start = datetime!(2020-1-1 0:00:00.00 UTC);
    let end = datetime!(2020-1-31 23:59:59.99 UTC);
    // returns historic quotes with daily interval
    let resp = provider.get_quote_history(\"AAPL\", start, end).unwrap();
    let quotes = resp.quotes().unwrap();
    println!(\"Apple's quotes in January: {:?}\", quotes);
}

```
# Get the history of quotes for time range
Another method to retrieve a range of quotes is by requesting the quotes for a given period and
lookup frequency. Here is an example retrieving the daily quotes for the last month:
```rust
use yahoo_finance_api as yahoo;

fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    let response = provider.get_quote_range(\"AAPL\", \"1d\", \"1mo\").unwrap();
    let quotes = response.quotes().unwrap();
    println!(\"Apple's quotes of the last month: {:?}\", quotes);
}
```
# Search for a ticker given a search string (e.g. company name):
```rust
use yahoo_finance_api as yahoo;

fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    let resp = provider.search_ticker(\"Apple\").unwrap();

    let mut apple_found = false;
    println!(\"All tickers found while searching for 'Apple':\");
    for item in resp.quotes
    {
        println!(\"{}\", item.symbol)
    }
}
```
"
)]

use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;

#[cfg(feature = "blocking")]
use reqwest::blocking::{Client, ClientBuilder};
#[cfg(not(feature = "blocking"))]
use reqwest::{Client, ClientBuilder};
use reqwest::{Proxy, StatusCode};

// re-export time crate
pub use quotes::decimal::Decimal;
pub use time;

mod quotes;
mod search_result;
mod yahoo_error;
pub use quotes::{
    AdjClose, AssetProfile, CapitalGain, CurrentTradingPeriod, DefaultKeyStatistics, Dividend,
    ExtendedQuoteSummary, FinancialData, PeriodInfo, Quote, QuoteBlock, QuoteList, QuoteType,
    Split, SummaryDetail, TradingPeriods, YChart, YMetaData, YQuoteBlock, YQuoteSummary, YResponse,
    YSummaryData,
};
pub use search_result::{
    YNewsItem, YOptionChain, YOptionChainData, YOptionChainResult, YOptionContract, YOptionDetails,
    YQuote, YQuoteItem, YQuoteItemOpt, YSearchResult, YSearchResultOpt,
};
pub use yahoo_error::YahooError;

const YCHART_URL: &str = "https://query1.finance.yahoo.com/v8/finance/chart";
const YSEARCH_URL: &str = "https://query2.finance.yahoo.com/v1/finance/search";
const Y_GET_COOKIE_URL: &str = "https://fc.yahoo.com";
const Y_GET_CRUMB_URL: &str = "https://query1.finance.yahoo.com/v1/test/getcrumb";

// special yahoo hardcoded keys and headers
const Y_COOKIE_REQUEST_HEADER: &str = "set-cookie";
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36";

// Macros instead of constants,
macro_rules! YCHART_PERIOD_QUERY {
    () => {
        "{url}/{symbol}?symbol={symbol}&period1={start}&period2={end}&interval={interval}&events=div|split|capitalGains"
    };
}
macro_rules! YCHART_PERIOD_QUERY_PRE_POST {
    () => {
        "{url}/{symbol}?symbol={symbol}&period1={start}&period2={end}&interval={interval}&events=div|split|capitalGains&includePrePost={prepost}"
    };
}
macro_rules! YCHART_RANGE_QUERY {
  () => {
    "{url}/{symbol}?symbol={symbol}&interval={interval}&range={range}&events=div|split|capitalGains"
  };
}
macro_rules! YCHART_PERIOD_INTERVAL_QUERY {
    () => {
        "{url}/{symbol}?symbol={symbol}&range={range}&interval={interval}&includePrePost={prepost}"
    };
}
macro_rules! YTICKER_QUERY {
    () => {
        "{url}?q={name}"
    };
}
macro_rules! YQUOTE_SUMMARY_QUERY {
    () => {
        "https://query2.finance.yahoo.com/v10/finance/quoteSummary/{symbol}?modules=financialData,quoteType,defaultKeyStatistics,assetProfile,summaryDetail&corsDomain=finance.yahoo.com&formatted=false&symbol={symbol}&crumb={crumb}"
    }
}

/// Container for connection parameters to yahoo! finance server
pub struct YahooConnector {
    client: Client,
    url: &'static str,
    search_url: &'static str,
    timeout: Option<Duration>,
    user_agent: Option<String>,
    proxy: Option<Proxy>,
    cookie: Option<String>,
    crumb: Option<String>,
}

#[derive(Default)]
pub struct YahooConnectorBuilder {
    inner: ClientBuilder,
    timeout: Option<Duration>,
    user_agent: Option<String>,
    proxy: Option<Proxy>,
}

impl YahooConnector {
    /// Constructor for a new instance of the yahoo connector.
    pub fn new() -> Result<YahooConnector, YahooError> {
        Self::builder().build()
    }

    pub fn builder() -> YahooConnectorBuilder {
        YahooConnectorBuilder {
            inner: Client::builder(),
            user_agent: Some(USER_AGENT.to_string()),
            ..Default::default()
        }
    }
}

impl Default for YahooConnector {
    fn default() -> Self {
        YahooConnector {
            client: Client::default(),
            url: YCHART_URL,
            search_url: YSEARCH_URL,
            timeout: None,
            user_agent: Some(USER_AGENT.to_string()),
            proxy: None,
            cookie: None,
            crumb: None,
        }
    }
}

impl YahooConnectorBuilder {
    pub fn new() -> Self {
        YahooConnector::builder()
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn user_agent(mut self, user_agent: &str) -> Self {
        self.user_agent = Some(user_agent.to_string());
        self
    }

    pub fn proxy(mut self, proxy: Proxy) -> Self {
        self.proxy = Some(proxy);
        self
    }

    pub fn build(mut self) -> Result<YahooConnector, YahooError> {
        if let Some(timeout) = &self.timeout {
            self.inner = self.inner.timeout(timeout.clone());
        }
        if let Some(user_agent) = &self.user_agent {
            self.inner = self.inner.user_agent(user_agent.clone());
        }
        if let Some(proxy) = &self.proxy {
            self.inner = self.inner.proxy(proxy.clone());
        }

        Ok(YahooConnector {
            client: self.inner.build()?,
            timeout: self.timeout,
            user_agent: self.user_agent,
            proxy: self.proxy,
            ..Default::default()
        })
    }

    pub fn build_with_client(client: Client) -> Result<YahooConnector, YahooError> {
        Ok(YahooConnector {
            client,
            ..Default::default()
        })
    }
}

#[cfg(not(feature = "blocking"))]
pub mod async_impl;

#[cfg(feature = "blocking")]
pub mod blocking_impl;
