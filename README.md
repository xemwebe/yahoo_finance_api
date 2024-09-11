# yahoo! finance API
This project provides a set of functions to receive data from the
the [yahoo! finance](https://finance.yahoo.com) website via their API. This project
is licensed under Apache 2.0 or MIT license (see files LICENSE-Apache2.0 and LICENSE-MIT).

Since version 0.3 and the upgrade to ```reqwest``` 0.10, all requests to the yahoo API return futures, using ```async``` features.
Therefore, the functions need to be called from within another ```async``` function with ```.await``` or via functions like ```block_on```. The examples are based on the ```tokio``` runtime applying the ```tokio-test``` crate.

Use the `blocking` feature to get the previous behavior back: i.e. `yahoo_finance_api = {"version" = "1.0", features = ["blocking"]}`. 

# Get the latest available quote:
```rust
use yahoo_finance_api as yahoo;
use std::time::{Duration, UNIX_EPOCH};
use time::OffsetDateTime;
use tokio_test;

fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    // get the latest quotes in 1 minute intervals
    let response = tokio_test::block_on(provider.get_latest_quotes("AAPL", "1d")).unwrap();
    // extract just the latest valid quote summery
    // including timestamp,open,close,high,low,volume
    let quote = response.last_quote().unwrap();
    let time: OffsetDateTime =
        OffsetDateTime::from(UNIX_EPOCH + Duration::from_secs(quote.timestamp));
    println!("At {} quote price of Apple was {}", time, quote.close);
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
    let resp = tokio_test::block_on(provider.get_quote_history("AAPL", start, end)).unwrap();
    let quotes = resp.quotes().unwrap();
    println!("Apple's quotes in January: {:?}", quotes);
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
    let response = tokio_test::block_on(provider.get_quote_range("AAPL", "1d", "1mo")).unwrap();
    let quotes = response.quotes().unwrap();
    println!("Apple's quotes of the last month: {:?}", quotes);
}
```

# Search for a ticker given a search string (e.g. company name):
```rust
use yahoo_finance_api as yahoo;
use tokio_test;

fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    let resp = tokio_test::block_on(provider.search_ticker("Apple")).unwrap();

    let mut apple_found = false;
    println!("All tickers found while searching for 'Apple':");
    for item in resp.quotes 
    {
        println!("{}", item.symbol)
    }
}
```
Some fields like `longname` are only optional and will be replaced by default 
values if missing (e.g. empty string). If you do not like this behavior, 
use `search_ticker_opt` instead which contains `Option<String>` fields, 
returning `None` if the field found missing in the response.

# Time period labels

Time periods are given as strings, combined from the number of periods (except for "ytd" and "max"
and a string label specifying a single period. The following period labels are supported:

| label | description |
|:-----:|:-----------:|
|   m   |   minute    |
|   h   |   hour      |
|   d   |   day       |
|   wk  |   week      |
|   mo  |   month     |
|   y   |   year      |
|  ytd  |  year-to-date |
|  max  |  maximum    |

# Valid parameter combinations

User @satvikpendem, here is a list of supported quote intervals for a given range

| range | interval |
|:-----:|:--------:|
|  1d   | 1m, 2m, 5m, 15m, 30m, 90m, 1h, 1d, 5d, 1wk, 1mo, 3mo |
|  1mo  | 2m, 3m, 5m, 15m, 30m, 90m, 1h, 1d, 5d, 1wk, 1mo, 3mo |
|  3mo  | 1h, 1d, 1wk, 1mo, 3mo |
|  6mo  | 1h, 1d, 1wk, 1mo, 3mo |
|  1y   | 1h, 1d, 1wk, 1mo, 3mo |
|  2y   | 1h, 1d, 1wk, 1mo, 3mo |
|  5y   | 1d, 1wk, 1mo, 3mo |
|  10y   | 1d, 1wk, 1mo, 3mo |
|  ytd   | 1m, 2m, 5m, 15m, 30m, 90m, 1h, 1d, 5d, 1wk, 1mo, 3mo |
|  max   | 1m, 2m, 5m, 15m, 30m, 90m, 1h, 1d, 5d, 1wk, 1mo, 3mo |

