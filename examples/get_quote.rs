use yahoo_finance_api as yahoo;
#[cfg(not(feature="blocking"))]
use tokio_test;

#[cfg(not(feature="blocking"))]
fn get_quote() -> Result<f64, yahoo::YahooError> {
    let provider = yahoo::YahooConnector::new();
    // get the latest quotes in 1 minute intervals
    let response = tokio_test::block_on(provider.get_latest_quotes("AAPL", "1m")).unwrap();
    // extract just the latest valid quote summery
    let quote = response.last_quote()?;
    Ok(quote.close)
}

#[cfg(feature="blocking")]
fn get_quote() -> Result<f64, yahoo::YahooError> {
    let provider = yahoo::YahooConnector::new();
    // get the latest quotes in 1 minute intervals
    let response = provider.get_latest_quotes("AAPL", "1m").unwrap();
    // extract just the latest valid quote summery
    let quote = response.last_quote()?;
    Ok(quote.close)
}


fn main() {
    let quote = get_quote().unwrap();
    println!("Most recent price of Apple is {}", quote);
}
