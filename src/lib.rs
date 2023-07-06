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
    let provider = yahoo::YahooConnector::new();
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
    let provider = yahoo::YahooConnector::new();
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
    let provider = yahoo::YahooConnector::new();
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
    let provider = yahoo::YahooConnector::new();
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
    let provider = yahoo::YahooConnector::new();
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
    let provider = yahoo::YahooConnector::new();
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
    let provider = yahoo::YahooConnector::new();
    let response = provider.get_quote_range(\"AAPL\", \"1d\", \"1mo\").unwrap();
    let quotes = response.quotes().unwrap();
    println!(\"Apple's quotes of the last month: {:?}\", quotes);
}
```
# Search for a ticker given a search string (e.g. company name):
```rust
use yahoo_finance_api as yahoo;

fn main() {
    let provider = yahoo::YahooConnector::new();
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

use std::{
    time::Duration,
};
use time::OffsetDateTime;

#[cfg(feature = "blocking")]
use reqwest::blocking::{Client, ClientBuilder};
use reqwest::StatusCode;
#[cfg(not(feature = "blocking"))]
use reqwest::{Client, ClientBuilder};

// re-export time crate
pub use time;

mod quotes;
mod quote_summary;
mod search_result;
mod yahoo_error;
pub use quotes::{
    AdjClose, Dividend, PeriodInfo, Quote, QuoteBlock, QuoteList, Split, TradingPeriod, YChart,
    YMetaData, YQuoteBlock, YResponse,
};
pub use quote_summary::{YQuoteResponse, YQuoteSummary};
pub use search_result::{
    YNewsItem, YOptionResult, YOptionResults, YQuoteItem, YQuoteItemOpt, YSearchResult,
    YSearchResultOpt,
};
pub use yahoo_error::YahooError;

const YCHART_URL: &str = "https://query1.finance.yahoo.com/v8/finance/chart";
const YSEARCH_URL: &str = "https://query2.finance.yahoo.com/v1/finance/search";

// Macros instead of constants,
macro_rules! YCHART_PERIOD_QUERY {
    () => {
        "{url}/{symbol}?symbol={symbol}&period1={start}&period2={end}&interval={interval}&events=div|split"
    };
}
macro_rules! YCHART_RANGE_QUERY {
    () => {
        "{url}/{symbol}?symbol={symbol}&interval={interval}&range={range}&events=div|split"
    };
}
macro_rules! YTICKER_QUERY {
    () => {
        "{url}?q={name}"
    };
}

/// Container for connection parameters to yahoo! finance server
pub struct YahooConnector {
    client: Client,
    url: &'static str,
    search_url: &'static str,
}

#[derive(Default)]
pub struct YahooConnectorBuilder {
    inner: ClientBuilder,
}

impl YahooConnector {
    /// Constructor for a new instance of the yahoo connector.
    pub fn new() -> YahooConnector {
        YahooConnector {
            client: Client::default(),
            url: YCHART_URL,
            search_url: YSEARCH_URL,
        }
    }

    pub fn builder() -> YahooConnectorBuilder {
        YahooConnectorBuilder {
            inner: Client::builder(),
        }
    }
}

impl Default for YahooConnector {
    fn default() -> Self {
        Self::new()
    }
}

impl YahooConnectorBuilder {
    pub fn build(self) -> Result<YahooConnector, YahooError> {
        let builder = Client::builder();

        Ok(YahooConnector {
            client: builder.build()?,
            url: YCHART_URL,
            search_url: YSEARCH_URL,
        })
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.inner = self.inner.timeout(timeout);

        self
    }
}

#[cfg(not(feature = "blocking"))]
pub mod async_impl;

#[cfg(feature = "blocking")]
pub mod blocking_impl;
