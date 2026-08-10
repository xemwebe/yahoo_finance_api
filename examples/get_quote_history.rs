// Retrieve the full quote history of a ticker.
use yahoo_finance_api as yahoo;

#[cfg(not(feature = "blocking"))]
#[tokio::main]
async fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    let start = time::OffsetDateTime::UNIX_EPOCH;
    let end = time::OffsetDateTime::now_utc();
    let quote_history = provider
        .get_quote_history("VTI", start, end)
        .await
        .unwrap();
    println!("Quote history of VTI:\n{:#?}", quote_history);
}

#[cfg(feature = "blocking")]
fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    let start = time::OffsetDateTime::UNIX_EPOCH;
    let end = time::OffsetDateTime::now_utc();
    let quote_history = provider.get_quote_history("VTI", start, end).unwrap();
    println!("Quote history of VTI:\n{:#?}", quote_history);
}
