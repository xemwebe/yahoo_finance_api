#[cfg(not(feature = "blocking"))]
use tokio_test;
use yahoo_finance_api as yahoo;

#[cfg(not(feature = "blocking"))]
fn get_history() -> Result<yahoo::YResponse, yahoo::YahooError> {
    let provider = yahoo::YahooConnector::new();
    let start = time::OffsetDateTime::UNIX_EPOCH;
    let end= time::OffsetDateTime::now_utc();
    tokio_test::block_on(provider.get_quote_history("VTI", start, end))
}

#[cfg(feature = "blocking")]
fn get_history() -> Result<yahoo::YResponse, yahoo::YahooError> {
    let provider = yahoo::YahooConnector::new();
    let start = time::OffsetDateTime::UNIX_EPOCH;
    let end= time::OffsetDateTime::now_utc();
    provider.get_quote_history("VTI", start, end)
}

fn main() {
    let quote_history = get_history().unwrap();
    println!("Quote history of VTI:\n{:#?}", quote_history);
}
