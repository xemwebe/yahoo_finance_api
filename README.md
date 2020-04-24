# yahoo! finance API
This project provides a set of functions to receive data from the
the [yahoo! finance](https://finance.yahoo.com) website via their API. This project
is licensed under Apache 2.0 or MIT license (see files LICENSE-Apache2.0 and LICENSE-MIT).

There is already an existing rust library [yahoo-finance-rs](https://github.com/fbriden/yahoo-finance-rs),
which I intended to use for my own projects. However, due some issues in the implementation (the library panics
in some cases if yahoo does provide somehow invalid data), I currently can't use it. Once this issue is fixed,
I might switch back and drop development of this library.


Get the latest available quote:
```rust
use yahoo_finance_api as yahoo;
use std::time::{Duration, UNIX_EPOCH};
use chrono::prelude::*;

fn main() {
    let provider = yahoo::YahooConnector::new();
    // get the latest quotes in 1 minute intervals
    let response = provider.get_latest_quotes("AAPL", "1m").unwrap();
    // extract just the latest valid quote summery
    // including timestamp,open,close,high,low,volume
    let quote = response.last_quote().unwrap();
    let time: DateTime<Utc> = 
        DateTime::from(UNIX_EPOCH + Duration::from_secs(quote.timestamp));
    println!("At {} quote price of Apple was {}", time.to_rfc3339(), quote.close);
}
```

Get history of quotes for given time period:
```rust
use yahoo_finance_api as yahoo;
use std::time::{Duration, UNIX_EPOCH};
use chrono::{Utc,TimeZone};

fn main() {
    let provider = yahoo::YahooConnector::new();
    let start = Utc.ymd(2020, 1, 1).and_hms_milli(0, 0, 0, 0);
    let end = Utc.ymd(2020, 1, 31).and_hms_milli(23, 59, 59, 999);
    // returns historic quotes with daily interval
    let resp = provider.get_quote_history("AAPL", start, end).unwrap();
    let quotes = resp.quotes().unwrap();
    println!("Apple's quotes in January: {:?}", quotes);
}
```