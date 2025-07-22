use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use time::OffsetDateTime;

use super::YahooError;

#[cfg(not(feature = "decimal"))]
pub mod decimal {
    pub type Decimal = f64;
    pub const ZERO: Decimal = 0.0;
}

#[cfg(feature = "decimal")]
pub mod decimal {
    pub type Decimal = rust_decimal::Decimal;
    pub const ZERO: Decimal = Decimal::ZERO;
}

pub use decimal::*;

#[derive(Deserialize, Debug)]
pub struct YResponse {
    pub chart: YChart,
}

impl YResponse {
    pub(crate) fn map_error_msg(self) -> Result<YResponse, YahooError> {
        if self.chart.result.is_none() {
            if let Some(y_error) = self.chart.error {
                return Err(YahooError::ApiError(y_error));
            }
        }
        Ok(self)
    }

    fn check_historical_consistency(&self) -> Result<&Vec<YQuoteBlock>, YahooError> {
        let Some(result) = &self.chart.result else {
            return Err(YahooError::NoResult);
        };

        for stock in result {
            let n = stock.timestamp.as_ref().map_or(0, |v| v.len());

            if n == 0 {
                return Err(YahooError::NoQuotes);
            }

            let quote = &stock.indicators.quote[0];

            if quote.open.is_none()
                || quote.high.is_none()
                || quote.low.is_none()
                || quote.volume.is_none()
                || quote.close.is_none()
            {
                return Err(YahooError::DataInconsistency);
            }

            let open_len = quote.open.as_ref().map_or(0, |v| v.len());
            let high_len = quote.high.as_ref().map_or(0, |v| v.len());
            let low_len = quote.low.as_ref().map_or(0, |v| v.len());
            let volume_len = quote.volume.as_ref().map_or(0, |v| v.len());
            let close_len = quote.close.as_ref().map_or(0, |v| v.len());

            if open_len != n || high_len != n || low_len != n || volume_len != n || close_len != n {
                return Err(YahooError::DataInconsistency);
            }
        }
        Ok(result)
    }

    pub fn from_json(json: serde_json::Value) -> Result<YResponse, YahooError> {
        Ok(serde_json::from_value(json)?)
    }

    /// Return the latest valid quote
    pub fn last_quote(&self) -> Result<Quote, YahooError> {
        let stock = &self.check_historical_consistency()?[0];

        let n = stock.timestamp.as_ref().map_or(0, |v| v.len());

        for i in (0..n).rev() {
            let quote = stock
                .indicators
                .get_ith_quote(stock.timestamp.as_ref().unwrap()[i], i);
            if quote.is_ok() {
                return quote;
            }
        }
        Err(YahooError::NoQuotes)
    }

    pub fn quotes(&self) -> Result<Vec<Quote>, YahooError> {
        let stock = &self.check_historical_consistency()?[0];

        let mut quotes = Vec::new();
        let n = stock.timestamp.as_ref().map_or(0, |v| v.len());
        for i in 0..n {
            let timestamp = stock.timestamp.as_ref().unwrap()[i];
            let quote = stock.indicators.get_ith_quote(timestamp, i);
            if let Ok(q) = quote {
                quotes.push(q);
            }
        }
        Ok(quotes)
    }

    pub fn metadata(&self) -> Result<YMetaData, YahooError> {
        let Some(result) = &self.chart.result else {
            return Err(YahooError::NoResult);
        };
        let stock = &result[0];
        Ok(stock.meta.to_owned())
    }

    /// This method retrieves information about the splits that might have
    /// occured during the considered time period
    pub fn splits(&self) -> Result<Vec<Split>, YahooError> {
        let Some(result) = &self.chart.result else {
            return Err(YahooError::NoResult);
        };
        let stock = &result[0];

        if let Some(events) = &stock.events {
            if let Some(splits) = &events.splits {
                let mut data = splits.values().cloned().collect::<Vec<Split>>();
                data.sort_unstable_by_key(|d| d.date);
                return Ok(data);
            }
        }
        Ok(vec![])
    }

