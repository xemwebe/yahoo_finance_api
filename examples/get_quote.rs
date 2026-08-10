// Retrieve the latest quote for a user-provided ticker symbol.
use std::io::Write;
use yahoo::Decimal;
use yahoo_finance_api as yahoo;

#[cfg(not(feature = "blocking"))]
async fn get_quote(name: &str) -> Result<Decimal, yahoo::YahooError> {
    let provider = yahoo::YahooConnector::new()?;
    // get the latest quotes with the given interval
    let response = provider.get_latest_quotes(name, "1d").await?;
    // extract just the latest valid quote summary
    let quote = response.last_quote()?;
    Ok(quote.close)
}

#[cfg(feature = "blocking")]
fn get_quote(name: &str) -> Result<Decimal, yahoo::YahooError> {
    let provider = yahoo::YahooConnector::new()?;
    // get the latest quotes with the given interval
    let response = provider.get_latest_quotes(name, "1d")?;
    // extract just the latest valid quote summary
    let quote = response.last_quote()?;
    Ok(quote.close)
}

#[cfg(not(feature = "blocking"))]
#[tokio::main]
async fn main() {
    print!("Please enter a quote name: ");
    std::io::stdout().lock().flush().unwrap();
    let mut quote_name = String::new();
    std::io::stdin().read_line(&mut quote_name).unwrap();
    let quote_name = quote_name.trim();
    let quote = get_quote(quote_name).await.unwrap();
    println!("Most recent price of {quote_name} is {quote}");
}

#[cfg(feature = "blocking")]
fn main() {
    print!("Please enter a quote name: ");
    std::io::stdout().lock().flush().unwrap();
    let mut quote_name = String::new();
    std::io::stdin().read_line(&mut quote_name).unwrap();
    let quote_name = quote_name.trim();
    let quote = get_quote(quote_name).unwrap();
    println!("Most recent price of {quote_name} is {quote}");
}
