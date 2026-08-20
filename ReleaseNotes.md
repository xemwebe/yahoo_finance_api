## Release 5.0.0
### Breaking changes
+ `FinancialData::total_cash`/`total_debt`: `Option<i64>` -> `Option<f64>` (preserves
  fractional values; negative values were already handled). Update code that reads these
  as integers, e.g. `total_cash as i64` no longer compiles.
+ `CompanyOfficer::name`/`title`: `String` -> `Option<String>` (Yahoo returns `null` for
  some officers). Use `name.as_deref()`/`unwrap_or("")` instead of a bare `&name`.
+ `YEarningsColumn::label`: `String` -> `Option<String>`.
+ `AssetProfile::full_time_employees`: `Option<u32>` -> `Option<i64>` (Yahoo has emitted
  negative employee counts).
+ `YMetaData::first_trade_date`: `Option<i32>` -> `Option<i64>` (timestamps before 1970
  or after 2038 overflow `i32`).
+ `SummaryDetail::strike_price`: `Option<u32>` -> `Option<f64>` (options can have
  fractional strikes, e.g. 212.5).
+ `YahooError` gained a new variant `ServerError(String)` (5xx). Code that does an
  exhaustive `match` over `YahooError` must add a `ServerError(_)` arm.
+ `YahooError::EmptyResponse` and `YahooError::HtmlResponse` are also new since the last
  published release (4.2.0) — exhaustive `match` over `YahooError` must handle them too.

### New features
+ new method: `get_ticker_info(symbol)` returns all 20 quoteSummary modules (assetProfile,
  summaryDetail, defaultKeyStatistics, quoteType, financialData, recommendationTrend,
  earningsTrend, earningsHistory, earnings, upgradeDowngradeHistory, calendarEvents,
  insiderHolders, insiderTransactions, majorHoldersBreakdown, institutionOwnership,
  fundOwnership, netSharePurchaseActivity, fundProfile, topHoldings, secFilings)
+ quoteSummary modules tolerate missing or empty list fields (serde default)
+ new method: `YQuoteSummary::from_json` for offline deserialization
+ error handling: quoteSummary API errors (v10) trigger the crumb/Unauthorized retry;
  invalid URLs are reported as `YahooError::InvalidUrl` instead of panicking
+ expand the examples suite (16 examples: ticker_info, quote_range, errors, search_opt, ...)
+ robust deserialization: `Infinity`/`NaN`/`-Infinity` strings and numeric strings in
  quoteSummary `f64` fields are tolerated; placeholder strings (`"N/A"`, `"--"`) become
  `None`; `{"raw": ..., "fmt": ...}` dict values are unwrapped; `null` list fields fall
  back to empty vectors; empty chart results/quote/adjclose blocks return typed errors
  instead of panicking
+ new fields: `SummaryDetail::all_time_high/all_time_low/non_diluted_market_cap`,
  `FundValuation` gains the full category/bond-valuation field set used by yfinance
+ retries and session refresh: `get_ticker_info` and `get_financial_events` refresh the
  crumb+cookie pair on 401/403/5xx/parse errors and on 200-JSON "Invalid Crumb"/"Unauthorized"
  responses; chart requests retry once on empty/HTML/5xx responses
+ error classification: new `YahooError::ServerError` (5xx), `HtmlResponse`, `EmptyResponse`;
  403 is mapped to `Unauthorized` (stale session), 404 on the crumb endpoint to `FetchFailed`;
  HTTP 429 and plain-text "too many requests" bodies are definitive (never retried)
+ `get_financial_events` aggregates all result documents (not just the first) and parses
  numeric-string cells (e.g. `"1.6"`) for EPS fields

