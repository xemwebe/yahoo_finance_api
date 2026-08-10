// Handle the YahooError variants returned by the API.
use yahoo_finance_api as yahoo;

fn report(err: &yahoo::YahooError) {
    match err {
        yahoo::YahooError::ApiError(msg) => println!(
            "  ApiError: {}",
            msg.description.as_deref().unwrap_or("no description")
        ),
        yahoo::YahooError::NoResult => println!("  NoResult: the API returned no data"),
        yahoo::YahooError::NoQuotes => println!("  NoQuotes: no valid quotes in the response"),
        other => println!("  Other: {:?}", other),
    }
}

#[cfg(not(feature = "blocking"))]
#[tokio::main]
async fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();

    // 1. Success path
    match provider.get_quote_range("AAPL", "1d", "5d").await {
        Ok(hist) => println!("AAPL: {} bars", hist.quotes().unwrap().len()),
        Err(err) => report(&err),
    }

    // 2. Unknown ticker: Yahoo usually replies with an error payload
    match provider.get_quote_range("NOT_A_REAL_TICKER", "1d", "5d").await {
        Ok(_) => println!("unexpected success"),
        Err(err) => report(&err),
    }

    // 3. Unsupported combination: intraday 1m interval over a long period
    //    (usually rejected by the API)
    match provider.get_quote_range("AAPL", "1m", "1y").await {
        Ok(_) => println!("unexpected success"),
        Err(err) => report(&err),
    }

    // 4. NoResult without the network: quotes() on an empty chart result
    let json = serde_json::json!({"chart": {"result": null, "error": null}});
    let response = yahoo::YResponse::from_json(json).unwrap();
    if let Err(err) = response.quotes() {
        report(&err);
    }
}

#[cfg(feature = "blocking")]
fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();

    // 1. Success path
    match provider.get_quote_range("AAPL", "1d", "5d") {
        Ok(hist) => println!("AAPL: {} bars", hist.quotes().unwrap().len()),
        Err(err) => report(&err),
    }

    // 2. Unknown ticker: Yahoo usually replies with an error payload
    match provider.get_quote_range("NOT_A_REAL_TICKER", "1d", "5d") {
        Ok(_) => println!("unexpected success"),
        Err(err) => report(&err),
    }

    // 3. Unsupported combination: intraday 1m interval over a long period
    //    (usually rejected by the API)
    match provider.get_quote_range("AAPL", "1m", "1y") {
        Ok(_) => println!("unexpected success"),
        Err(err) => report(&err),
    }

    // 4. NoResult without the network: quotes() on an empty chart result
    let json = serde_json::json!({"chart": {"result": null, "error": null}});
    let response = yahoo::YResponse::from_json(json).unwrap();
    if let Err(err) = response.quotes() {
        report(&err);
    }
}
