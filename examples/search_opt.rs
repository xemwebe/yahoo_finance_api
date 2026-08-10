// Search for tickers, keeping the Option fields of the result.
use yahoo_finance_api as yahoo;

#[cfg(not(feature = "blocking"))]
#[tokio::main]
async fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    let result = provider.search_ticker_opt("Apple").await.unwrap();
    println!("{} hits:", result.count);

    // YQuoteItemOpt keeps short_name/long_name as Option, because Yahoo
    // may omit them for some entries
    for hit in &result.quotes {
        println!(
            "{} ({}) | short: {:?} | long: {:?}",
            hit.symbol, hit.exchange, hit.short_name, hit.long_name
        );
    }

    println!("{} news items", result.news.len());
    for item in &result.news {
        println!("- {}", item.title);
    }
}

#[cfg(feature = "blocking")]
fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();
    let result = provider.search_ticker_opt("Apple").unwrap();
    println!("{} hits:", result.count);

    // YQuoteItemOpt keeps short_name/long_name as Option, because Yahoo
    // may omit them for some entries
    for hit in &result.quotes {
        println!(
            "{} ({}) | short: {:?} | long: {:?}",
            hit.symbol, hit.exchange, hit.short_name, hit.long_name
        );
    }

    println!("{} news items", result.news.len());
    for item in &result.news {
        println!("- {}", item.title);
    }
}