## Release 4.2.0
+ Added an optional rate-limiting `governor` feature to gracefully handle request throttling and prevent HTTP 429 rate limits.
+ Support for custom rate limits via `YahooConnectorBuilder::rate_limit()`.
+ extend quotes by field `yield_` (representing Yahoo's yield field)
+ devcontainer added
+ Route all Yahoo API requests through proxy pool
+ Fix: get_crumb, get_ticker_info, get_financial_events now use proxy
+ Fix: cookie header sends only name=value, not raw Set-Cookie attribute
+ Fix: stale crumb in URL after Invalid Crumb refresh
+ Add https_only(true) for security

## Release 4.1.1
+ security update for dependencies
+ update to reqwest 13.4 and thiserror 2.0
+ test_mutual_fund_capital_gains set to ignore since yahou seems to have disabled or change the API with respect to capital gains

## Relese 4.1.0
+ new method: get_financial_events(ticker, limit) - All financial events
+ new method: get_earnings_only(ticker, limit) - Earnings reports only (filters out meetings)

## Release 4.0.0
+ allow negative time stamps for date range before UNIX EPOCH
+ use `rustls` as tls background to avoid problems with API
+ improved support for missing or NaN values
+ remove search for options, since API seems to be discontinued

## Release 3.1.0
+ improved ticker info

## Release 3.0.0
+ improved builder pattern
+ many more fields are now optional
+ using range query to retrive more data
+ add new query `get_ticker_info`

## Release 2.4.0
+ make post-market fields optional, since they are not always returned
+ add Cargo.lock to repository
+ change edition to 2021
+ disable feature rust_decimal (use feature `decimal` to enable use of `rust_decimal`)

## Release 2.3.0
+ refactoring option fetching, all fields are now optional
+ currency `YMetaData` is now optional, since it is not always returned by the API
+ new feature `decimal` to use `rust_decimal` crate for representation of amounts
+ exmample `get_quote` asks user to input quote name
+ fix accidental escape characters in README.md

## Release 2.2.1
+ remove superfluous file `quote_summary.rs` (See note to Release 2.0.0)

## Release 2.2.0
+ specify user agent instead of default
+ add new method `build_with_agent(self, user_agent: &str)` to allow use of custom agent
+ constructor may fail now, returning a Result

## Release 2.1.0d
+ enable to retreive asset metadata
+ enable to fetch capital gains available on Mutual Funds
+ fix: support quote where firstTradeDate equals null
+ fx rate example added

## Release 2.0.1
re-export the time crate

## Release 2.0.0
Breaking change: Method `get_summary` has to be removed, since this is no longer part of the free
API interface of yahoo! finance.

## Release 1.6.1
Documentation update

## Release 1.6.0
The members `mumerator` and `denominator` of struct `Split` has been changed to from `u64` to `f64`.
Most often, these should be small integers, but at least in some cases, the API returns these
values as float. Fractional numerator or denominater seem to be unlikely, but not impossible,
therefore the struct was updated to accept float. Unfortunately, this is breaking change.

## Release 1.5.0
New method add `get_summary` to extract a summary of various data on a list of given quotes.
There is a new example `quote_summary` demonstrating the output.

## Release 1.4.0
Migration from chrono to time

## Release 1.3.0
`unwrap()` removed
Switch to using `thiserror` crate for error propagation
Using `Client` instance of reqwest.
Error message have possibly changed and method `build()` could fail now.
New Feature: Stop request on timeout

## Release 1.2.2
Bug fix in indexation, which in some cases caused failures when fetching the latest quote.

## Release 1.2.1
New example with blocking feature.

## Release 1.2.0
Added support for dividends and stock splits, see the new examples for splits and dividends and some code clean-up.

## Release 1.1.5
Upgrade to version 0.4.* of tokio-test

## Release 1.1.4
Mainly bug fixes and exports added for most structs.
`search_result_opt` has been added, since sometimes not all fields are returned. These has been replaced by `Option<...>` type fields. The interface
of the `search_result` is left untouched, but returns now a default value (e.g.) empty string instead of an error.

## Release 1.1.0
New function supporting search for Quote ticker has been added, which required an additional URL path to access the Yahoo API. The previously single file project has been split up into separate files for improved maintainability. Especially, the blocking and async implementations are now
in separate files.

**Note**: Yahoo-Error type has changed. `FetchFailed` has now a string as argument instead of the status code passed over by `reqwest` to decouple the interface from `reqwest`. The former error code `InvalidStatusCode` has been renamed to `InvalidJson`, which a more proper name since this error is returned if the response could not be read as JSON.

# Release 1.0.0
The library is working stable with and without blocking feature enabled.
