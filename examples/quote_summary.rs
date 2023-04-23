#[cfg(not(feature = "blocking"))]
use tokio_test;
use yahoo_finance_api as yahoo;

#[cfg(not(feature = "blocking"))]
fn get_summary() -> Result<yahoo::YQuoteResponse, yahoo::YahooError> {
    let provider = yahoo::YahooConnector::new();
    // get the quote summary for both stocks
    tokio_test::block_on(provider.get_summary(&["AAPL", "IBM"]))
}

#[cfg(feature = "blocking")]
fn get_summary() -> Result<yahoo::YQuoteResponse, yahoo::YahooError> {
    let provider = yahoo::YahooConnector::new();
    // get the quote summary for both stocks
    provider.get_summary(&["AAPL", "IBM"])
}

fn main() {
    let quote_summary = get_summary().unwrap();
    println!("Quote summary of Apple and IBM {:#?}", quote_summary);
}
