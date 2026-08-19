//! # yahoo! finance API
//!
//! This project provides a set of functions to receive data from the
//! [yahoo! finance](https://finance.yahoo.com) website via their API. This project
//! is licensed under Apache 2.0 or MIT license (see files LICENSE-Apache2.0 and LICENSE-MIT).
//!
//! Since version 0.3, all requests to the yahoo API return futures, using `async` features (the upgrade to `reqwest` 0.13 arrived in 4.1.1).
//! Therefore, the functions need to be called from within another ```async``` function with ```.await``` (e.g. via `#[tokio::main]`).
//! The examples below are based on the ```tokio``` runtime.
//!
//! Use the `blocking` feature to get the previous behavior back: i.e. `yahoo_finance_api = {"version": "4.2", features = ["blocking"]}`.
//!
//! # Features
//!
//! - `blocking`: provide a blocking (non-async) API via the `blocking_impl` module.
//! - `governor`: rate-limit requests to avoid HTTP 429 responses. Defaults to
//!   10 requests/second; configure via `YahooConnectorBuilder::rate_limit`.
//! - `decimal`: represent prices as `rust_decimal::Decimal` instead of `f64`.
//! - `debug`: include the full response body in deserialization error
//!   messages (only when the error contains "expected value").
//!
#![cfg_attr(
    not(feature = "blocking"),
    doc = "
# Get the latest available quote:
```ignore
use yahoo_finance_api as yahoo;
use time::OffsetDateTime;
use tokio;

#[tokio::main]
async fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    // get the latest quotes with the given interval
    let response = provider.get_latest_quotes(\"AAPL\", \"1d\").await.unwrap();
    // extract just the latest valid quote summary
    // including timestamp,open,close,high,low,volume
    let quote = response.last_quote().unwrap();
    let time: OffsetDateTime =
        OffsetDateTime::from_unix_timestamp(quote.timestamp).unwrap();
    println!(\"At {} quote price of Apple was {}\", time, quote.close);
}
```
# Get history of quotes for given time period:
```ignore
use yahoo_finance_api as yahoo;
use time::macros::datetime;
use tokio;

#[tokio::main]
async fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    let start = datetime!(2020-1-1 0:00:00.00 UTC);
    let end = datetime!(2020-1-31 23:59:59.99 UTC);
    // returns historic quotes with daily interval
    let resp = provider.get_quote_history(\"AAPL\", start, end).await.unwrap();
    let quotes = resp.quotes().unwrap();
    println!(\"Apple's quotes in January: {:?}\", quotes);
}
```
# Get the history of quotes for time range
Another method to retrieve a range of quotes is by requesting the quotes for a given period and
lookup frequency. Here is an example retrieving the daily quotes for the last month:
```ignore
use yahoo_finance_api as yahoo;
use tokio;

#[tokio::main]
async fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    let response = provider.get_quote_range(\"AAPL\", \"1d\", \"1mo\").await.unwrap();
    let quotes = response.quotes().unwrap();
    println!(\"Apple's quotes of the last month: {:?}\", quotes);
}
```

# Search for a ticker given a search string (e.g. company name):
```ignore
use yahoo_finance_api as yahoo;
use tokio;

#[tokio::main]
async fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    let resp = provider.search_ticker(\"Apple\").await.unwrap();

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
```ignore
use yahoo_finance_api as yahoo;
use time::OffsetDateTime;

fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    // get the latest quotes with the given interval
    let response = provider.get_latest_quotes(\"AAPL\", \"1d\").unwrap();
    // extract just the latest valid quote summary
    // including timestamp,open,close,high,low,volume
    let quote = response.last_quote().unwrap();
    let time: OffsetDateTime =
        OffsetDateTime::from_unix_timestamp(quote.timestamp).unwrap();
    println!(\"At {} quote price of Apple was {}\", time, quote.close);
}
```
# Get history of quotes for given time period:
```ignore
use yahoo_finance_api as yahoo;
use time::macros::datetime;

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
```ignore
use yahoo_finance_api as yahoo;

fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    let response = provider.get_quote_range(\"AAPL\", \"1d\", \"1mo\").unwrap();
    let quotes = response.quotes().unwrap();
    println!(\"Apple's quotes of the last month: {:?}\", quotes);
}
```
# Search for a ticker given a search string (e.g. company name):
```ignore
use yahoo_finance_api as yahoo;

fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    let resp = provider.search_ticker(\"Apple\").unwrap();

    println!(\"All tickers found while searching for 'Apple':\");
    for item in resp.quotes
    {
        println!(\"{}\", item.symbol)
    }
}
```
"
)]

