// Print the metadata attached to a chart response.
use yahoo_finance_api as yahoo;

#[cfg(not(feature = "blocking"))]
#[tokio::main]
async fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    let hist = provider.get_quote_range("AAPL", "1d", "5d").await.unwrap();

    let meta = hist.metadata().unwrap();
    println!("symbol: {}", meta.symbol);
    println!("long name: {:?}", meta.long_name);
    println!("currency: {:?}", meta.currency);
    println!("instrument: {}", meta.instrument_type);
    println!("exchange: {}", meta.exchange_name);
    println!("timezone: {}", meta.timezone);
}

#[cfg(feature = "blocking")]
fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    let hist = provider.get_quote_range("AAPL", "1d", "5d").unwrap();

    let meta = hist.metadata().unwrap();
    println!("symbol: {}", meta.symbol);
    println!("long name: {:?}", meta.long_name);
    println!("currency: {:?}", meta.currency);
    println!("instrument: {}", meta.instrument_type);
    println!("exchange: {}", meta.exchange_name);
    println!("timezone: {}", meta.timezone);
}
