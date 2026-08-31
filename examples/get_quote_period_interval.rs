// Retrieve intraday quotes (1-minute interval, including pre/post market data) for a ticker.
use yahoo_finance_api as yahoo;

#[cfg(not(feature = "blocking"))]
#[tokio::main]
async fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    let quote_history = provider
        .get_quote_period_interval("AAPL", "1d", "1m", true)
        .await
        .unwrap();
    println!("Quote history of AAPL:\n{:#?}", quote_history);
}

#[cfg(feature = "blocking")]
fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    let quote_history = provider
        .get_quote_period_interval("AAPL", "1d", "1m", true)
        .unwrap();
    println!("Quote history of AAPL:\n{:#?}", quote_history);
}