#[cfg(feature = "debug")]
extern crate serde_json_path_to_error as serde_json;

use std::time::Duration;
use time::OffsetDateTime;

#[cfg(feature = "governor")]
use std::{num::NonZeroU32, sync::Arc};

#[cfg(feature = "blocking")]
use reqwest::blocking::{Client, ClientBuilder};
use reqwest::Proxy;
#[cfg(not(feature = "blocking"))]
use reqwest::{Client, ClientBuilder};

#[cfg(feature = "governor")]
use governor::{
    clock::DefaultClock,
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};

#[cfg(feature = "governor")]
type YRateLimiter = RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>;

// re-export time crate
pub use quotes::decimal::Decimal;
pub use time;

mod quotes;
mod response;
mod search_result;
mod yahoo_error;
pub use quotes::{
    AdjClose, AssetProfile, CalendarEarnings, CalendarEvents, CapitalGain, CurrentTradingPeriod,
    DefaultKeyStatistics, Dividend, Earnings, EarningsChart, EarningsChartQuarterly,
    EarningsEstimate, EarningsHistory, EarningsHistoryItem, EarningsTrend, EarningsTrendItem,
    EpsRevisions, EpsTrend, ExtendedQuoteSummary, FinancialData, FinancialEvent, FinancialsChart,
    FinancialsChartQuarterly, FinancialsChartYearly, FundFeesExpenses, FundManagementInfo,
    FundOwnership, FundOwnershipItem, FundProfile, FundValuation, GrowthEstimate, InsiderHolder,
    InsiderHolders, InsiderTransaction, InsiderTransactions, InstitutionOwnership,
    InstitutionOwnershipItem, MajorHoldersBreakdown, NetSharePurchaseActivity, PeriodInfo, Quote,
    QuoteBlock, QuoteList, QuoteType, RawValue, RecommendationTrend, RecommendationTrendItem,
    RevenueEstimate, SecFiling, SecFilingExhibit, SecFilings, Split, SummaryDetail, TopHolding,
    TopHoldings, TradingPeriods, UpgradeDowngradeHistory, UpgradeDowngradeItem, YChart, YMetaData,
    YQuoteBlock, YQuoteSummary, YResponse, YSummaryData,
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
const Y_EARNINGS_URL: &str = "https://query1.finance.yahoo.com/v1/finance/visualization";

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
        "https://query2.finance.yahoo.com/v10/finance/quoteSummary/{symbol}?modules=financialData,quoteType,defaultKeyStatistics,assetProfile,summaryDetail,recommendationTrend,earningsTrend,earningsHistory,earnings,upgradeDowngradeHistory,calendarEvents,insiderHolders,insiderTransactions,majorHoldersBreakdown,institutionOwnership,fundOwnership,netSharePurchaseActivity,fundProfile,topHoldings,secFilings&corsDomain=finance.yahoo.com&formatted=false&symbol={symbol}&crumb={crumb}"
    }
}
macro_rules! YEARNINGS_QUERY {
    () => {
        "{url}?lang={lang}&region={region}&crumb={crumb}"
    };
}

/// Container for connection parameters to yahoo! finance server
pub struct YahooConnector {
    client: Client,
    url: &'static str,
    search_url: &'static str,
    cookie: Option<String>,
    crumb: Option<String>,
    #[cfg(feature = "governor")]
    rate_limiter: Option<Arc<YRateLimiter>>,
}

/// Builder for configuring a [`YahooConnector`] (timeout, user agent, proxy,
/// rate limit) before the HTTP client is created. Start with
/// [`YahooConnectorBuilder::new`] or [`YahooConnector::builder`], then call
/// [`YahooConnectorBuilder::build`].
#[derive(Default)]
pub struct YahooConnectorBuilder {
    inner: ClientBuilder,
    timeout: Option<Duration>,
    user_agent: Option<String>,
    proxy: Option<Proxy>,
    #[cfg(feature = "governor")]
    rate_limit_per_second: Option<Option<NonZeroU32>>,
}

