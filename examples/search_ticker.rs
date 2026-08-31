// Search for tickers by name.
use yahoo_finance_api as yahoo;

#[cfg(not(feature = "blocking"))]
#[tokio::main]
async fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    let resp = provider.search_ticker("Apple").await.unwrap();

    println!("All tickers found while searching for 'Apple':");
    for item in resp.quotes {
        println!("{}", item.symbol)
    }
}

#[cfg(feature = "blocking")]
fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    let resp = provider.search_ticker("Apple").unwrap();

    println!("All tickers found while searching for 'Apple':");
    for item in resp.quotes {
        println!("{}", item.symbol)
    }
}
