use std::io::Write;
use yahoo::Decimal;
use yahoo_finance_api as yahoo;

#[cfg(not(feature = "blocking"))]
fn get_quote(name: &str) -> Result<Decimal, yahoo::YahooError> {
    let provider = yahoo::YahooConnector::new().unwrap();
    // get the latest quotes in 1 minute intervals
    let response = tokio_test::block_on(provider.get_latest_quotes(name, "1d")).unwrap();
    // extract just the latest valid quote summery
    let quote = response.last_quote()?;
    Ok(quote.close)
}

#[cfg(feature = "blocking")]
fn get_quote(name: &str) -> Result<Decimal, yahoo::YahooError> {
    let provider = yahoo::YahooConnector::new().unwrap();
    // get the latest quotes in 1 minute intervals
    let response = provider.get_latest_quotes(name, "1d").unwrap();
    // extract just the latest valid quote summery
    let quote = response.last_quote()?;
    Ok(quote.close)
}

fn main() {
    print!("Please enter a quote name: ");
    std::io::stdout().lock().flush().unwrap();
    let mut quote_name = String::new();
    std::io::stdin().read_line(&mut quote_name).unwrap();
    let quote_name = quote_name.trim();
    let quote = get_quote(&quote_name).unwrap();
    println!("Most recent price of {quote_name} is {quote}");
}
