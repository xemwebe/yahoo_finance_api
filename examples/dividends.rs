use std::time::{Duration, UNIX_EPOCH};

use chrono::{DateTime, Utc};

use yahoo_finance_api as yahoo;

#[cfg(not(feature = "blocking"))]
#[tokio::main]
async fn main() {
    let conn   = yahoo::YahooConnector::new();
    let ticker = "OKE";
    let start  = DateTime::parse_from_rfc3339("2020-07-25T00:00:00.00Z").unwrap().with_timezone(&Utc);
    let end    = DateTime::parse_from_rfc3339("2020-11-01T00:00:00.00Z").unwrap().with_timezone(&Utc);
    let hist  = conn.get_quote_history(ticker, start, end).await.unwrap();

    println!("{}", ticker);
    println!("QUOTES");
    for quote in hist.quotes().unwrap() {
        let time = DateTime::<Utc>::from(UNIX_EPOCH + Duration::from_secs(quote.timestamp));
        println!("{} | {:.2} | {:.2}", time, quote.open, quote.close);
    }

    // Display dividends paid during the requested period
    println!("DIVIDENDS");
    for dividend in hist.dividends().unwrap() {
        let date = DateTime::<Utc>::from(UNIX_EPOCH + Duration::from_secs(dividend.date));
        println!("{} | {:.3}", date, dividend.amount);
    }
}

#[cfg(feature = "blocking")]
fn main() {
    let conn   = yahoo::YahooConnector::new();
    let ticker = "OKE";
    let start  = DateTime::parse_from_rfc3339("2020-07-25T00:00:00.00Z").unwrap().with_timezone(&Utc);
    let end    = DateTime::parse_from_rfc3339("2020-11-01T00:00:00.00Z").unwrap().with_timezone(&Utc);
    let hist  = conn.get_quote_history(ticker, start, end).unwrap();

    println!("{}", ticker);
    println!("QUOTES");
    for quote in hist.quotes().unwrap() {
        let time = DateTime::<Utc>::from(UNIX_EPOCH + Duration::from_secs(quote.timestamp));
        println!("{} | {:.2} | {:.2}", time, quote.open, quote.close);
    }

    // Display dividends paid during the requested period
    println!("DIVIDENDS");
    for dividend in hist.dividends().unwrap() {
        let date = DateTime::<Utc>::from(UNIX_EPOCH + Duration::from_secs(dividend.date));
        println!("{} | {:.3}", date, dividend.amount);
    }
}
