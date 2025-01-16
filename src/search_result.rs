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

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct YOptionChain {
    pub option_chain: YOptionChainResult,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct YOptionChainResult {
    pub result: Vec<YOptionChainData>,
    pub error: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct YOptionChainData {
    pub underlying_symbol: String,
    pub expiration_dates: Vec<u64>,
    pub strikes: Vec<f64>,
    pub has_mini_options: bool,
    pub quote: YQuote,
    pub options: Vec<YOptionDetails>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct YQuote {
    pub language: String,
    pub region: String,
    pub quote_type: String,
    pub triggerable: bool,
    pub quote_source_name: String,
    pub currency: String,
    pub eps_current_year: f64,
    pub price_eps_current_year: f64,
    pub shares_outstanding: u64,
    pub book_value: f64,
    pub fifty_day_average: f64,
    pub fifty_day_average_change: f64,
    pub fifty_day_average_change_percent: f64,
    pub two_hundred_day_average: f64,
    pub two_hundred_day_average_change: f64,
    pub two_hundred_day_average_change_percent: f64,
    pub market_cap: u64,
    #[serde(rename = "forwardPE")]
    pub forward_pe: f64,
    pub price_to_book: f64,
    pub source_interval: u64,
    pub exchange_timezone_name: String,
    pub exchange_timezone_short_name: String,
    pub gmt_off_set_milliseconds: i64,
    pub esg_populated: bool,
    pub tradeable: bool,
    pub market_state: String,
    pub short_name: String,
    pub fifty_two_week_high_change: f64,
    pub fifty_two_week_high_change_percent: f64,
    pub fifty_two_week_low: f64,
    pub fifty_two_week_high: f64,
    pub dividend_date: u64,
    pub earnings_timestamp: u64,
    pub earnings_timestamp_start: u64,
    pub earnings_timestamp_end: u64,
    pub trailing_annual_dividend_rate: f64,
    #[serde(rename = "trailingPE")]
    pub trailing_pe: f64,
    pub trailing_annual_dividend_yield: f64,
    pub eps_trailing_twelve_months: f64,
    pub eps_forward: f64,
    pub price_hint: u64,
    pub post_market_change_percent: Option<f64>,
    pub post_market_time: Option<u64>,
    pub post_market_price: Option<f64>,
    pub post_market_change: Option<f64>,
    pub regular_market_change_percent: f64,
    pub regular_market_day_range: String,
    pub regular_market_previous_close: f64,
    pub bid: f64,
    pub ask: f64,
    pub bid_size: u64,
    pub ask_size: u64,
    pub message_board_id: String,
    pub full_exchange_name: String,
    pub long_name: String,
    pub financial_currency: String,
    pub average_daily_volume3_month: u64,
    pub average_daily_volume10_day: u64,
    pub fifty_two_week_low_change: f64,
    pub fifty_two_week_low_change_percent: f64,
    pub fifty_two_week_range: String,
    pub market: String,
    pub exchange_data_delayed_by: u64,
    pub regular_market_price: f64,
    pub regular_market_time: u64,
    pub regular_market_change: f64,
    pub regular_market_open: f64,
    pub regular_market_day_high: f64,
    pub regular_market_day_low: f64,
    pub regular_market_volume: u64,
    pub exchange: String,
    pub symbol: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct YOptionDetails {
    pub expiration_date: u64,
    pub has_mini_options: bool,
    pub calls: Vec<YOptionContract>,
    pub puts: Vec<YOptionContract>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct YOptionContract {
    pub contract_symbol: Option<String>,
    pub strike: Option<f64>,
    pub currency: Option<String>,
    pub last_price: Option<f64>,
    pub change: Option<f64>,
    pub percent_change: Option<f64>,
    pub volume: Option<u64>,
    pub open_interest: Option<u64>,
    pub bid: Option<f64>,
    pub ask: Option<f64>,
    pub contract_size: Option<String>,
    pub expiration: Option<u64>,
    pub last_trade_date: Option<u64>,
    pub implied_volatility: Option<f64>,
    pub in_the_money: Option<bool>,
}
