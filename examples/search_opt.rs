// Search for tickers, keeping the Option fields of the result.
use yahoo_finance_api as yahoo;

fn offline_demo() {
    // Without the network: deserialize a canned search response that
    // omits shortname, then convert it to the default-filled variant
    let json = serde_json::json!({
        "count": 1,
        "quotes": [{
            "exchange": "NMS",
            "shortname": null,
            "quoteType": "EQUITY",
            "symbol": "AAPL",
            "index": "EQUITY",
            "score": 1.0,
            "typeDisp": "Equity",
            "longname": "Apple Inc.",
            "isYahooFinance": true
        }],
        "news": []
    });
    let opt = yahoo::YSearchResultOpt::from_json(json).unwrap();
    let Some(hit) = opt.quotes.first() else {
        println!("no search hits in canned response");
        return;
    };
    println!(
        "opt: short_name={:?} long_name={:?}",
        hit.short_name, hit.long_name
    );

    // YSearchResult::from_opt replaces the missing fields with defaults
    let filled = yahoo::YSearchResult::from_opt(&opt);
    let hit = filled.quotes.first().unwrap();
    println!(
        "from_opt: short_name={:?} long_name={:?}",
        hit.short_name, hit.long_name
    );
}

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

    offline_demo();
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

    offline_demo();
}
