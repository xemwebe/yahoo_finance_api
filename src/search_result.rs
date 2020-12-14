use serde::Deserialize;

use super::YahooError;

#[derive(Deserialize, Debug)]
pub struct YSearchResultOpt {
    pub count: u32,
    pub quotes: Vec<YQuoteItemOpt>,
    pub news: Vec<YNewsItem>,
}

#[derive(Deserialize, Debug)]
pub struct YQuoteItemOpt {
    pub exchange: String,
    #[serde(rename = "shortname")]
    pub short_name: Option<String>,
    #[serde(rename = "quoteType")]
    pub quote_type: String,
    pub symbol: String,
    pub index: String,
    pub score: f64,
    #[serde(rename = "typeDisp")]
    pub type_display: String,
    #[serde(rename = "longname")]
    pub long_name: Option<String>,
    #[serde(rename = "isYahooFinance")]
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
        match serde_json::from_value(json) {
            Ok(v) => Ok(v),
            Err(e) => Err(YahooError::DeserializeFailed(e.to_string())),
        }
    }
}

#[derive(Debug)]
pub struct YSearchResult {
    pub count: u32,
    pub quotes: Vec<YQuoteItem>,
    pub news: Vec<YNewsItem>,
}


#[derive(Deserialize, Debug)]
pub struct YQuoteItem {
    pub exchange: String,
    #[serde(rename = "shortname")]
    pub short_name: String,
    #[serde(rename = "quoteType")]
    pub quote_type: String,
    pub symbol: String,
    pub index: String,
    pub score: f64,
    #[serde(rename = "typeDisp")]
    pub type_display: String,
    #[serde(rename = "longname")]
    pub long_name: String,
    #[serde(rename = "isYahooFinance")]
    pub is_yahoo_finance: bool,
}

impl YQuoteItem {
    fn from_yquote_item_opt(quote: &YQuoteItemOpt) -> YQuoteItem {
        YQuoteItem{
            exchange: quote.exchange.clone(),
            short_name: quote.short_name.as_ref().unwrap_or(&("".to_string())).clone(),
            quote_type: quote.quote_type.clone(),
            symbol: quote.symbol.clone(),
            index: quote.index.clone(),
            score: quote.score,
            type_display: quote.type_display.clone(),
            long_name: quote.long_name.as_ref().unwrap_or(&("".to_string())).clone(),
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
        YSearchResult{
            count: search_result_opt.count,
            quotes: remove_opt(&search_result_opt.quotes),
            news: search_result_opt.news.clone(),
        }
    }
}
