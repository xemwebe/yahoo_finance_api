#[cfg(not(feature = "blocking"))]
use tokio_test;
use yahoo_finance_api as yahoo;

#[cfg(not(feature = "blocking"))]
fn search_apple_options() {
    let provider = yahoo::YahooConnector::new();
    let resp = tokio_test::block_on(provider.search_options("AAPL")).unwrap();

    println!("All options found on stock 'AAPL':");
    for item in resp.options {
        println!(
            "name: {}, strike: {}, last trade date: {}",
            item.name, item.strike, item.last_trade_date
        );
    }
}

#[cfg(feature = "blocking")]
fn search_apple_options() {
    let provider = yahoo::YahooConnector::new();
    let resp = provider.search_options("AAPL").unwrap();

    println!("All options found on stock 'AAPL':");
    for item in resp.options {
        println!(
            "name: {}, strike: {}, last trade date: {}",
            item.name, item.strike, item.last_trade_date
        );
    }
}

fn main() {
    search_apple_options();
}
