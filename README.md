# yahoo_finance_api

[![crates.io](https://img.shields.io/crates/v/yahoo_finance_api.svg)](https://crates.io/crates/yahoo_finance_api) · [![docs.rs](https://img.shields.io/docsrs/yahoo_finance_api.svg)](https://docs.rs/yahoo_finance_api) · [![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT) · [![CI](https://github.com/xemwebe/yahoo_finance_api/actions/workflows/checks.yml/badge.svg)](https://github.com/xemwebe/yahoo_finance_api/actions/workflows/checks.yml)

A Rust client for the [Yahoo! Finance](https://finance.yahoo.com) API: historical market data, ticker fundamentals, and financial events.

- Historical quotes (OHLCV) with configurable intervals and ranges, including pre/post market data
- Corporate actions: dividends, splits, capital gains
- Ticker fundamentals: 20 `quoteSummary` modules (analyst estimates and recommendations, earnings calendar, holders and insider activity, SEC filings, fund profiles and top holdings)
- Financial events: earnings, meetings, calls
- Ticker search

Async by default; an optional `blocking` feature provides a synchronous API.

## Methods

| method | purpose |
|:-------|:--------|
| `get_latest_quotes(ticker, interval)` | latest quotes |
| `get_quote_history(ticker, start, end)` | quotes for a date range (daily interval) |
| `get_quote_range(ticker, interval, range)` | quotes for a range label (`1mo`, `1y`, ...) |
| `get_quote_history_interval(ticker, start, end, interval)` | quotes for a date range with a custom interval |
| `get_quote_history_interval_prepost(ticker, start, end, interval, prepost)` | same, with pre/post market data |
| `get_quote_period_interval(ticker, range, interval, prepost)` | quotes for a range label with interval and pre/post market data |
| `search_ticker(name)` / `search_ticker_opt(name)` | search for tickers (optional fields kept or replaced by defaults) |
| `get_ticker_info(symbol)` | ticker fundamentals (`quoteSummary` modules) |
| `get_financial_events(ticker, limit)` | earnings, meeting and call dates (max 250) |
| `get_earnings_only(ticker, limit)` | earnings events only |

The `YResponse` returned by the quote methods provides `quotes()`, `last_quote()`, `metadata()`, `dividends()`, `splits()` and `capital_gains()`.

With the `blocking` feature enabled, all of the above methods are available on the blocking connector as well.

## Installation

```toml
[dependencies]
yahoo_finance_api = "4.2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

`tokio` is only needed to run the async examples below.

Minimum supported Rust version: 1.70. See [ReleaseNotes.md](ReleaseNotes.md) for the changelog.

## Cargo features

| feature | description |
|:--------|:------------|
| `blocking` | blocking (non-async) API |
| `governor` | proactive rate limiting, 10 requests/sec by default |
| `decimal` | represent prices as `rust_decimal::Decimal` instead of `f64` |
| `debug` | include the full response body in deserialization error messages |

## Usage

The examples below use `#[tokio::main]`; in a real application, any async runtime works. With the `blocking` feature, the same methods are available without `async`/`await`.

### Quotes

Get the latest quote:

```rust
use yahoo_finance_api as yahoo;
use time::OffsetDateTime;
use tokio;

#[tokio::main]
async fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    let response = provider.get_latest_quotes("AAPL", "1d").await.unwrap();
    let quote = response.last_quote().unwrap();
    let time = OffsetDateTime::from_unix_timestamp(quote.timestamp).unwrap();
    println!("At {} the price of Apple was {}", time, quote.close);
}
```

Get the quote history for a date range, or for a range label and interval:

```rust
use yahoo_finance_api as yahoo;
use time::macros::datetime;
use tokio;

#[tokio::main]
async fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();

    // By start/end dates (daily interval)
    let start = datetime!(2024-1-1 0:00:00.00 UTC);
    let end = datetime!(2024-1-31 23:59:59.99 UTC);
    let resp = provider.get_quote_history("AAPL", start, end).await.unwrap();
    let january_quotes = resp.quotes().unwrap();

    // By range label + interval
    let resp = provider.get_quote_range("AAPL", "1d", "1mo").await.unwrap();
    let last_month = resp.quotes().unwrap();

    // Custom interval, e.g. weekly
    let resp = provider.get_quote_history_interval("AAPL", start, end, "1wk").await.unwrap();

    // Intraday with pre/post market data
    let resp = provider.get_quote_period_interval("AAPL", "5d", "5m", true).await.unwrap();
}
```

### Corporate actions

`YResponse` also exposes the dividends, splits and capital gains recorded in the requested period:

```rust
use yahoo_finance_api as yahoo;
use tokio;

#[tokio::main]
async fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    let resp = provider.get_quote_range("AAPL", "1d", "1y").await.unwrap();
    let dividends = resp.dividends().unwrap();
    let splits = resp.splits().unwrap();
}
```

Ready-to-run examples are in [`examples/`](examples/).

### Ticker info (fundamentals)

`get_ticker_info` fetches detailed fundamental data about a ticker in a single request. The returned `YQuoteSummary` contains one `YSummaryData` per module; each module is `None` if Yahoo did not return it for that ticker (e.g. `fundProfile`/`topHoldings` exist only for funds and ETFs).

```rust
use yahoo_finance_api as yahoo;
use tokio;

#[tokio::main]
async fn main() {
    let mut provider = yahoo::YahooConnector::new().unwrap();
    let result = provider.get_ticker_info("AAPL").await.unwrap();
    let summary = result.quote_summary.unwrap().result.unwrap().remove(0);

    // Company profile, key statistics, financial data, ...
    println!("City: {:?}", summary.asset_profile.unwrap().city);
    // Analyst recommendations and estimates
    println!("Recommendations: {:?}", summary.recommendation_trend.unwrap().trend);
    // Upcoming earnings / dividend dates
    println!("Calendar: {:?}", summary.calendar_events.unwrap());
    // Top institutional holders
    println!("Top holder: {:?}",
        summary.institution_ownership.unwrap().ownership_list[0].organization);
}
```

Available modules (all fields are `Option`):

| module | description |
|:-------|:------------|
| `assetProfile`, `summaryDetail`, `defaultKeyStatistics`, `quoteType`, `financialData` | company profile, valuation and key statistics |
| `recommendationTrend` | analyst recommendation counts (strongBuy/buy/hold/sell/strongSell) |
| `earningsTrend`, `earningsHistory`, `earnings` | analyst estimates, past EPS surprises, earnings charts |
| `upgradeDowngradeHistory` | analyst rating changes |
| `calendarEvents` | upcoming earnings, dividend and ex-dividend dates |
| `insiderHolders`, `insiderTransactions`, `majorHoldersBreakdown`, `institutionOwnership`, `fundOwnership`, `netSharePurchaseActivity` | holders and insider activity |
| `fundProfile`, `topHoldings` | fund metadata and top holdings (funds/ETFs only) |
| `secFilings` | SEC filings list |

Note: for futures, currencies and indexes, Yahoo only returns a small subset of these modules.

### Financial events

Retrieve earnings, meeting and call dates for a ticker (up to 250 events):

```rust
use yahoo_finance_api as yahoo;
use tokio;

#[tokio::main]
async fn main() {
    let mut provider = yahoo::YahooConnector::new().unwrap();
    let events = provider.get_financial_events("AAPL", 100).await.unwrap();
    for event in events {
        println!("{}: estimate {:?}, actual {:?}", event.earnings_date, event.eps_estimate, event.reported_eps);
    }

    // Only earnings events (meetings and calls filtered out):
    let earnings = provider.get_earnings_only("AAPL", 100).await.unwrap();
}
```

### Search

```rust
use yahoo_finance_api as yahoo;
use tokio;

#[tokio::main]
async fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    let resp = provider.search_ticker("Apple").await.unwrap();

    for item in resp.quotes {
        println!("{}", item.symbol);
    }
}
```

Some fields like `longname` are only optional and will be replaced by default values if missing (e.g. empty string). If you do not like this behavior, use `search_ticker_opt` instead, which keeps `Option<String>` fields and returns `None` when a field is missing.

## Rate limiting

To prevent overwhelming the Yahoo! Finance API and avoid getting rate-limited (HTTP 429), enable the optional `governor` feature. This integrates a proactive rate limiter: the library waits before sending a request instead of returning a rate limit error.

```toml
[dependencies]
yahoo_finance_api = { version = "4.2", features = ["governor"] }
```

When enabled, `YahooConnector` defaults to **10 requests per second**. Override or disable it at runtime via the builder:

```rust
use yahoo_finance_api as yahoo;
use std::num::NonZeroU32;

fn main() {
    // 1. Default (10 requests/sec when `governor` feature is active)
    let provider = yahoo::YahooConnector::new().unwrap();

    // 2. Custom limit (e.g. 5 requests/sec)
    let provider = yahoo::YahooConnector::builder()
        .rate_limit(Some(NonZeroU32::new(5).unwrap()))
        .build().unwrap();

    // 3. Explicitly disable rate limiter even when `governor` feature is compiled in
    let provider = yahoo::YahooConnector::builder()
        .rate_limit(None)
        .build().unwrap();
}
```

## Time period labels

Time periods are given as strings, combined from the number of periods (except for "ytd" and "max")
and a string label specifying a single period. The following period labels are supported:

| label | description |
|:-----:|:-----------:|
|   m   |   minute    |
|   h   |   hour      |
|   d   |   day       |
|  wk   |   week      |
|  mo   |   month     |
|   y   |   year      |
|  ytd  |  year-to-date |
|  max  |  maximum    |

## Valid parameter combinations

Supported quote intervals for a given range:

| range | interval |
|:-----:|:--------:|
|  1d   | 1m, 2m, 5m, 15m, 30m, 90m, 1h, 1d, 5d, 1wk, 1mo, 3mo |
|  1mo  | 2m, 3m, 5m, 15m, 30m, 90m, 1h, 1d, 5d, 1wk, 1mo, 3mo |
|  3mo  | 1h, 1d, 1wk, 1mo, 3mo |
|  6mo  | 1h, 1d, 1wk, 1mo, 3mo |
|  1y   | 1h, 1d, 1wk, 1mo, 3mo |
|  2y   | 1h, 1d, 1wk, 1mo, 3mo |
|  5y   | 1d, 1wk, 1mo, 3mo |
|  10y  | 1d, 1wk, 1mo, 3mo |
|  ytd  | 1m, 2m, 5m, 15m, 30m, 90m, 1h, 1d, 5d, 1wk, 1mo, 3mo |
|  max  | 1m, 2m, 5m, 15m, 30m, 90m, 1h, 1d, 5d, 1wk, 1mo, 3mo |

## Error handling

All methods return `Result<_, YahooError>`. The errors fall into a few categories:

- Transport: `ConnectionFailed`, `FetchFailed`
- Yahoo API: `ApiError`, `Unauthorized`, `InvalidCrumb`, `InvalidCookie`, `NoCookies`, `TooManyRequests`
- Empty or inconsistent data: `NoResult`, `NoQuotes`, `DataInconsistency`, `MissingField`
- Deserialization: `DeserializeFailed`; with the `debug` feature, `DeserializeFailedDebug` includes the full response body

## Contributing

Interested in contributing? See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-Apache2.0) or [MIT license](LICENSE-MIT), at your option.
