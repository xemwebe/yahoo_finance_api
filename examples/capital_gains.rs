// Capital gain distributions, reported for mutual funds only.
use time::macros::datetime;
use time::OffsetDateTime;

use yahoo_finance_api as yahoo;

#[cfg(not(feature = "blocking"))]
#[tokio::main]
async fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();

    // VFINX is a Vanguard S&P 500 fund; capital gains are distributed in
    // December. Note that Yahoo may omit the capitalGains event entirely,
    // in which case the result is simply empty.
    let start = datetime!(2023-11-01 00:00:00.00 UTC);
    let end = datetime!(2024-01-15 00:00:00.00 UTC);
    let hist = provider
        .get_quote_history("VFINX", start, end)
        .await
        .unwrap();

    let gains = hist.capital_gains().unwrap();
    if gains.is_empty() {
        println!("no capital gains in the requested period");
    } else {
        for gain in gains {
            let date = OffsetDateTime::from_unix_timestamp(gain.date).unwrap();
            println!("{} | {}", date, gain.amount);
        }
    }
}

#[cfg(feature = "blocking")]
fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();

    // VFINX is a Vanguard S&P 500 fund; capital gains are distributed in
    // December. Note that Yahoo may omit the capitalGains event entirely,
    // in which case the result is simply empty.
    let start = datetime!(2023-11-01 00:00:00.00 UTC);
    let end = datetime!(2024-01-15 00:00:00.00 UTC);
    let hist = provider.get_quote_history("VFINX", start, end).unwrap();

    let gains = hist.capital_gains().unwrap();
    if gains.is_empty() {
        println!("no capital gains in the requested period");
    } else {
        for gain in gains {
            let date = OffsetDateTime::from_unix_timestamp(gain.date).unwrap();
            println!("{} | {}", date, gain.amount);
        }
    }
}
