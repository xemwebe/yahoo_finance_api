// Configure the YahooConnector: timeout, user agent, proxy and a custom reqwest client.
use std::time::Duration;
use yahoo_finance_api as yahoo;

#[cfg(not(feature = "blocking"))]
async fn latest_close(provider: &yahoo::YahooConnector) -> yahoo::Decimal {
    provider
        .get_latest_quotes("AAPL", "1d")
        .await
        .unwrap()
        .last_quote()
        .unwrap()
        .close
}

#[cfg(feature = "blocking")]
fn latest_close(provider: &yahoo::YahooConnector) -> yahoo::Decimal {
    provider
        .get_latest_quotes("AAPL", "1d")
        .unwrap()
        .last_quote()
        .unwrap()
        .close
}

#[cfg(not(feature = "blocking"))]
#[tokio::main]
async fn main() {
    // 0. Explicit builder: YahooConnectorBuilder::new() is equivalent to builder()
    let provider = yahoo::YahooConnectorBuilder::new().build().unwrap();
    println!("builder::new: {}", latest_close(&provider).await);

    // 1. Default configuration
    let provider = yahoo::YahooConnector::new().unwrap();
    println!("default: {}", latest_close(&provider).await);

    // 2. Timeout and custom user agent
    let provider = yahoo::YahooConnector::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("my-app/1.0")
        .build()
        .unwrap();
    println!("timeout + user_agent: {}", latest_close(&provider).await);

    // 3. Route requests through a proxy (replace with your proxy address)
    let proxy = reqwest::Proxy::all("http://localhost:8080").unwrap();
    let provider = yahoo::YahooConnector::builder()
        .proxy(proxy)
        .build()
        .unwrap();
    match provider.get_latest_quotes("AAPL", "1d").await {
        Ok(response) => {
            if let Ok(quote) = response.last_quote() {
                println!("proxy: {}", quote.close);
            }
        }
        Err(err) => println!("proxy request failed, skipping: {err:?}"),
    }

    // 4. Custom reqwest client
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap();
    let provider = yahoo::YahooConnectorBuilder::build_with_client(client).unwrap();
    println!("custom client: {}", latest_close(&provider).await);
}

#[cfg(feature = "blocking")]
fn main() {
    // 0. Explicit builder: YahooConnectorBuilder::new() is equivalent to builder()
    let provider = yahoo::YahooConnectorBuilder::new().build().unwrap();
    println!("builder::new: {}", latest_close(&provider));

    // 1. Default configuration
    let provider = yahoo::YahooConnector::new().unwrap();
    println!("default: {}", latest_close(&provider));

    // 2. Timeout and custom user agent
    let provider = yahoo::YahooConnector::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("my-app/1.0")
        .build()
        .unwrap();
    println!("timeout + user_agent: {}", latest_close(&provider));

    // 3. Route requests through a proxy (replace with your proxy address)
    let proxy = reqwest::Proxy::all("http://localhost:8080").unwrap();
    let provider = yahoo::YahooConnector::builder()
        .proxy(proxy)
        .build()
        .unwrap();
    match provider.get_latest_quotes("AAPL", "1d") {
        Ok(response) => {
            if let Ok(quote) = response.last_quote() {
                println!("proxy: {}", quote.close);
            }
        }
        Err(err) => println!("proxy request failed, skipping: {err:?}"),
    }

    // 4. Custom reqwest client
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap();
    let provider = yahoo::YahooConnectorBuilder::build_with_client(client).unwrap();
    println!("custom client: {}", latest_close(&provider));
}
