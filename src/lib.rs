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
")]

use chrono::{DateTime, Utc};
use reqwest::StatusCode;

#[cfg(not(feature = "blocking"))]
use tokio_compat_02::FutureExt;

mod quotes;
mod search_result;
mod yahoo_error;
pub use quotes::{YResponse, Quote, YChart, YQuoteBlock, YMetaData, TradingPeriod, PeriodInfo, QuoteBlock, AdjClose, QuoteList};
pub use search_result::{YSearchResultOpt, YQuoteItemOpt, YSearchResult, YQuoteItem, YNewsItem};
pub use yahoo_error::YahooError;

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
pub mod async_impl;

#[cfg(feature = "blocking")]
pub mod blocking_impl;
