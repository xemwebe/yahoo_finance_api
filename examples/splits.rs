use std::time::{Duration, UNIX_EPOCH};

use time::{macros::datetime, OffsetDateTime};

use yahoo_finance_api as yahoo;

#[cfg(not(feature = "blocking"))]
#[tokio::main]
async fn main() {
    let conn = yahoo::YahooConnector::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();

    let ticker = "TSLA";
    let start = datetime!(2020-08-28 00:00:00.00 UTC);
    let end = datetime!(2020-09-02 00:00:00.00 UTC);
    let hist = conn.get_quote_history(ticker, start, end).await.unwrap();

    // Get the clean history
    println!("{}", ticker);
    println!("QUOTES");
    for quote in hist.quotes().unwrap() {
        let time = OffsetDateTime::from(UNIX_EPOCH + Duration::from_secs(quote.timestamp));
        println!("{} | {:.2} | {:.2}", time, quote.open, quote.close);
    }

    // Get any splits that occured during the requested period
    println!("SPLITS");
    for split in hist.splits().unwrap() {
        let date = OffsetDateTime::from(UNIX_EPOCH + Duration::from_secs(split.date));
        println!("{} | {} : {}", date, split.numerator, split.denominator);
    }
}

#[cfg(feature = "blocking")]
fn main() {
    let conn = yahoo::YahooConnector::new();
    let ticker = "TSLA";
    let start = datetime!(2020-08-28 00:00:00.00 UTC);
    let end = datetime!(2020-09-02 00:00:00.00 UTC);
    let hist = conn.get_quote_history(ticker, start, end).unwrap();

    // Get the clean history
    println!("{}", ticker);
    println!("QUOTES");
    for quote in hist.quotes().unwrap() {
        let time = OffsetDateTime::from(UNIX_EPOCH + Duration::from_secs(quote.timestamp));
        println!("{} | {:.2} | {:.2}", time, quote.open, quote.close);
    }

    // Get any splits that occured during the requested period
    println!("SPLITS");
    for split in hist.splits().unwrap() {
        let date = OffsetDateTime::from(UNIX_EPOCH + Duration::from_secs(split.date));
        println!("{} | {} : {}", date, split.numerator, split.denominator);
    }
}
