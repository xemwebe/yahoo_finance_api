use serde::Deserialize;

use super::YahooError;

#[derive(Deserialize, Debug)]
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

#[derive(Deserialize, Debug)]
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

impl YSearchResult {
    pub fn from_json(json: serde_json::Value) -> Result<YSearchResult, YahooError> {
        match serde_json::from_value(json) {
            Ok(v) => Ok(v),
            Err(e) => Err(YahooError::DeserializeFailed(e.to_string())),
        }
    }
}
