#[cfg(not(feature = "blocking"))]
use tokio_test;
use yahoo_finance_api as yahoo;

#[cfg(not(feature = "blocking"))]
fn search_apple_options() {
    let provider = yahoo::YahooConnector::new().unwrap();
    let resp = tokio_test::block_on(provider.search_options("AAPL")).unwrap();

    println!("All options found on stock 'AAPL':");
    for item in resp.calls {
        println!(
            "name: {}, strike: {}, last trade date: {}",
            item.contract_symbol, item.strike, item.last_trade_date
        );
    }
}

#[cfg(feature = "blocking")]
fn search_apple_options() {
    let provider = yahoo::YahooConnector::new().unwrap();
    let resp = provider.search_options("AAPL").unwrap();

    println!("All options found on stock 'AAPL':");
    for item in resp.calls {
        println!(
            "name: {}, strike: {}, last trade date: {}",
            item.contract_symbol, item.strike, item.last_trade_date
        );
    }
}

fn main() {
    search_apple_options();
}