    /// This method retrieves information about the dividends that have
    /// been recorded during the considered time period.
    ///
    /// Note: Date is the ex-dividend date)
    pub fn dividends(&self) -> Result<Vec<Dividend>, YahooError> {
        let Some(result) = &self.chart.result else {
            return Err(YahooError::NoResult);
        };
        let stock = &result[0];

        if let Some(events) = &stock.events {
            if let Some(dividends) = &events.dividends {
                let mut data = dividends.values().cloned().collect::<Vec<Dividend>>();
                data.sort_unstable_by_key(|d| d.date);
                return Ok(data);
            }
        }
        Ok(vec![])
    }

    /// This method retrieves information about the capital gains that might have
    /// occured during the considered time period (available only for Mutual Funds)
    pub fn capital_gains(&self) -> Result<Vec<CapitalGain>, YahooError> {
        let Some(result) = &self.chart.result else {
            return Err(YahooError::NoResult);
        };
        let stock = &result[0];

        if let Some(events) = &stock.events {
            if let Some(capital_gain) = &events.capital_gains {
                let mut data = capital_gain.values().cloned().collect::<Vec<CapitalGain>>();
                data.sort_unstable_by_key(|d| d.date);
                return Ok(data);
            }
        }
        Ok(vec![])
    }
}

/// Struct for single quote
#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct Quote {
    pub timestamp: i64,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub volume: u64,
    pub close: Decimal,
    pub adjclose: Decimal,
}

#[derive(Deserialize, Debug)]
pub struct YChart {
    pub result: Option<Vec<YQuoteBlock>>,
    pub error: Option<YErrorMessage>,
}

