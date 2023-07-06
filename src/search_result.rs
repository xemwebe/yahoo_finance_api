use select::document::Document;
use select::predicate::{Class, Name};
use serde::Deserialize;

use super::YahooError;

#[derive(Deserialize, Debug)]
pub struct YSearchResultOpt {
    pub count: u32,
    pub quotes: Vec<YQuoteItemOpt>,
    pub news: Vec<YNewsItem>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct YQuoteItemOpt {
    pub exchange: String,
    #[serde(rename = "shortname")]
    pub short_name: Option<String>,
    pub quote_type: String,
    pub symbol: String,
    pub index: String,
    pub score: f64,
    #[serde(rename = "typeDisp")]
    pub type_display: String,
    #[serde(rename = "longname")]
    pub long_name: Option<String>,
    pub is_yahoo_finance: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct YNewsItem {
    pub uuid: String,
    pub title: String,
    pub publisher: String,
    pub link: String,
    #[serde(rename = "providerPublishTime")]
    pub provider_publish_time: u64,
    #[serde(rename = "type")]
    pub newstype: String,
}

impl YSearchResultOpt {
    pub fn from_json(json: serde_json::Value) -> Result<YSearchResultOpt, YahooError> {
        Ok(serde_json::from_value(json)?)
    }
}

#[derive(Debug)]
pub struct YSearchResult {
    pub count: u32,
    pub quotes: Vec<YQuoteItem>,
    pub news: Vec<YNewsItem>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct YQuoteItem {
    pub exchange: String,
    #[serde(rename = "shortname")]
    pub short_name: String,
    pub quote_type: String,
    pub symbol: String,
    pub index: String,
    pub score: f64,
    #[serde(rename = "typeDisp")]
    pub type_display: String,
    #[serde(rename = "longname")]
    pub long_name: String,
    pub is_yahoo_finance: bool,
}

impl YQuoteItem {
    fn from_yquote_item_opt(quote: &YQuoteItemOpt) -> YQuoteItem {
        YQuoteItem {
            exchange: quote.exchange.clone(),
            short_name: quote
                .short_name
                .as_ref()
                .unwrap_or(&("".to_string()))
                .clone(),
            quote_type: quote.quote_type.clone(),
            symbol: quote.symbol.clone(),
            index: quote.index.clone(),
            score: quote.score,
            type_display: quote.type_display.clone(),
            long_name: quote
                .long_name
                .as_ref()
                .unwrap_or(&("".to_string()))
                .clone(),
            is_yahoo_finance: quote.is_yahoo_finance,
        }
    }
}

fn remove_opt(quotes: &[YQuoteItemOpt]) -> Vec<YQuoteItem> {
    let mut new_quotes = Vec::new();
    for quote in quotes {
        new_quotes.push(YQuoteItem::from_yquote_item_opt(quote));
    }
    new_quotes
}

impl YSearchResult {
    pub fn from_opt(search_result_opt: &YSearchResultOpt) -> YSearchResult {
        YSearchResult {
            count: search_result_opt.count,
            quotes: remove_opt(&search_result_opt.quotes),
            news: search_result_opt.news.clone(),
        }
    }
}

#[derive(Debug)]
pub struct YOptionResult {
    pub name: String,
    pub strike: f64,
    pub last_trade_date: String,
    pub last_price: f64,
    pub bid: f64,
    pub ask: f64,
    pub change: f64,
    pub change_pct: f64,
    pub volume: i32,
    pub open_interest: i32,
    pub impl_volatility: f64,
}

#[derive(Debug)]
pub struct YOptionResults {
    pub options: Vec<YOptionResult>,
}

impl YOptionResults {
    pub fn scrape(http_res: &str) -> Self {
        let document = Document::from(http_res);

        if let Some(table) = document.find(Class("list-options")).next() {
            let rows = table.find(Name("tr"));
            let options = rows
                .skip(1)
                .map(|row| {
                    let columns = row.find(Name("td"));
                    let cols: Vec<String> = columns
                        .take(11)
                        .map(|s| s.text().trim().to_owned())
                        .collect();
                    cols
                })
                .map(|sv| {
                    YOptionResult {
                        name: sv[0].clone(),
                        last_trade_date: sv[1].clone(),
                        strike: sv[2].replace(',', "").parse::<f64>().unwrap_or(0.0),
                        last_price: sv[3].replace(',', "").parse::<f64>().unwrap_or(0.0),
                        bid: sv[4].replace(',', "").parse::<f64>().unwrap_or(0.0),
                        ask: sv[5].replace(',', "").parse::<f64>().unwrap_or(0.0),
                        change: sv[6].replace(',', "").parse::<f64>().unwrap_or(0.0),
                        change_pct: sv[7]
                            .replace(',', "")
                            .trim_end_matches('%')
                            .parse::<f64>()
                            .unwrap_or(0.0),
                        volume: sv[8].replace(',', "").parse::<i32>().unwrap_or(0),
                        open_interest: sv[9].replace(',', "").parse::<i32>().unwrap_or(0),
                        impl_volatility: sv[10]
                            .replace(',', "")
                            .trim_end_matches('%')
                            .parse::<f64>()
                            .unwrap_or(0.0),
                    }
                })
                .collect();
            Self { options }
        } else {
            Self {
                options: Vec::new(),
            }
        }
    }
}
