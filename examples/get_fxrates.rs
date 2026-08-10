// Retrieve the historical EUR/USD exchange rate.
use yahoo_finance_api as yahoo;

#[cfg(not(feature = "blocking"))]
#[tokio::main]
async fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    let start = time::OffsetDateTime::UNIX_EPOCH;
    let end = time::OffsetDateTime::now_utc();
    let quote_history = provider
        .get_quote_history("EURUSD=X", start, end)
        .await
        .unwrap();
    println!("Quote history of EUR/USD FX rate:\n{:#?}", quote_history);
}

#[cfg(feature = "blocking")]
fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    let start = time::OffsetDateTime::UNIX_EPOCH;
    let end = time::OffsetDateTime::now_utc();
    let quote_history = provider.get_quote_history("EURUSD=X", start, end).unwrap();
    println!("Quote history of EUR/USD FX rate:\n{:#?}", quote_history);
}
