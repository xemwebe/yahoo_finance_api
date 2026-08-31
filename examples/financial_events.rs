// Retrieve earnings, meeting and call dates for a ticker.
use yahoo_finance_api as yahoo;

#[cfg(not(feature = "blocking"))]
#[tokio::main]
async fn main() {
    let mut provider = yahoo::YahooConnector::new().unwrap();
    let events = provider.get_financial_events("AAPL", 10).await.unwrap();
    for event in events {
        println!(
            "{} | {} | estimate: {:?} | actual: {:?}",
            event.earnings_date, event.event_type, event.eps_estimate, event.reported_eps
        );
    }

    // Only earnings events (meetings and calls filtered out)
    let earnings = provider.get_earnings_only("AAPL", 10).await.unwrap();
    println!("Earnings events: {}", earnings.len());
}

#[cfg(feature = "blocking")]
fn main() {
    let mut provider = yahoo::YahooConnector::new().unwrap();
    let events = provider.get_financial_events("AAPL", 10).unwrap();
    for event in events {
        println!(
            "{} | {} | estimate: {:?} | actual: {:?}",
            event.earnings_date, event.event_type, event.eps_estimate, event.reported_eps
        );
    }

    // Only earnings events (meetings and calls filtered out)
    let earnings = provider.get_earnings_only("AAPL", 10).unwrap();
    println!("Earnings events: {}", earnings.len());
}
