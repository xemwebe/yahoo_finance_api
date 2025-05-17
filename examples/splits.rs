#[cfg(not(feature = "blocking"))]
use std::time::Duration;

use time::macros::datetime;
use time::OffsetDateTime;

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
        let time = OffsetDateTime::from_unix_timestamp(quote.timestamp).unwrap();
        println!("{} | {:.2} | {:.2}", time, quote.open, quote.close);
    }

    // Get any splits that occured during the requested period
    println!("SPLITS");
    for split in hist.splits().unwrap() {
        let date = OffsetDateTime::from_unix_timestamp(split.date).unwrap();
        println!("{} | {} : {}", date, split.numerator, split.denominator);
    }
}

#[cfg(feature = "blocking")]
fn main() {
    let conn = yahoo::YahooConnector::new().unwrap();

    let ticker = "TSLA";
    let start = datetime!(2020-08-28 00:00:00.00 UTC);
    let end = datetime!(2020-09-02 00:00:00.00 UTC);
    let hist = conn.get_quote_history(ticker, start, end).unwrap();

    // Get the clean history
    println!("{}", ticker);
    println!("QUOTES");
    for quote in hist.quotes().unwrap() {
        let time = OffsetDateTime::from_unix_timestamp(quote.timestamp).unwrap();
        println!("{} | {:.2} | {:.2}", time, quote.open, quote.close);
    }

    // Get any splits that occured during the requested period
    println!("SPLITS");
    for split in hist.splits().unwrap() {
        let date = OffsetDateTime::from_unix_timestamp(split.date).unwrap();
        println!("{} | {} : {}", date, split.numerator, split.denominator);
    }
}
