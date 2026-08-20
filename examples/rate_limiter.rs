// Rate limiting with the `governor` feature.
#[cfg(feature = "governor")]
use std::num::NonZeroU32;
#[cfg(feature = "governor")]
use yahoo_finance_api as yahoo;

#[cfg(all(feature = "governor", not(feature = "blocking")))]
#[tokio::main]
async fn main() {
    // 1. Default: 10 requests/sec when `governor` is enabled
    let provider = yahoo::YahooConnector::new().unwrap();
    let _ = provider.get_latest_quotes("AAPL", "1d").await.unwrap();

    // 2. Custom limit (e.g. 5 requests/sec)
    let provider = yahoo::YahooConnector::builder()
        .rate_limit(Some(NonZeroU32::new(5).unwrap()))
        .build()
        .unwrap();
    let _ = provider.get_latest_quotes("AAPL", "1d").await.unwrap();

    // 3. Explicitly disable the rate limiter even when `governor` is compiled in
    let provider = yahoo::YahooConnector::builder()
        .rate_limit(None)
        .build()
        .unwrap();
    let _ = provider.get_latest_quotes("AAPL", "1d").await.unwrap();
}

#[cfg(all(feature = "governor", feature = "blocking"))]
fn main() {
    // 1. Default: 10 requests/sec when `governor` is enabled
    let provider = yahoo::YahooConnector::new().unwrap();
    let _ = provider.get_latest_quotes("AAPL", "1d").unwrap();

    // 2. Custom limit (e.g. 5 requests/sec)
    let provider = yahoo::YahooConnector::builder()
        .rate_limit(Some(NonZeroU32::new(5).unwrap()))
        .build()
        .unwrap();
    let _ = provider.get_latest_quotes("AAPL", "1d").unwrap();

    // 3. Explicitly disable the rate limiter even when `governor` is compiled in
    let provider = yahoo::YahooConnector::builder()
        .rate_limit(None)
        .build()
        .unwrap();
    let _ = provider.get_latest_quotes("AAPL", "1d").unwrap();
}

#[cfg(not(feature = "governor"))]
fn main() {
    println!("Enable the `governor` feature to run this example.");
}