#[derive(Deserialize, Debug)]
pub struct YQuoteBlock {
    pub meta: YMetaData,
    pub timestamp: Option<Vec<i64>>,
    pub events: Option<EventsBlock>,
    pub indicators: QuoteBlock,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct YMetaData {
    pub currency: Option<String>,
    pub symbol: String,
    pub long_name: Option<String>,
    pub short_name: Option<String>,
    pub instrument_type: String,
    pub exchange_name: String,
    pub full_exchange_name: String,
    #[serde(default)]
    pub first_trade_date: Option<i32>,
    pub regular_market_time: Option<u32>,
    pub gmtoffset: i32,
    pub timezone: String,
    pub exchange_timezone_name: String,
    pub regular_market_price: Option<Decimal>,
    pub chart_previous_close: Option<Decimal>,
    pub previous_close: Option<Decimal>,
    pub has_pre_post_market_data: bool,
    pub fifty_two_week_high: Option<Decimal>,
    pub fifty_two_week_low: Option<Decimal>,
    pub regular_market_day_high: Option<Decimal>,
    pub regular_market_day_low: Option<Decimal>,
    pub regular_market_volume: Option<Decimal>,
    #[serde(default)]
    pub scale: Option<i32>,
    pub price_hint: i32,
    pub current_trading_period: CurrentTradingPeriod,
    #[serde(default)]
    pub trading_periods: TradingPeriods,
    pub data_granularity: String,
    pub range: String,
    pub valid_ranges: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct TradingPeriods {
    pub pre: Option<Vec<Vec<PeriodInfo>>>,
    pub regular: Option<Vec<Vec<PeriodInfo>>>,
    pub post: Option<Vec<Vec<PeriodInfo>>>,
}

impl<'de> Deserialize<'de> for TradingPeriods {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Regular,
            Pre,
            Post,
        }

        struct TradingPeriodsVisitor;

        impl<'de> Visitor<'de> for TradingPeriodsVisitor {
            type Value = TradingPeriods;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct (or array) TradingPeriods")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<TradingPeriods, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let mut regular: Vec<PeriodInfo> = Vec::new();

                while let Ok(Some(mut e)) = seq.next_element::<Vec<PeriodInfo>>() {
                    regular.append(&mut e);
                }

                Ok(TradingPeriods {
                    pre: None,
                    regular: Some(vec![regular]),
                    post: None,
                })
            }

            fn visit_map<V>(self, mut map: V) -> Result<TradingPeriods, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut pre = None;
                let mut post = None;
                let mut regular = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Pre => {
                            if pre.is_some() {
                                return Err(de::Error::duplicate_field("pre"));
                            }
                            pre = Some(map.next_value()?);
                        }
                        Field::Post => {
                            if post.is_some() {
                                return Err(de::Error::duplicate_field("post"));
                            }
                            post = Some(map.next_value()?);
                        }
                        Field::Regular => {
                            if regular.is_some() {
                                return Err(de::Error::duplicate_field("regular"));
                            }
                            regular = Some(map.next_value()?);
                        }
                    }
                }
                Ok(TradingPeriods { pre, post, regular })
            }
        }

        deserializer.deserialize_any(TradingPeriodsVisitor)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct CurrentTradingPeriod {
    pub pre: PeriodInfo,
    pub regular: PeriodInfo,
    pub post: PeriodInfo,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct PeriodInfo {
    pub timezone: String,
    pub start: u32,
    pub end: u32,
    pub gmtoffset: i32,
}

#[derive(Deserialize, Debug)]
pub struct QuoteBlock {
    quote: Vec<QuoteList>,
    #[serde(default)]
    adjclose: Option<Vec<AdjClose>>,
}

impl QuoteBlock {
    fn get_ith_quote(&self, timestamp: i64, i: usize) -> Result<Quote, YahooError> {
        let adjclose = match &self.adjclose {
            Some(vec_of_adjclose) => match vec_of_adjclose[0].adjclose {
                Some(ref adjclose) => adjclose[i],
                None => None,
            },
            None => None,
        };

        let quote = &self.quote[0];
        // reject if close is not set

        let open = match quote.open {
            Some(ref open) => open[i],
            None => None,
        };

        let high = match quote.high {
            Some(ref high) => high[i],
            None => None,
        };

        let low = match quote.low {
            Some(ref low) => low[i],
            None => None,
        };

        let volume = match quote.volume {
            Some(ref volume) => volume[i],
            None => None,
        };

        let close = match quote.close {
            Some(ref close) => close[i],
            None => None,
        };

        if close.is_none() {
            return Err(YahooError::NoQuotes);
        }

        Ok(Quote {
            timestamp,
            open: open.unwrap_or(ZERO),
            high: high.unwrap_or(ZERO),
            low: low.unwrap_or(ZERO),
            volume: volume.unwrap_or(0),
            close: close.unwrap(),
            adjclose: adjclose.unwrap_or(ZERO),
        })
    }
}

#[derive(Deserialize, Debug)]
pub struct AdjClose {
    adjclose: Option<Vec<Option<Decimal>>>,
}

#[derive(Deserialize, Debug)]
pub struct QuoteList {
    pub volume: Option<Vec<Option<u64>>>,
    pub high: Option<Vec<Option<Decimal>>>,
    pub close: Option<Vec<Option<Decimal>>>,
    pub low: Option<Vec<Option<Decimal>>>,
    pub open: Option<Vec<Option<Decimal>>>,
}

#[derive(Deserialize, Debug)]
pub struct EventsBlock {
    pub splits: Option<HashMap<i64, Split>>,
    pub dividends: Option<HashMap<i64, Dividend>>,
    #[serde(rename = "capitalGains")]
    pub capital_gains: Option<HashMap<i64, CapitalGain>>,
}

/// This structure simply models a split that has occured.
#[derive(Deserialize, Debug, Clone)]
pub struct Split {
    /// This is the date (timestamp) when the split occured
    pub date: i64,
    /// Numerator of the split. For instance a 1:5 split means you get 5 share
    /// wherever you had one before the split. (Here the numerator is 1 and
    /// denom is 5). A reverse split is considered as nothing but a regular
    /// split with a numerator > denom.
    pub numerator: Decimal,
    /// Denominator of the split. For instance a 1:5 split means you get 5 share
    /// wherever you had one before the split. (Here the numerator is 1 and
    /// denom is 5). A reverse split is considered as nothing but a regular
    /// split with a numerator > denom.
    pub denominator: Decimal,
    /// A textual representation of the split.
    #[serde(rename = "splitRatio")]
    pub split_ratio: String,
}

/// This structure simply models a dividend which has been recorded.
#[derive(Deserialize, Debug, Clone)]
pub struct Dividend {
    /// This is the price of the dividend
    pub amount: Decimal,
    /// This is the ex-dividend date as UNIX timestamp
    pub date: i64,
}

/// This structure simply models a capital gain which has been recorded.
#[derive(Deserialize, Debug, Clone)]
pub struct CapitalGain {
    /// This is the amount of capital gain distributed by the fund
    pub amount: f64,
    /// This is the recorded date of the capital gain
    pub date: i64,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct YQuoteSummary {
    #[serde(rename = "quoteSummary")]
    pub quote_summary: Option<ExtendedQuoteSummary>,
    pub finance: Option<YFinance>,
}

#[derive(Deserialize, Debug)]
pub struct YFinance {
    pub result: Option<serde_json::Value>,
    pub error: Option<YErrorMessage>,
}

#[derive(Deserialize, Debug)]
pub struct YErrorMessage {
    pub code: Option<String>,
    pub description: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct ExtendedQuoteSummary {
    pub result: Option<Vec<YSummaryData>>,
    pub error: Option<YErrorMessage>,
}

impl YQuoteSummary {
    pub fn from_json(json: serde_json::Value) -> Result<YQuoteSummary, YahooError> {
        Ok(serde_json::from_value(json)?)
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct YSummaryData {
    pub asset_profile: Option<AssetProfile>,
    pub summary_detail: Option<SummaryDetail>,
    pub default_key_statistics: Option<DefaultKeyStatistics>,
    pub quote_type: Option<QuoteType>,
    pub financial_data: Option<FinancialData>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AssetProfile {
    pub address1: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
    pub country: Option<String>,
    pub phone: Option<String>,
    pub website: Option<String>,
    pub industry: Option<String>,
    pub sector: Option<String>,
    pub long_business_summary: Option<String>,
    pub full_time_employees: Option<u32>,
    pub company_officers: Vec<CompanyOfficer>,
    pub audit_risk: Option<u16>,
    pub board_risk: Option<u16>,
    pub compensation_risk: Option<u16>,
    pub share_holder_rights_risk: Option<u16>,
    pub overall_risk: Option<u16>,
    pub governance_epoch_date: Option<u32>,
    pub compensation_as_of_epoch_date: Option<u32>,
    pub ir_website: Option<String>,
    pub max_age: Option<u32>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CompanyOfficer {
    pub name: String,
    pub age: Option<u32>,
    pub title: String,
    pub year_born: Option<u32>,
    pub fiscal_year: Option<u32>,
    pub total_pay: Option<ValueWrapper>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ValueWrapper {
    pub raw: Option<i64>,
    pub fmt: Option<String>,
    pub long_fmt: Option<String>,
}

fn deserialize_f64_special<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: serde_json::Value = Deserialize::deserialize(deserializer)?;
    match s {
        serde_json::Value::String(ref v) if v.eq_ignore_ascii_case("infinity") => {
            Ok(Some(f64::INFINITY))
        }
        serde_json::Value::String(ref v) if v.eq_ignore_ascii_case("-infinity") => {
            Ok(Some(f64::NEG_INFINITY))
        }
        serde_json::Value::String(ref v) if v.eq_ignore_ascii_case("nan") => Ok(Some(f64::NAN)),
        serde_json::Value::Number(n) => n
            .as_f64()
            .ok_or_else(|| serde::de::Error::custom("Invalid number"))
            .map(Some),
        serde_json::Value::Null => Ok(None),
        _ => Err(serde::de::Error::custom(format!(
            "Invalid type for f64: {:?}",
            s
        ))),
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SummaryDetail {
    pub max_age: Option<i64>,
    pub price_hint: Option<i64>,
    pub previous_close: Option<f64>,
    pub open: Option<f64>,
    pub day_low: Option<f64>,
    pub day_high: Option<f64>,
    pub regular_market_previous_close: Option<f64>,
    pub regular_market_open: Option<f64>,
    pub regular_market_day_low: Option<f64>,
    pub regular_market_day_high: Option<f64>,
    pub dividend_rate: Option<f64>,
    pub dividend_yield: Option<f64>,
    pub ex_dividend_date: Option<i64>,
    pub payout_ratio: Option<f64>,
    pub five_year_avg_dividend_yield: Option<f64>,
    pub beta: Option<f64>,
    /// The trailing_pe field may contain the string "Infinity" instead of f64, in which case we return f64::MAX
    #[serde(
        default,
        deserialize_with = "deserialize_f64_special",
        rename = "trailingPE"
    )]
    pub trailing_pe: Option<f64>,
    #[serde(
        default,
        rename = "forwardPE",
        deserialize_with = "deserialize_f64_special"
    )]
    pub forward_pe: Option<f64>,
    pub volume: Option<u64>,
    pub regular_market_volume: Option<u64>,
    pub average_volume: Option<u64>,
    #[serde(rename = "averageVolume10days")]
    pub average_volume_10days: Option<u64>,
    #[serde(rename = "averageDailyVolume10Day")]
    pub average_daily_volume_10day: Option<u64>,
    pub bid: Option<f64>,
    pub ask: Option<f64>,
    pub bid_size: Option<i64>,
    pub ask_size: Option<i64>,
    pub market_cap: Option<u64>,
    pub fifty_two_week_low: Option<f64>,
    pub fifty_two_week_high: Option<f64>,
    #[serde(
        default,
        rename = "priceToSalesTrailing12Months",
        deserialize_with = "deserialize_f64_special"
    )]
    pub price_to_sales_trailing12months: Option<f64>,
    pub fifty_day_average: Option<f64>,
    pub two_hundred_day_average: Option<f64>,
    pub trailing_annual_dividend_rate: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_f64_special")]
    pub trailing_annual_dividend_yield: Option<f64>,
    pub currency: Option<String>,
    pub from_currency: Option<String>,
    pub to_currency: Option<String>,
    pub last_market: Option<String>,
    pub coin_market_cap_link: Option<String>,
    pub algorithm: Option<String>,
    pub tradeable: Option<bool>,
    pub expire_date: Option<u32>,
    pub strike_price: Option<u32>,
    pub open_interest: Option<Decimal>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DefaultKeyStatistics {
    pub max_age: Option<i64>,
    pub price_hint: Option<u64>,
    pub enterprise_value: Option<i64>,
    #[serde(
        default,
        rename = "forwardPE",
        deserialize_with = "deserialize_f64_special"
    )]
    pub forward_pe: Option<f64>,
    pub profit_margins: Option<f64>,
    pub float_shares: Option<u64>,
    pub shares_outstanding: Option<u64>,
    pub shares_short: Option<u64>,
    pub shares_short_prior_month: Option<u64>,
    pub shares_short_previous_month_date: Option<u64>,
    pub date_short_interest: Option<i64>,
    pub shares_percent_shares_out: Option<f64>,
    pub held_percent_insiders: Option<f64>,
    pub held_percent_institutions: Option<f64>,
    pub short_ratio: Option<f64>,
    pub short_percent_of_float: Option<f64>,
    pub beta: Option<f64>,
    pub implied_shares_outstanding: Option<u64>,
    pub category: Option<String>,
    pub book_value: Option<f64>,
    pub price_to_book: Option<f64>,
    pub fund_family: Option<String>,
    pub fund_inception_date: Option<u32>,
    pub legal_type: Option<String>,
    pub last_fiscal_year_end: Option<i64>,
    pub next_fiscal_year_end: Option<i64>,
    pub most_recent_quarter: Option<i64>,
    pub earnings_quarterly_growth: Option<f64>,
    pub net_income_to_common: Option<i64>,
    pub trailing_eps: Option<f64>,
    pub forward_eps: Option<f64>,
    pub last_split_factor: Option<String>,
    pub last_split_date: Option<i64>,
    pub enterprise_to_revenue: Option<f64>,
    pub enterprise_to_ebitda: Option<f64>,
    #[serde(rename = "52WeekChange")]
    pub fifty_two_week_change: Option<f64>,
    #[serde(rename = "SandP52WeekChange")]
    pub sand_p_fifty_two_week_change: Option<f64>,
    pub last_dividend_value: Option<f64>,
    pub last_dividend_date: Option<i64>,
    pub latest_share_class: Option<String>,
    pub lead_investor: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QuoteType {
    pub exchange: Option<String>,
    pub quote_type: Option<String>,
    pub symbol: Option<String>,
    pub underlying_symbol: Option<String>,
    pub short_name: Option<String>,
    pub long_name: Option<String>,
    pub first_trade_date_epoch_utc: Option<i64>,
    #[serde(rename = "timeZoneFullName")]
    pub timezone_full_name: Option<String>,
    #[serde(rename = "timeZoneShortName")]
    pub timezone_short_name: Option<String>,
    pub uuid: Option<String>,
    pub message_board_id: Option<String>,
    pub gmt_off_set_milliseconds: Option<i64>,
    pub max_age: Option<i64>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FinancialData {
    pub max_age: Option<i64>,
    pub current_price: Option<f64>,
    pub target_high_price: Option<f64>,
    pub target_low_price: Option<f64>,
    pub target_mean_price: Option<f64>,
    pub target_median_price: Option<f64>,
    pub recommendation_mean: Option<f64>,
    pub recommendation_key: Option<String>,
    pub number_of_analyst_opinions: Option<u64>,
    pub total_cash: Option<u64>,
    pub total_cash_per_share: Option<f64>,
    pub ebitda: Option<i64>,
    pub total_debt: Option<u64>,
    pub quick_ratio: Option<f64>,
    pub current_ratio: Option<f64>,
    pub total_revenue: Option<i64>,
    pub debt_to_equity: Option<f64>,
    pub revenue_per_share: Option<f64>,
    pub return_on_assets: Option<f64>,
    pub return_on_equity: Option<f64>,
    pub gross_profits: Option<i64>,
    pub free_cashflow: Option<i64>,
    pub operating_cashflow: Option<i64>,
    pub earnings_growth: Option<f64>,
    pub revenue_growth: Option<f64>,
    pub gross_margins: Option<f64>,
    pub ebitda_margins: Option<f64>,
    pub operating_margins: Option<f64>,
    pub profit_margins: Option<f64>,
    pub financial_currency: Option<String>,
}

// Структуры для earnings dates response
#[derive(Deserialize, Debug, Clone)]
pub struct YEarningsResponse {
    pub finance: YEarningsFinance,
}

#[derive(Deserialize, Debug, Clone)]
pub struct YEarningsFinance {
    pub result: Vec<YEarningsResult>,
    pub error: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct YEarningsResult {
    pub documents: Vec<YEarningsDocument>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct YEarningsDocument {
    pub columns: Vec<YEarningsColumn>,
    pub rows: Vec<Vec<serde_json::Value>>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct YEarningsColumn {
    pub label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FinancialEvent {
    pub earnings_date: OffsetDateTime,
    pub event_type: String,
    pub eps_estimate: Option<f64>,
    pub reported_eps: Option<f64>,
    pub surprise_percent: Option<f64>,
    pub timezone: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_period_info() {
        let period_info_json = r#"
        {
            "timezone": "EST",
            "start": 1705501800,
            "end": 1705525200,
            "gmtoffset": -18000
        }
        "#;
        let period_info_expected = PeriodInfo {
            timezone: "EST".to_string(),
            start: 1705501800,
            end: 1705525200,
            gmtoffset: -18000,
        };
        let period_info_deserialized: PeriodInfo = serde_json::from_str(period_info_json).unwrap();
        assert_eq!(&period_info_deserialized, &period_info_expected);
    }

    #[test]
    fn test_deserialize_trading_periods_simple() {
        let trading_periods_json = r#"
        [
            [
                {
                    "timezone": "EST",
                    "start": 1705501800,
                    "end": 1705525200,
                    "gmtoffset": -18000
                }

            ]
        ]
        "#;
        let trading_periods_expected = TradingPeriods {
            pre: None,
            regular: Some(vec![vec![PeriodInfo {
                timezone: "EST".to_string(),
                start: 1705501800,
                end: 1705525200,
                gmtoffset: -18000,
            }]]),
            post: None,
        };
        let trading_periods_deserialized: TradingPeriods =
            serde_json::from_str(trading_periods_json).unwrap();
        assert_eq!(&trading_periods_expected, &trading_periods_deserialized);
    }

    #[test]
    fn test_deserialize_trading_periods_complex_regular_only() {
        let trading_periods_json = r#"
        {
            "regular": [
              [
                {
                  "timezone": "EST",
                  "start": 1705501800,
                  "end": 1705525200,
                  "gmtoffset": -18000
                }
              ]
            ]
        }
       "#;
        let trading_periods_expected = TradingPeriods {
            pre: None,
            regular: Some(vec![vec![PeriodInfo {
                timezone: "EST".to_string(),
                start: 1705501800,
                end: 1705525200,
                gmtoffset: -18000,
            }]]),
            post: None,
        };
        let trading_periods_deserialized: TradingPeriods =
            serde_json::from_str(trading_periods_json).unwrap();
        assert_eq!(&trading_periods_expected, &trading_periods_deserialized);
    }

    #[test]
    fn test_deserialize_trading_periods_complex() {
        let trading_periods_json = r#"
        {
            "pre": [
              [
                {
                  "timezone": "EST",
                  "start": 1705482000,
                  "end": 1705501800,
                  "gmtoffset": -18000
                }
              ]
            ],
            "post": [
              [
                {
                  "timezone": "EST",
                  "start": 1705525200,
                  "end": 1705539600,
                  "gmtoffset": -18000
                }
              ]
            ],
            "regular": [
              [
                {
                  "timezone": "EST",
                  "start": 1705501800,
                  "end": 1705525200,
                  "gmtoffset": -18000
                }
              ]
            ]
        }
       "#;
        let trading_periods_expected = TradingPeriods {
            pre: Some(vec![vec![PeriodInfo {
                timezone: "EST".to_string(),
                start: 1705482000,
                end: 1705501800,
                gmtoffset: -18000,
            }]]),
            regular: Some(vec![vec![PeriodInfo {
                timezone: "EST".to_string(),
                start: 1705501800,
                end: 1705525200,
                gmtoffset: -18000,
            }]]),
            post: Some(vec![vec![PeriodInfo {
                timezone: "EST".to_string(),
                start: 1705525200,
                end: 1705539600,
                gmtoffset: -18000,
            }]]),
        };
        let trading_periods_deserialized: TradingPeriods =
            serde_json::from_str(trading_periods_json).unwrap();
        assert_eq!(&trading_periods_expected, &trading_periods_deserialized);
    }

    #[test]
    fn test_deserialize_f64_special() {
        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        struct MyStruct {
            #[serde(default, deserialize_with = "deserialize_f64_special")]
            bad: Option<f64>,
            good: Option<f64>,
        }

        let json_data = r#"{ "bad": "Infinity", "good": 999.999 }"#;
        let _: MyStruct = serde_json::from_str(json_data).unwrap();

        let json_data = r#"{ "bad": 123.45 }"#;
        let _: MyStruct = serde_json::from_str(json_data).unwrap();

        let json_data = r#"{ "bad": null }"#;
        let _: MyStruct = serde_json::from_str(json_data).unwrap();

        let json_data = r#"{ "bad": "NaN" }"#;
        let _: MyStruct = serde_json::from_str(json_data).unwrap();

        let json_data = r#"{ "bad": "-Infinity" }"#;
        let _: MyStruct = serde_json::from_str(json_data).unwrap();

        let json_data = r#"{ }"#;
        let _: MyStruct = serde_json::from_str(json_data).unwrap();
    }
}
