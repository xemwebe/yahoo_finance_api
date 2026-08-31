// Retrieve ticker fundamentals (quoteSummary modules) for a symbol.
use yahoo_finance_api as yahoo;

fn print_summary(summary: &yahoo::YSummaryData) {
    // Not all modules are present for every asset type
    // (e.g. funds, crypto and indices have no assetProfile)
    if let Some(profile) = &summary.asset_profile {
        println!("City: {:?}", profile.city);
    }
    if let Some(calendar) = &summary.calendar_events {
        println!("Calendar: {:?}", calendar);
    }
    if let Some(holders) = &summary.institution_ownership {
        if let Some(top) = holders.ownership_list.first() {
            println!("Top institutional holder: {:?}", top.organization);
        }
    }
}

#[cfg(not(feature = "blocking"))]
#[tokio::main]
async fn main() {
    let mut provider = yahoo::YahooConnector::new().unwrap();
    let result = provider.get_ticker_info("AAPL").await.unwrap();

    let Some(summary) = result
        .quote_summary
        .and_then(|q| q.result)
        .and_then(|mut r| r.pop())
    else {
        println!("no quote summary in response");
        return;
    };
    print_summary(&summary);
}

#[cfg(feature = "blocking")]
fn main() {
    let mut provider = yahoo::YahooConnector::new().unwrap();
    let result = provider.get_ticker_info("AAPL").unwrap();

    let Some(summary) = result
        .quote_summary
        .and_then(|q| q.result)
        .and_then(|mut r| r.pop())
    else {
        println!("no quote summary in response");
        return;
    };
    print_summary(&summary);
}
