use std::time::{Duration, UNIX_EPOCH};

use time::{macros::datetime, OffsetDateTime};

use yahoo_finance_api as yahoo;

#[cfg(not(feature = "blocking"))]
#[tokio::main]
async fn main() {
    let conn = yahoo::YahooConnector::new().unwrap();
    let ticker = "OKE";
    let start = datetime!(2020-07-25 00:00:00.00 UTC);
    let end = datetime!(2020-11-01 00:00:00.00 UTC);
    let hist = conn.get_quote_history(ticker, start, end).await.unwrap();

    println!("{}", ticker);
    println!("QUOTES");
    for quote in hist.quotes().unwrap() {
        let time = OffsetDateTime::from(UNIX_EPOCH + Duration::from_secs(quote.timestamp));
        println!("{} | {:.2} | {:.2}", time, quote.open, quote.close);
    }

    // Display dividends paid during the requested period
    println!("DIVIDENDS");
    for dividend in hist.dividends().unwrap() {
        let date = OffsetDateTime::from(UNIX_EPOCH + Duration::from_secs(dividend.date));
        println!("{} | {:.3}", date, dividend.amount);
    }
}

#[cfg(feature = "blocking")]
fn main() {
    let conn = yahoo::YahooConnector::new().unwrap();
    let ticker = "OKE";
    let start = datetime!(2020-07-25 00:00:00.00 UTC);
    let end = datetime!(2020-11-01 00:00:00.00 UTC);
    let hist = conn.get_quote_history(ticker, start, end).unwrap();

    println!("{}", ticker);
    println!("QUOTES");
    for quote in hist.quotes().unwrap() {
        let time = OffsetDateTime::from(UNIX_EPOCH + Duration::from_secs(quote.timestamp));
        println!("{} | {:.2} | {:.2}", time, quote.open, quote.close);
    }

    // Display dividends paid during the requested period
    println!("DIVIDENDS");
    for dividend in hist.dividends().unwrap() {
        let date = OffsetDateTime::from(UNIX_EPOCH + Duration::from_secs(dividend.date));
        println!("{} | {:.3}", date, dividend.amount);
    }
}
