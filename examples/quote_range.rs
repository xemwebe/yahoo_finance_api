// Retrieve quotes with a shorthand range or an explicit start/end period.
use time::macros::datetime;
use time::OffsetDateTime;

use yahoo_finance_api as yahoo;

#[cfg(not(feature = "blocking"))]
#[tokio::main]
async fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();

    // 1. Shorthand: a range string instead of explicit dates
    let hist = provider
        .get_quote_range("AAPL", "1d", "5d")
        .await
        .unwrap();
    println!("range 5d: {} bars", hist.quotes().unwrap().len());

    // 2. Explicit start/end period with a custom interval
    let start = datetime!(2024-01-02 00:00:00.00 UTC);
    let end = datetime!(2024-01-12 00:00:00.00 UTC);
    let hist = provider
        .get_quote_history_interval("AAPL", start, end, "1d")
        .await
        .unwrap();
    println!("interval 1d: {} bars", hist.quotes().unwrap().len());

    // 3. Same period with an intraday interval, also including
    //    pre/post market bars. Note that with interval="1d" the pre/post
    //    flag has no effect - it only applies to intraday intervals.
    //    Intraday data is only served for the last 730 days, so use a
    //    recent window.
    let end = time::OffsetDateTime::now_utc();
    let start = end - time::Duration::days(4);
    let hist = provider
        .get_quote_history_interval_prepost("AAPL", start, end, "1h", true)
        .await
        .unwrap();
    println!("interval 1h + prepost: {} bars", hist.quotes().unwrap().len());

    // 4. With prepost=true the bars cover the pre-market (from 04:00 ET)
    //    and after-hours (until 20:00 ET) sessions, extending beyond the
    //    regular 09:30-16:00 ET session
    let quotes = hist.quotes().unwrap();
    if let Some(first) = quotes.first() {
        println!(
            "first bar: {}",
            OffsetDateTime::from_unix_timestamp(first.timestamp).unwrap()
        );
    }
    if let Some(last) = quotes.last() {
        println!(
            "last bar: {}",
            OffsetDateTime::from_unix_timestamp(last.timestamp).unwrap()
        );
    }
}

#[cfg(feature = "blocking")]
fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();

    // 1. Shorthand: a range string instead of explicit dates
    let hist = provider.get_quote_range("AAPL", "1d", "5d").unwrap();
    println!("range 5d: {} bars", hist.quotes().unwrap().len());

    // 2. Explicit start/end period with a custom interval
    let start = datetime!(2024-01-02 00:00:00.00 UTC);
    let end = datetime!(2024-01-12 00:00:00.00 UTC);
    let hist = provider
        .get_quote_history_interval("AAPL", start, end, "1d")
        .unwrap();
    println!("interval 1d: {} bars", hist.quotes().unwrap().len());

    // 3. Same period with an intraday interval, also including
    //    pre/post market bars. Note that with interval="1d" the pre/post
    //    flag has no effect - it only applies to intraday intervals.
    //    Intraday data is only served for the last 730 days, so use a
    //    recent window.
    let end = time::OffsetDateTime::now_utc();
    let start = end - time::Duration::days(4);
    let hist = provider
        .get_quote_history_interval_prepost("AAPL", start, end, "1h", true)
        .unwrap();
    println!("interval 1h + prepost: {} bars", hist.quotes().unwrap().len());

    // 4. With prepost=true the bars cover the pre-market (from 04:00 ET)
    //    and after-hours (until 20:00 ET) sessions, extending beyond the
    //    regular 09:30-16:00 ET session
    let quotes = hist.quotes().unwrap();
    if let Some(first) = quotes.first() {
        println!(
            "first bar: {}",
            OffsetDateTime::from_unix_timestamp(first.timestamp).unwrap()
        );
    }
    if let Some(last) = quotes.last() {
        println!(
            "last bar: {}",
            OffsetDateTime::from_unix_timestamp(last.timestamp).unwrap()
        );
    }
}