impl YahooConnector {
    /// Constructor for a new instance of the yahoo connector.
    pub fn new() -> Result<YahooConnector, YahooError> {
        Self::builder().build()
    }

    /// Create a new [`YahooConnectorBuilder`] to configure the connector
    /// (timeout, user agent, proxy, rate limiting) before building it.
    pub fn builder() -> YahooConnectorBuilder {
        YahooConnectorBuilder {
            inner: Client::builder(),
            user_agent: Some(USER_AGENT.to_string()),
            ..Default::default()
        }
    }

    /// Internal default implementation used exclusively by the builder.
    /// Note: This default implementation does not set the user agent in the client,
    /// so it does not work on its own. The builder will set the user agent.
    fn default_internal() -> Self {
        YahooConnector {
            client: Client::default(),
            url: YCHART_URL,
            search_url: YSEARCH_URL,
            cookie: None,
            crumb: None,
            #[cfg(feature = "governor")]
            rate_limiter: None,
        }
    }
}

impl YahooConnectorBuilder {
    /// Create a new builder with the default settings.
    pub fn new() -> Self {
        YahooConnector::builder()
    }

    /// Set the maximum time a single request may take before being aborted.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set a custom `User-Agent` header for all requests.
    pub fn user_agent(mut self, user_agent: &str) -> Self {
        self.user_agent = Some(user_agent.to_string());
        self
    }

    /// Route all requests through the given HTTP proxy.
    pub fn proxy(mut self, proxy: Proxy) -> Self {
        self.proxy = Some(proxy);
        self
    }

    /// Set the maximum number of requests per second. Default is 10 requests/second when the
    /// `governor` feature is enabled. Pass `None` to disable rate limiting even when the feature is enabled.
    #[cfg(feature = "governor")]
    pub fn rate_limit(mut self, requests_per_second: Option<NonZeroU32>) -> Self {
        self.rate_limit_per_second = Some(requests_per_second);
        self
    }

    /// Build the [`YahooConnector`] with the configured options.
    pub fn build(mut self) -> Result<YahooConnector, YahooError> {
        if let Some(timeout) = &self.timeout {
            self.inner = self.inner.timeout(*timeout);
        }
        if let Some(user_agent) = &self.user_agent {
            self.inner = self.inner.user_agent(user_agent.clone());
        }
        if let Some(proxy) = &self.proxy {
            self.inner = self.inner.proxy(proxy.clone());
        }
        self.inner = self.inner.https_only(true);

        #[cfg(feature = "governor")]
        let rate_limiter = {
            // `rate_limit_per_second` is Option<Option<NonZeroU32>>:
            // None = unconfigured (defaults to 10 rps), Some(None) = explicitly disabled, Some(Some(n)) = n rps.
            let rps_option = self
                .rate_limit_per_second
                .unwrap_or_else(|| NonZeroU32::new(10));
            rps_option.map(|rps| Arc::new(RateLimiter::direct(Quota::per_second(rps))))
        };

        Ok(YahooConnector {
            client: self.inner.build()?,
            #[cfg(feature = "governor")]
            rate_limiter,
            ..YahooConnector::default_internal()
        })
    }

    /// Build a `YahooConnector` using a custom pre-configured `reqwest::Client`.
    ///
    /// When the `governor` feature is enabled, a default rate limit of 10 requests/second
    /// is applied. To customize or disable the rate limit with a custom client, use
    /// `YahooConnector::builder().rate_limit(...).build()` instead.
    pub fn build_with_client(client: Client) -> Result<YahooConnector, YahooError> {
        #[cfg(feature = "governor")]
        let rate_limiter = Some(Arc::new(RateLimiter::direct(Quota::per_second(
            NonZeroU32::new(10).unwrap(),
        ))));

        Ok(YahooConnector {
            client,
            #[cfg(feature = "governor")]
            rate_limiter,
            ..YahooConnector::default_internal()
        })
    }
}

#[cfg(not(feature = "blocking"))]
pub mod async_impl;

#[cfg(feature = "blocking")]
pub mod blocking_impl;
