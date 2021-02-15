#[cfg(not(feature = "blocking"))]
use tokio_test;
use yahoo_finance_api as yahoo;

#[cfg(not(feature = "blocking"))]
fn search_apple() {
    let provider = yahoo::YahooConnector::new();
    let resp = tokio_test::block_on(provider.search_ticker("AAPL")).unwrap();

    println!("All tickers found while searching for 'Apple':");
    for item in resp.quotes {
        println!("{}", item.symbol)
    }
}

#[cfg(feature = "blocking")]
fn search_apple() {
    let provider = yahoo::YahooConnector::new();
    let resp = provider.search_ticker("AAPL").unwrap();

    println!("All tickers found while searching for 'Apple':");
    for item in resp.quotes {
        println!("{}", item.symbol)
    }
}

fn main() {
    search_apple();
}
