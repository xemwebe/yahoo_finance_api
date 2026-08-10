use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
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

#[derive(Deserialize, Debug, Serialize)]
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

#[derive(Deserialize, Debug, Serialize)]
pub struct YChart {
    pub result: Option<Vec<YQuoteBlock>>,
    pub error: Option<YErrorMessage>,
}

#[derive(Deserialize, Debug, Default, Serialize)]
pub struct YQuoteBlock {
    pub meta: YMetaData,
    pub timestamp: Option<Vec<i64>>,
    pub events: Option<EventsBlock>,
    pub indicators: QuoteBlock,
}

#[derive(Deserialize, Debug, Default, Clone, Serialize)]
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

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize)]
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

#[derive(Deserialize, Debug, Default, Clone, Serialize)]
pub struct CurrentTradingPeriod {
    pub pre: PeriodInfo,
    pub regular: PeriodInfo,
    pub post: PeriodInfo,
}

#[derive(Deserialize, Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct PeriodInfo {
    pub timezone: String,
    pub start: u32,
    pub end: u32,
    pub gmtoffset: i32,
}

#[derive(Deserialize, Debug, Default, Serialize)]
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

#[derive(Deserialize, Debug, Serialize)]
pub struct AdjClose {
    adjclose: Option<Vec<Option<Decimal>>>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct QuoteList {
    pub volume: Option<Vec<Option<u64>>>,
    pub high: Option<Vec<Option<Decimal>>>,
    pub close: Option<Vec<Option<Decimal>>>,
    pub low: Option<Vec<Option<Decimal>>>,
    pub open: Option<Vec<Option<Decimal>>>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct EventsBlock {
    pub splits: Option<HashMap<i64, Split>>,
    pub dividends: Option<HashMap<i64, Dividend>>,
    #[serde(rename = "capitalGains")]
    pub capital_gains: Option<HashMap<i64, CapitalGain>>,
}

/// This structure simply models a split that has occured.
#[derive(Deserialize, Debug, Clone, Serialize)]
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
#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct Dividend {
    /// This is the price of the dividend
    pub amount: Decimal,
    /// This is the ex-dividend date as UNIX timestamp
    pub date: i64,
}

/// This structure simply models a capital gain which has been recorded.
#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct CapitalGain {
    /// This is the amount of capital gain distributed by the fund
    pub amount: f64,
    /// This is the recorded date of the capital gain
    pub date: i64,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YQuoteSummary {
    #[serde(rename = "quoteSummary")]
    pub quote_summary: Option<ExtendedQuoteSummary>,
    pub finance: Option<YFinance>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct YFinance {
    pub result: Option<serde_json::Value>,
    pub error: Option<YErrorMessage>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct YErrorMessage {
    pub code: Option<String>,
    pub description: Option<String>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct ExtendedQuoteSummary {
    pub result: Option<Vec<YSummaryData>>,
    pub error: Option<YErrorMessage>,
}

impl YQuoteSummary {
    pub fn from_json(json: serde_json::Value) -> Result<YQuoteSummary, YahooError> {
        Ok(serde_json::from_value(json)?)
    }
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YSummaryData {
    pub asset_profile: Option<AssetProfile>,
    pub summary_detail: Option<SummaryDetail>,
    pub default_key_statistics: Option<DefaultKeyStatistics>,
    pub quote_type: Option<QuoteType>,
    pub financial_data: Option<FinancialData>,
    pub recommendation_trend: Option<RecommendationTrend>,
    pub earnings_trend: Option<EarningsTrend>,
    pub earnings_history: Option<EarningsHistory>,
    pub earnings: Option<Earnings>,
    pub upgrade_downgrade_history: Option<UpgradeDowngradeHistory>,
    pub calendar_events: Option<CalendarEvents>,
    pub insider_holders: Option<InsiderHolders>,
    pub insider_transactions: Option<InsiderTransactions>,
    pub major_holders_breakdown: Option<MajorHoldersBreakdown>,
    pub institution_ownership: Option<InstitutionOwnership>,
    pub fund_ownership: Option<FundOwnership>,
    pub net_share_purchase_activity: Option<NetSharePurchaseActivity>,
    pub fund_profile: Option<FundProfile>,
    pub top_holdings: Option<TopHoldings>,
    pub sec_filings: Option<SecFilings>,
}

#[derive(Deserialize, Debug, Serialize)]
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

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanyOfficer {
    pub name: String,
    pub age: Option<u32>,
    pub title: String,
    pub year_born: Option<u32>,
    pub fiscal_year: Option<u32>,
    pub total_pay: Option<ValueWrapper>,
}

#[derive(Deserialize, Debug, Serialize)]
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

#[derive(Deserialize, Debug, Serialize)]
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
    #[serde(rename = "yield")]
    pub yield_: Option<f64>,
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

#[derive(Deserialize, Debug, Serialize)]
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
    #[serde(rename = "yield")]
    pub _yield: Option<f64>,
}

#[derive(Deserialize, Debug, Serialize)]
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

#[derive(Deserialize, Debug, Serialize)]
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

/// A numeric value with Yahoo's formatted string variants (`{raw, fmt, longFmt}`).
#[derive(Deserialize, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RawValue {
    pub raw: Option<f64>,
    pub fmt: Option<String>,
    pub long_fmt: Option<String>,
}

/// `recommendationTrend` module: analyst recommendation counts over the last months.
#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationTrend {
    pub max_age: Option<i64>,
    pub trend: Vec<RecommendationTrendItem>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationTrendItem {
    pub period: Option<String>,
    pub strong_buy: Option<i64>,
    pub buy: Option<i64>,
    pub hold: Option<i64>,
    pub sell: Option<i64>,
    pub strong_sell: Option<i64>,
}

/// `earningsTrend` module: analyst estimates (earnings, revenue, growth) per period.
#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EarningsTrend {
    pub max_age: Option<i64>,
    pub trend: Vec<EarningsTrendItem>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EarningsTrendItem {
    pub max_age: Option<i64>,
    pub period: Option<String>,
    pub end_date: Option<String>,
    pub growth: Option<RawValue>,
    pub earnings_estimate: Option<EarningsEstimate>,
    pub revenue_estimate: Option<RevenueEstimate>,
    pub eps_trend: Option<EpsTrend>,
    pub eps_revisions: Option<EpsRevisions>,
    pub growth_estimate: Option<GrowthEstimate>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EarningsEstimate {
    pub avg: Option<RawValue>,
    pub low: Option<RawValue>,
    pub high: Option<RawValue>,
    pub year_ago_eps: Option<RawValue>,
    pub number_of_analysts: Option<RawValue>,
    pub growth: Option<RawValue>,
    pub earnings_currency: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevenueEstimate {
    pub avg: Option<RawValue>,
    pub low: Option<RawValue>,
    pub high: Option<RawValue>,
    pub number_of_analysts: Option<RawValue>,
    pub year_ago_revenue: Option<RawValue>,
    pub growth: Option<RawValue>,
    pub revenue_currency: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EpsTrend {
    pub current: Option<RawValue>,
    #[serde(rename = "7daysAgo")]
    pub seven_days_ago: Option<RawValue>,
    #[serde(rename = "30daysAgo")]
    pub thirty_days_ago: Option<RawValue>,
    #[serde(rename = "60daysAgo")]
    pub sixty_days_ago: Option<RawValue>,
    #[serde(rename = "90daysAgo")]
    pub ninety_days_ago: Option<RawValue>,
    pub eps_trend_currency: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EpsRevisions {
    pub up_last_7days: Option<RawValue>,
    pub up_last_30days: Option<RawValue>,
    pub down_last_30days: Option<RawValue>,
    #[serde(rename = "downLast7Days")]
    pub down_last7_days: Option<RawValue>,
    pub down_last_90days: Option<RawValue>,
    pub eps_revisions_currency: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrowthEstimate {
    pub avg: Option<RawValue>,
    pub low: Option<RawValue>,
    pub high: Option<RawValue>,
    pub year_ago_eps: Option<RawValue>,
    pub number_of_analysts: Option<RawValue>,
    pub growth: Option<RawValue>,
    pub earnings_currency: Option<String>,
}

/// `earningsHistory` module: actual vs estimated EPS per past quarter.
#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EarningsHistory {
    pub max_age: Option<i64>,
    pub default_methodology: Option<String>,
    pub history: Vec<EarningsHistoryItem>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EarningsHistoryItem {
    pub max_age: Option<i64>,
    pub eps_actual: Option<RawValue>,
    pub eps_estimate: Option<RawValue>,
    pub eps_difference: Option<RawValue>,
    pub surprise_percent: Option<RawValue>,
    pub quarter: Option<RawValue>,
    pub currency: Option<String>,
    pub period: Option<String>,
}

/// `earnings` module: earnings charts and current quarter estimates.
#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Earnings {
    pub max_age: Option<i64>,
    pub earnings_chart: Option<EarningsChart>,
    pub financials_chart: Option<FinancialsChart>,
    pub financial_currency: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EarningsChart {
    pub quarterly: Vec<EarningsChartQuarterly>,
    pub current_quarter_estimate: Option<f64>,
    pub current_quarter_estimate_date: Option<String>,
    pub current_calendar_quarter: Option<String>,
    pub current_quarter_estimate_year: Option<i64>,
    pub current_fiscal_quarter: Option<String>,
    pub current_period_end_date: Option<i64>,
    pub earnings_date: Option<Vec<i64>>,
    pub is_earnings_date_estimate: Option<bool>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EarningsChartQuarterly {
    pub date: Option<String>,
    pub actual: Option<f64>,
    pub estimate: Option<f64>,
    pub fiscal_quarter: Option<String>,
    pub calendar_quarter: Option<String>,
    pub difference: Option<String>,
    pub surprise_pct: Option<String>,
    pub period_end_date: Option<i64>,
    pub reported_date: Option<i64>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancialsChart {
    pub yearly: Vec<FinancialsChartYearly>,
    pub quarterly: Vec<FinancialsChartQuarterly>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancialsChartYearly {
    pub date: Option<i64>,
    pub revenue: Option<f64>,
    pub earnings: Option<f64>,
    pub profit_margin: Option<f64>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancialsChartQuarterly {
    pub date: Option<String>,
    pub fiscal_quarter: Option<String>,
    pub revenue: Option<f64>,
    pub earnings: Option<f64>,
    pub profit_margin: Option<f64>,
}

/// `upgradeDowngradeHistory` module: analyst rating changes.
#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeDowngradeHistory {
    pub max_age: Option<i64>,
    pub history: Vec<UpgradeDowngradeItem>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeDowngradeItem {
    pub epoch_grade_date: Option<i64>,
    pub firm: Option<String>,
    pub to_grade: Option<String>,
    pub from_grade: Option<String>,
    pub action: Option<String>,
    pub price_target_action: Option<String>,
    pub current_price_target: Option<f64>,
    pub prior_price_target: Option<f64>,
}

/// `calendarEvents` module: upcoming earnings, dividend and ex-dividend dates.
#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarEvents {
    pub max_age: Option<i64>,
    pub earnings: Option<CalendarEarnings>,
    pub ex_dividend_date: Option<i64>,
    pub dividend_date: Option<i64>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarEarnings {
    pub earnings_date: Option<Vec<i64>>,
    pub earnings_call_date: Option<Vec<i64>>,
    pub is_earnings_date_estimate: Option<bool>,
    pub earnings_average: Option<f64>,
    pub earnings_low: Option<f64>,
    pub earnings_high: Option<f64>,
    pub revenue_average: Option<f64>,
    pub revenue_low: Option<f64>,
    pub revenue_high: Option<f64>,
}

/// `insiderHolders` module: insider position details.
#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InsiderHolders {
    pub max_age: Option<i64>,
    pub holders: Vec<InsiderHolder>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InsiderHolder {
    pub max_age: Option<i64>,
    pub name: Option<String>,
    pub relation: Option<String>,
    pub url: Option<String>,
    pub transaction_description: Option<String>,
    pub latest_trans_date: Option<RawValue>,
    pub position_direct: Option<RawValue>,
    pub position_direct_date: Option<RawValue>,
    pub position_indirect: Option<RawValue>,
    pub position_indirect_date: Option<RawValue>,
    pub shares: Option<RawValue>,
    pub value: Option<RawValue>,
}

/// `insiderTransactions` module: recent insider share transactions.
#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InsiderTransactions {
    pub max_age: Option<i64>,
    pub transactions: Vec<InsiderTransaction>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InsiderTransaction {
    pub max_age: Option<i64>,
    pub shares: Option<RawValue>,
    pub value: Option<RawValue>,
    pub filer_url: Option<String>,
    pub transaction_text: Option<String>,
    pub filer_name: Option<String>,
    pub filer_relation: Option<String>,
    pub money_text: Option<String>,
    pub start_date: Option<RawValue>,
    pub ownership: Option<String>,
}

/// `majorHoldersBreakdown` module: aggregate insider/institution ownership percentages.
#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MajorHoldersBreakdown {
    pub max_age: Option<i64>,
    pub insiders_percent_held: Option<f64>,
    pub institutions_percent_held: Option<f64>,
    pub institutions_float_percent_held: Option<f64>,
    pub institutions_count: Option<i64>,
}

/// `institutionOwnership` module: top institutional holders.
#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstitutionOwnership {
    pub max_age: Option<i64>,
    pub ownership_list: Vec<InstitutionOwnershipItem>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstitutionOwnershipItem {
    pub max_age: Option<i64>,
    pub report_date: Option<RawValue>,
    pub organization: Option<String>,
    pub pct_held: Option<RawValue>,
    pub position: Option<RawValue>,
    pub value: Option<RawValue>,
    pub pct_change: Option<RawValue>,
}

/// `fundOwnership` module: top mutual fund holders.
#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FundOwnership {
    pub max_age: Option<i64>,
    pub ownership_list: Vec<FundOwnershipItem>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FundOwnershipItem {
    pub max_age: Option<i64>,
    pub report_date: Option<RawValue>,
    pub organization: Option<String>,
    pub pct_held: Option<RawValue>,
    pub position: Option<RawValue>,
    pub value: Option<RawValue>,
    pub pct_change: Option<RawValue>,
}

/// `netSharePurchaseActivity` module: aggregate insider buying/selling activity.
#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetSharePurchaseActivity {
    pub max_age: Option<i64>,
    pub period: Option<String>,
    pub buy_info_count: Option<i64>,
    pub buy_info_shares: Option<i64>,
    pub buy_percent_insider_shares: Option<f64>,
    pub sell_info_count: Option<i64>,
    pub sell_info_shares: Option<i64>,
    pub sell_percent_insider_shares: Option<f64>,
    pub net_info_count: Option<i64>,
    pub net_info_shares: Option<i64>,
    pub net_percent_insider_shares: Option<f64>,
    pub net_inst_shares_buying: Option<i64>,
    pub net_inst_buying_percent: Option<f64>,
    pub total_insider_shares: Option<i64>,
}

/// `fundProfile` module: fund metadata (management, fees, category).
#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FundProfile {
    pub max_age: Option<i64>,
    pub style_box_url: Option<String>,
    pub family: Option<String>,
    pub category_name: Option<String>,
    pub legal_type: Option<String>,
    pub management_info: Option<FundManagementInfo>,
    pub fees_expenses_investment: Option<FundFeesExpenses>,
    pub fees_expenses_investment_cat: Option<FundFeesExpenses>,
    pub brokerages: Option<Vec<serde_json::Value>>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FundManagementInfo {
    pub manager_name: Option<String>,
    pub manager_bio: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FundFeesExpenses {
    pub annual_report_expense_ratio: Option<f64>,
    pub annual_holdings_turnover: Option<f64>,
    pub total_net_assets: Option<f64>,
    pub projection_values: Option<RawValue>,
}

/// `topHoldings` module: fund's top holdings and asset allocation.
#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopHoldings {
    pub max_age: Option<i64>,
    pub cash_position: Option<f64>,
    pub stock_position: Option<f64>,
    pub bond_position: Option<f64>,
    pub other_position: Option<f64>,
    pub preferred_position: Option<f64>,
    pub convertible_position: Option<f64>,
    pub holdings: Vec<TopHolding>,
    pub equity_holdings: Option<FundValuation>,
    pub bond_holdings: Option<FundValuation>,
    pub bond_ratings: Vec<HashMap<String, f64>>,
    pub sector_weightings: Vec<HashMap<String, f64>>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopHolding {
    pub symbol: Option<String>,
    pub holding_name: Option<String>,
    pub holding_percent: Option<f64>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FundValuation {
    pub price_to_earnings: Option<f64>,
    pub price_to_book: Option<f64>,
    pub price_to_sales: Option<f64>,
    pub price_to_cashflow: Option<f64>,
}

/// `secFilings` module: SEC filings list.
#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecFilings {
    pub max_age: Option<i64>,
    pub filings: Vec<SecFiling>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecFiling {
    pub date: Option<String>,
    pub epoch_date: Option<i64>,
    #[serde(rename = "type")]
    pub filing_type: Option<String>,
    pub title: Option<String>,
    pub edgar_url: Option<String>,
    pub exhibits: Option<Vec<SecFilingExhibit>>,
    pub max_age: Option<i64>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecFilingExhibit {
    #[serde(rename = "type")]
    pub exhibit_type: Option<String>,
    pub url: Option<String>,
}

// Structs for the earnings dates response
#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct YEarningsResponse {
    pub finance: YEarningsFinance,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct YEarningsFinance {
    pub result: Vec<YEarningsResult>,
    pub error: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct YEarningsResult {
    pub documents: Vec<YEarningsDocument>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct YEarningsDocument {
    pub columns: Vec<YEarningsColumn>,
    pub rows: Vec<Vec<serde_json::Value>>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct YEarningsColumn {
    pub label: String,
}

#[serde_as]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FinancialEvent {
    // Custom serialization logic
    #[serde_as(as = "DisplayFromStr")]
    pub earnings_date: OffsetDateTime,
    pub event_type: String,
    pub eps_estimate: Option<f64>,
    pub reported_eps: Option<f64>,
    pub surprise_percent: Option<f64>,
    pub timezone: Option<String>,
}

// Implement serde_date_format

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

    #[test]
    fn test_deserialize_yield_field_in_summary_detail_and_default_key_statistics() {
        let summary_json = r#"{
            "yield": 0.42
        }"#;
        let summary: SummaryDetail = serde_json::from_str(summary_json).unwrap();
        assert_eq!(summary.yield_, Some(0.42));

        let stats_json = r#"{
            "yield": 1.23
        }"#;
        let stats: DefaultKeyStatistics = serde_json::from_str(stats_json).unwrap();
        assert_eq!(stats._yield, Some(1.23));
    }

    #[test]
    fn test_yresponse_serde() {
        let yr: YResponse = YResponse {
            chart: YChart {
                result: Some(vec![YQuoteBlock {
                    meta: YMetaData {
                        currency: Some("USD".to_string()),
                        symbol: "AAPL".to_string(),
                        long_name: Some("Apple Inc.".to_string()),
                        short_name: Some("AAPL".to_string()),
                        instrument_type: "EQUITY".to_string(),
                        exchange_name: "NASDAQ".to_string(),
                        ..Default::default()
                    },
                    ..Default::default()
                }]),
                error: None,
            },
        };
        let s = serde_json::to_string_pretty(&yr).unwrap();
        println!("{}", s);
        let expected = r#"{
  "chart": {
    "result": [
      {
        "meta": {
          "currency": "USD",
          "symbol": "AAPL",
          "longName": "Apple Inc.",
          "shortName": "AAPL",
          "instrumentType": "EQUITY",
          "exchangeName": "NASDAQ",
          "fullExchangeName": "",
          "firstTradeDate": null,
          "regularMarketTime": null,
          "gmtoffset": 0,
          "timezone": "",
          "exchangeTimezoneName": "",
          "regularMarketPrice": null,
          "chartPreviousClose": null,
          "previousClose": null,
          "hasPrePostMarketData": false,
          "fiftyTwoWeekHigh": null,
          "fiftyTwoWeekLow": null,
          "regularMarketDayHigh": null,
          "regularMarketDayLow": null,
          "regularMarketVolume": null,
          "scale": null,
          "priceHint": 0,
          "currentTradingPeriod": {
            "pre": {
              "timezone": "",
              "start": 0,
              "end": 0,
              "gmtoffset": 0
            },
            "regular": {
              "timezone": "",
              "start": 0,
              "end": 0,
              "gmtoffset": 0
            },
            "post": {
              "timezone": "",
              "start": 0,
              "end": 0,
              "gmtoffset": 0
            }
          },
          "tradingPeriods": {
            "pre": null,
            "regular": null,
            "post": null
          },
          "dataGranularity": "",
          "range": "",
          "validRanges": []
        },
        "timestamp": null,
        "events": null,
        "indicators": {
          "quote": [],
          "adjclose": null
        }
      }
    ],
    "error": null
  }
}"#;
        assert_eq!(s, expected);
    }

    #[test]
    fn test_deserialize_recommendation_trend() {
        let json = r#"
        {
            "maxAge": 86400,
            "trend": [
                {"period": "0m", "strongBuy": 6, "buy": 21, "hold": 15, "sell": 2, "strongSell": 2},
                {"period": "-1m", "strongBuy": 6, "buy": 22, "hold": 14, "sell": 2, "strongSell": 2}
            ]
        }
        "#;
        let trend: RecommendationTrend = serde_json::from_str(json).unwrap();
        assert_eq!(trend.trend.len(), 2);
        assert_eq!(trend.trend[0].period.as_deref(), Some("0m"));
        assert_eq!(trend.trend[0].strong_buy, Some(6));
        assert_eq!(trend.trend[0].strong_sell, Some(2));
        assert_eq!(trend.trend[1].hold, Some(14));
    }

    #[test]
    fn test_deserialize_earnings_trend() {
        let json = r#"
        {
            "maxAge": 86400,
            "trend": [
                {
                    "maxAge": 1,
                    "period": "0q",
                    "endDate": "2026-09-30",
                    "growth": {"raw": 0.0683, "fmt": "6.83%"},
                    "earningsEstimate": {
                        "avg": {"raw": 1.97549, "fmt": "1.98"},
                        "low": {"raw": 1.93, "fmt": "1.93"},
                        "high": {"raw": 2.04, "fmt": "2.04"},
                        "yearAgoEps": {"raw": 1.85, "fmt": "1.85"},
                        "numberOfAnalysts": {"raw": 28, "fmt": "28", "longFmt": "28"},
                        "growth": {"raw": 0.0678, "fmt": "6.78%"},
                        "earningsCurrency": "USD"
                    },
                    "revenueEstimate": {
                        "avg": {"raw": 113256580210, "fmt": "113.26B"},
                        "numberOfAnalysts": {"raw": 26, "fmt": "26"},
                        "yearAgoRevenue": {"raw": 102466000000, "fmt": "102.47B"},
                        "growth": {"raw": 0.1053, "fmt": "10.53%"},
                        "revenueCurrency": "USD"
                    },
                    "epsTrend": {
                        "current": {"raw": 1.97549, "fmt": "1.98"},
                        "7daysAgo": {"raw": 2.01204, "fmt": "2.01"},
                        "30daysAgo": {"raw": 2.00836, "fmt": "2.01"},
                        "60daysAgo": {"raw": 2.00767, "fmt": "2.01"},
                        "90daysAgo": {"raw": 2.00379, "fmt": "2"},
                        "epsTrendCurrency": "USD"
                    },
                    "epsRevisions": {
                        "upLast7days": {"raw": 1, "fmt": "1"},
                        "upLast30days": {"raw": 4, "fmt": "4"},
                        "downLast30days": {"raw": 2, "fmt": "2"},
                        "downLast7Days": {"raw": 1, "fmt": "1"},
                        "downLast90days": {},
                        "epsRevisionsCurrency": "USD"
                    },
                    "growthEstimate": null
                }
            ]
        }
        "#;
        let et: EarningsTrend = serde_json::from_str(json).unwrap();
        let item = &et.trend[0];
        assert_eq!(item.end_date.as_deref(), Some("2026-09-30"));
        assert_eq!(item.growth.as_ref().unwrap().raw, Some(0.0683));
        let ee = item.earnings_estimate.as_ref().unwrap();
        assert_eq!(ee.avg.as_ref().unwrap().raw, Some(1.97549));
        assert_eq!(ee.number_of_analysts.as_ref().unwrap().raw, Some(28.0));
        let eps_trend = item.eps_trend.as_ref().unwrap();
        assert_eq!(eps_trend.seven_days_ago.as_ref().unwrap().raw, Some(2.01204));
        assert_eq!(eps_trend.thirty_days_ago.as_ref().unwrap().raw, Some(2.00836));
        assert_eq!(eps_trend.sixty_days_ago.as_ref().unwrap().raw, Some(2.00767));
        assert_eq!(eps_trend.ninety_days_ago.as_ref().unwrap().raw, Some(2.00379));
        let eps_rev = item.eps_revisions.as_ref().unwrap();
        assert_eq!(eps_rev.up_last_7days.as_ref().unwrap().raw, Some(1.0));
        assert_eq!(eps_rev.down_last7_days.as_ref().unwrap().raw, Some(1.0));
        assert!(eps_rev.down_last_90days.as_ref().unwrap().raw.is_none());
        assert!(item.growth_estimate.is_none());
    }

    #[test]
    fn test_deserialize_earnings_history() {
        let json = r#"
        {
            "history": [
                {
                    "maxAge": 1,
                    "epsActual": {"raw": 1.85, "fmt": "1.85"},
                    "epsEstimate": {"raw": 1.76993, "fmt": "1.77"},
                    "epsDifference": {"raw": 0.08, "fmt": "0.08"},
                    "surprisePercent": {"raw": 0.0452, "fmt": "4.52%"},
                    "quarter": {"raw": 1759190400, "fmt": "2025-09-30"},
                    "currency": "USD",
                    "period": "-4q"
                }
            ],
            "defaultMethodology": "gaap",
            "maxAge": 86400
        }
        "#;
        let eh: EarningsHistory = serde_json::from_str(json).unwrap();
        let item = &eh.history[0];
        assert_eq!(item.eps_actual.as_ref().unwrap().raw, Some(1.85));
        assert_eq!(item.surprise_percent.as_ref().unwrap().raw, Some(0.0452));
        assert_eq!(item.quarter.as_ref().unwrap().raw, Some(1759190400.0));
        assert_eq!(item.period.as_deref(), Some("-4q"));
        assert_eq!(eh.default_methodology.as_deref(), Some("gaap"));
    }

    #[test]
    fn test_deserialize_earnings() {
        let json = r#"
        {
            "maxAge": 86400,
            "earningsChart": {
                "quarterly": [
                    {
                        "date": "3Q2025",
                        "actual": 1.85,
                        "estimate": 1.76993,
                        "fiscalQuarter": "4Q2025",
                        "calendarQuarter": "3Q2025",
                        "difference": "0.08",
                        "surprisePct": "4.52",
                        "periodEndDate": 1759190400,
                        "reportedDate": 1761856200
                    }
                ],
                "currentQuarterEstimate": 1.97549,
                "currentQuarterEstimateDate": "3Q",
                "currentCalendarQuarter": "3Q2026",
                "currentQuarterEstimateYear": 2026,
                "currentFiscalQuarter": "4Q2026",
                "currentPeriodEndDate": 1790726400,
                "earningsDate": [1793304000],
                "isEarningsDateEstimate": true
            },
            "financialsChart": {
                "yearly": [
                    {"date": 2022, "revenue": 394328000000, "earnings": 99803000000, "profitMargin": 0.2530964}
                ],
                "quarterly": [
                    {"date": "2Q2025", "fiscalQuarter": "3Q2025", "revenue": 94036000000, "earnings": 23434000000, "profitMargin": 0.24920243}
                ]
            },
            "financialCurrency": "USD"
        }
        "#;
        let e: Earnings = serde_json::from_str(json).unwrap();
        let chart = e.earnings_chart.as_ref().unwrap();
        assert_eq!(chart.quarterly[0].date.as_deref(), Some("3Q2025"));
        assert_eq!(chart.quarterly[0].difference.as_deref(), Some("0.08"));
        assert_eq!(chart.quarterly[0].actual, Some(1.85));
        assert_eq!(chart.current_quarter_estimate, Some(1.97549));
        assert_eq!(chart.earnings_date.as_ref().unwrap(), &vec![1793304000]);
        let fin = e.financials_chart.as_ref().unwrap();
        assert_eq!(fin.yearly[0].date, Some(2022));
        assert_eq!(fin.yearly[0].revenue, Some(394328000000.0));
        assert_eq!(fin.quarterly[0].date.as_deref(), Some("2Q2025"));
    }

    #[test]
    fn test_deserialize_upgrade_downgrade_history() {
        let json = r#"
        {
            "history": [
                {
                    "epochGradeDate": 1786365485,
                    "firm": "Jefferies",
                    "toGrade": "Underperform",
                    "fromGrade": "Hold",
                    "action": "down",
                    "priceTargetAction": "Lowers",
                    "currentPriceTarget": 263.66,
                    "priorPriceTarget": 285.56
                }
            ],
            "maxAge": 86400
        }
        "#;
        let udh: UpgradeDowngradeHistory = serde_json::from_str(json).unwrap();
        let item = &udh.history[0];
        assert_eq!(item.firm.as_deref(), Some("Jefferies"));
        assert_eq!(item.to_grade.as_deref(), Some("Underperform"));
        assert_eq!(item.action.as_deref(), Some("down"));
        assert_eq!(item.current_price_target, Some(263.66));
    }

    #[test]
    fn test_deserialize_calendar_events() {
        let json = r#"
        {
            "maxAge": 1,
            "earnings": {
                "earningsDate": [1793304000],
                "earningsCallDate": [1785441600],
                "isEarningsDateEstimate": true,
                "earningsAverage": 1.97643,
                "earningsLow": 1.93,
                "earningsHigh": 2.04,
                "revenueAverage": 113256580210,
                "revenueLow": 112137000000,
                "revenueHigh": 115068279000
            },
            "exDividendDate": 1786320000,
            "dividendDate": 1786579200
        }
        "#;
        let ce: CalendarEvents = serde_json::from_str(json).unwrap();
        let er = ce.earnings.as_ref().unwrap();
        assert_eq!(er.earnings_date.as_ref().unwrap(), &vec![1793304000]);
        assert_eq!(er.earnings_high, Some(2.04));
        assert_eq!(er.revenue_low, Some(112137000000.0));
        assert_eq!(ce.ex_dividend_date, Some(1786320000));
        assert_eq!(ce.dividend_date, Some(1786579200));
    }

    #[test]
    fn test_deserialize_insider_holders() {
        let json = r#"
        {
            "holders": [
                {
                    "maxAge": 1,
                    "name": "COOK TIMOTHY D",
                    "relation": "Chief Executive Officer",
                    "url": "",
                    "transactionDescription": "Sale",
                    "latestTransDate": {"raw": 1775088000, "fmt": "2026-04-02"},
                    "positionDirect": {"raw": 3280420, "fmt": "3.28M", "longFmt": "3,280,420"},
                    "positionDirectDate": {"raw": 1775088000, "fmt": "2026-04-02"}
                }
            ],
            "maxAge": 1
        }
        "#;
        let ih: InsiderHolders = serde_json::from_str(json).unwrap();
        let h = &ih.holders[0];
        assert_eq!(h.name.as_deref(), Some("COOK TIMOTHY D"));
        assert_eq!(h.relation.as_deref(), Some("Chief Executive Officer"));
        assert_eq!(h.position_direct.as_ref().unwrap().raw, Some(3280420.0));
        assert_eq!(
            h.position_direct.as_ref().unwrap().long_fmt.as_deref(),
            Some("3,280,420")
        );
        assert!(h.position_indirect.is_none());
    }

    #[test]
    fn test_deserialize_insider_transactions() {
        let json = r#"
        {
            "transactions": [
                {
                    "maxAge": 1,
                    "shares": {"raw": 116, "fmt": "116", "longFmt": "116"},
                    "value": {"raw": 34236, "fmt": "34.24k", "longFmt": "34,236"},
                    "filerUrl": "",
                    "transactionText": "Sale at price 295.14 per share.",
                    "filerName": "BORDERS BEN",
                    "filerRelation": "Officer",
                    "moneyText": "",
                    "startDate": {"raw": 1781568000, "fmt": "2026-06-16"},
                    "ownership": "D"
                },
                {
                    "maxAge": 1,
                    "shares": {"raw": 30104, "fmt": "30.1k", "longFmt": "30,104"},
                    "filerUrl": "",
                    "transactionText": "",
                    "filerName": "NEWSTEAD JENNIFER",
                    "filerRelation": "General Counsel",
                    "startDate": {"raw": 1781481600, "fmt": "2026-06-15"}
                }
            ],
            "maxAge": 1
        }
        "#;
        let it: InsiderTransactions = serde_json::from_str(json).unwrap();
        assert_eq!(it.transactions[0].filer_name.as_deref(), Some("BORDERS BEN"));
        assert_eq!(it.transactions[0].value.as_ref().unwrap().raw, Some(34236.0));
        assert_eq!(it.transactions[0].ownership.as_deref(), Some("D"));
        assert!(it.transactions[1].value.is_none());
        assert_eq!(
            it.transactions[1].filer_relation.as_deref(),
            Some("General Counsel")
        );
    }

    #[test]
    fn test_deserialize_major_holders_breakdown() {
        let json = r#"
        {
            "maxAge": 1,
            "insidersPercentHeld": 0.01647,
            "institutionsPercentHeld": 0.66289,
            "institutionsFloatPercentHeld": 0.67399,
            "institutionsCount": 7670
        }
        "#;
        let mhb: MajorHoldersBreakdown = serde_json::from_str(json).unwrap();
        assert_eq!(mhb.insiders_percent_held, Some(0.01647));
        assert_eq!(mhb.institutions_count, Some(7670));
    }

    #[test]
    fn test_deserialize_ownership_modules() {
        let json = r#"
        {
            "maxAge": 1,
            "ownershipList": [
                {
                    "maxAge": 1,
                    "reportDate": {"raw": 1782777600, "fmt": "2026-06-30"},
                    "organization": "Blackrock Inc.",
                    "pctHeld": {"raw": 0.0797, "fmt": "7.97%"},
                    "position": {"raw": 1162996939, "fmt": "1.16B", "longFmt": "1,162,996,939"},
                    "value": {"raw": 356766773028, "fmt": "356.77B", "longFmt": "356,766,773,028"},
                    "pctChange": {"raw": 0.016, "fmt": "1.60%"}
                }
            ]
        }
        "#;
        let inst: InstitutionOwnership = serde_json::from_str(json).unwrap();
        assert_eq!(
            inst.ownership_list[0].organization.as_deref(),
            Some("Blackrock Inc.")
        );
        assert_eq!(
            inst.ownership_list[0].position.as_ref().unwrap().raw,
            Some(1162996939.0)
        );
        assert_eq!(inst.ownership_list[0].pct_change.as_ref().unwrap().raw, Some(0.016));

        let fund: FundOwnership = serde_json::from_str(json).unwrap();
        assert_eq!(
            fund.ownership_list[0].organization.as_deref(),
            Some("Blackrock Inc.")
        );
    }

    #[test]
    fn test_deserialize_net_share_purchase_activity() {
        let json = r#"
        {
            "maxAge": 1,
            "period": "6m",
            "buyInfoCount": 11,
            "buyInfoShares": 434520,
            "buyPercentInsiderShares": 0.002,
            "sellInfoCount": 7,
            "sellInfoShares": 397875,
            "sellPercentInsiderShares": 0.002,
            "netInfoCount": 18,
            "netInfoShares": 36645,
            "netPercentInsiderShares": 0.0,
            "netInstSharesBuying": 62874610,
            "netInstBuyingPercent": 0.0065,
            "totalInsiderShares": 240366144
        }
        "#;
        let nsp: NetSharePurchaseActivity = serde_json::from_str(json).unwrap();
        assert_eq!(nsp.period.as_deref(), Some("6m"));
        assert_eq!(nsp.buy_info_count, Some(11));
        assert_eq!(nsp.net_info_shares, Some(36645));
        assert_eq!(nsp.buy_percent_insider_shares, Some(0.002));
    }

    #[test]
    fn test_deserialize_sec_filings() {
        let json = r#"
        {
            "filings": [
                {
                    "date": "2026-07-31",
                    "epochDate": 1785456000,
                    "type": "10-Q",
                    "title": "Periodic Financial Reports",
                    "edgarUrl": "https://finance.yahoo.com/sec-filing/AAPL/0000320193-26-000020_320193",
                    "exhibits": [
                        {"type": "EX-31.1", "url": "https://cdn.yahoofinance.com/prod/sec-filings/ex.htm"}
                    ],
                    "maxAge": 1
                }
            ],
            "maxAge": 86400
        }
        "#;
        let sf: SecFilings = serde_json::from_str(json).unwrap();
        let filing = &sf.filings[0];
        assert_eq!(filing.filing_type.as_deref(), Some("10-Q"));
        assert_eq!(filing.epoch_date, Some(1785456000));
        assert_eq!(
            filing.edgar_url.as_deref(),
            Some("https://finance.yahoo.com/sec-filing/AAPL/0000320193-26-000020_320193")
        );
        let exhibits = filing.exhibits.as_ref().unwrap();
        assert_eq!(exhibits[0].exhibit_type.as_deref(), Some("EX-31.1"));
    }

    #[test]
    fn test_deserialize_fund_profile() {
        let json = r#"
        {
            "maxAge": 1,
            "styleBoxUrl": "https://s.yimg.com/lq/i/fi/3_0stylelargeeq2.gif",
            "family": "Vanguard",
            "categoryName": "Large Blend",
            "legalType": "Exchange Traded Fund",
            "managementInfo": {"managerName": null, "managerBio": null},
            "feesExpensesInvestment": {
                "annualReportExpenseRatio": 0.0003,
                "annualHoldingsTurnover": 0.02,
                "totalNetAssets": 486952.38,
                "projectionValues": {}
            },
            "feesExpensesInvestmentCat": {
                "annualReportExpenseRatio": 0.0004,
                "annualHoldingsTurnover": 0.03,
                "totalNetAssets": 500000.0
            },
            "brokerages": []
        }
        "#;
        let fp: FundProfile = serde_json::from_str(json).unwrap();
        assert_eq!(fp.family.as_deref(), Some("Vanguard"));
        assert_eq!(fp.category_name.as_deref(), Some("Large Blend"));
        let fees = fp.fees_expenses_investment.as_ref().unwrap();
        assert_eq!(fees.annual_report_expense_ratio, Some(0.0003));
        assert!(fees.projection_values.as_ref().unwrap().raw.is_none());
        let fees_cat = fp.fees_expenses_investment_cat.as_ref().unwrap();
        assert_eq!(fees_cat.annual_holdings_turnover, Some(0.03));
    }

    #[test]
    fn test_deserialize_top_holdings() {
        let json = r#"
        {
            "maxAge": 1,
            "cashPosition": 0.0022,
            "stockPosition": 0.9957,
            "bondPosition": 0.0,
            "otherPosition": 0.002,
            "preferredPosition": 0.0,
            "convertiblePosition": 0.0,
            "holdings": [
                {"symbol": "NVDA", "holdingName": "NVIDIA Corp", "holdingPercent": 0.074991}
            ],
            "equityHoldings": {"priceToEarnings": 29.6, "priceToBook": 9.3, "priceToSales": 8.6, "priceToCashflow": 21.0},
            "bondHoldings": {},
            "bondRatings": [{"us_government": 0.0}],
            "sectorWeightings": [{"realestate": 0.0183}, {"technology": 0.334}]
        }
        "#;
        let th: TopHoldings = serde_json::from_str(json).unwrap();
        assert_eq!(th.stock_position, Some(0.9957));
        assert_eq!(th.holdings[0].symbol.as_deref(), Some("NVDA"));
        assert_eq!(th.holdings[0].holding_percent, Some(0.074991));
        assert_eq!(
            th.equity_holdings.as_ref().unwrap().price_to_earnings,
            Some(29.6)
        );
        assert!(th.bond_holdings.as_ref().unwrap().price_to_earnings.is_none());
        assert_eq!(th.sector_weightings[1].get("technology"), Some(&0.334));
        assert_eq!(th.bond_ratings[0].get("us_government"), Some(&0.0));
    }

    // Full-coverage tests: deserialize the real (hardcoded) API responses
    // captured in tests/fixtures and assert every field of the new modules.

    const AAPL_FIXTURE: &str = include_str!("../tests/fixtures/quote_summary_aapl.json");
    const VOO_FIXTURE: &str = include_str!("../tests/fixtures/quote_summary_voo.json");

    fn load_summary(fixture: &str) -> YSummaryData {
        let summary: YQuoteSummary = serde_json::from_str(fixture).unwrap();
        summary.quote_summary.unwrap().result.unwrap().remove(0)
    }

    #[test]
    fn test_full_recommendation_trend() {
        let s = load_summary(AAPL_FIXTURE);
        let trend = s.recommendation_trend.unwrap();
        assert_eq!(trend.max_age, Some(86400));
        assert_eq!(trend.trend.len(), 4);
        assert_eq!(trend.trend[0].period.as_deref(), Some("0m"));
        assert_eq!(trend.trend[0].strong_buy, Some(6));
        assert_eq!(trend.trend[0].buy, Some(21));
        assert_eq!(trend.trend[0].hold, Some(15));
        assert_eq!(trend.trend[0].sell, Some(2));
        assert_eq!(trend.trend[0].strong_sell, Some(2));
        assert_eq!(trend.trend[1].period.as_deref(), Some("-1m"));
        assert_eq!(trend.trend[1].buy, Some(22));
        assert_eq!(trend.trend[1].hold, Some(14));
        assert_eq!(trend.trend[2].period.as_deref(), Some("-2m"));
        assert_eq!(trend.trend[2].hold, Some(16));
        assert_eq!(trend.trend[2].sell, Some(1));
        assert_eq!(trend.trend[3].period.as_deref(), Some("-3m"));
        assert_eq!(trend.trend[3].strong_buy, Some(7));
        assert_eq!(trend.trend[3].buy, Some(23));
    }

    #[test]
    fn test_full_earnings_trend() {
        let s = load_summary(AAPL_FIXTURE);
        let et = s.earnings_trend.unwrap();
        assert_eq!(et.max_age, Some(1));
        assert_eq!(et.trend.len(), 4);
        let item = &et.trend[0];
        assert_eq!(item.max_age, Some(1));
        assert_eq!(item.period.as_deref(), Some("0q"));
        assert_eq!(item.end_date.as_deref(), Some("2026-09-30"));
        assert_eq!(item.growth.as_ref().unwrap().raw, Some(0.0683));
        assert_eq!(item.growth.as_ref().unwrap().fmt.as_deref(), Some("6.83%"));
        assert!(item.growth_estimate.is_none());

        let ee = item.earnings_estimate.as_ref().unwrap();
        assert_eq!(ee.avg.as_ref().unwrap().raw, Some(1.97549));
        assert_eq!(ee.avg.as_ref().unwrap().fmt.as_deref(), Some("1.98"));
        assert_eq!(ee.low.as_ref().unwrap().raw, Some(1.93));
        assert_eq!(ee.high.as_ref().unwrap().raw, Some(2.04));
        assert_eq!(ee.year_ago_eps.as_ref().unwrap().raw, Some(1.85));
        assert_eq!(ee.number_of_analysts.as_ref().unwrap().raw, Some(28.0));
        assert_eq!(ee.number_of_analysts.as_ref().unwrap().long_fmt.as_deref(), Some("28"));
        assert_eq!(ee.growth.as_ref().unwrap().raw, Some(0.0678));
        assert_eq!(ee.earnings_currency.as_deref(), Some("USD"));

        let re = item.revenue_estimate.as_ref().unwrap();
        assert_eq!(re.avg.as_ref().unwrap().raw, Some(113256580210.0));
        assert_eq!(
            re.avg.as_ref().unwrap().long_fmt.as_deref(),
            Some("113,256,580,210")
        );
        assert_eq!(re.low.as_ref().unwrap().raw, Some(112137000000.0));
        assert_eq!(re.high.as_ref().unwrap().raw, Some(115068279000.0));
        assert_eq!(re.number_of_analysts.as_ref().unwrap().raw, Some(26.0));
        assert_eq!(re.year_ago_revenue.as_ref().unwrap().raw, Some(102466000000.0));
        assert_eq!(re.growth.as_ref().unwrap().raw, Some(0.105299994));
        assert_eq!(re.revenue_currency.as_deref(), Some("USD"));

        let eps_trend = item.eps_trend.as_ref().unwrap();
        assert_eq!(eps_trend.current.as_ref().unwrap().raw, Some(1.97549));
        assert_eq!(eps_trend.seven_days_ago.as_ref().unwrap().raw, Some(2.01204));
        assert_eq!(eps_trend.thirty_days_ago.as_ref().unwrap().raw, Some(2.00836));
        assert_eq!(eps_trend.sixty_days_ago.as_ref().unwrap().raw, Some(2.00767));
        assert_eq!(eps_trend.ninety_days_ago.as_ref().unwrap().raw, Some(2.00379));
        assert_eq!(eps_trend.eps_trend_currency.as_deref(), Some("USD"));

        let eps_rev = item.eps_revisions.as_ref().unwrap();
        assert_eq!(eps_rev.up_last_7days.as_ref().unwrap().raw, Some(1.0));
        assert_eq!(eps_rev.up_last_30days.as_ref().unwrap().raw, Some(4.0));
        assert_eq!(eps_rev.down_last_30days.as_ref().unwrap().raw, Some(2.0));
        assert_eq!(eps_rev.down_last7_days.as_ref().unwrap().raw, Some(1.0));
        assert!(eps_rev.down_last_90days.as_ref().unwrap().raw.is_none());
        assert_eq!(eps_rev.eps_revisions_currency.as_deref(), Some("USD"));
    }

    #[test]
    fn test_full_earnings_history() {
        let s = load_summary(AAPL_FIXTURE);
        let eh = s.earnings_history.unwrap();
        assert_eq!(eh.max_age, Some(86400));
        assert_eq!(eh.default_methodology.as_deref(), Some("gaap"));
        assert_eq!(eh.history.len(), 4);
        let item = &eh.history[0];
        assert_eq!(item.max_age, Some(1));
        assert_eq!(item.eps_actual.as_ref().unwrap().raw, Some(1.85));
        assert_eq!(item.eps_estimate.as_ref().unwrap().raw, Some(1.76993));
        assert_eq!(item.eps_difference.as_ref().unwrap().raw, Some(0.08));
        assert_eq!(item.surprise_percent.as_ref().unwrap().raw, Some(0.0452));
        assert_eq!(item.surprise_percent.as_ref().unwrap().fmt.as_deref(), Some("4.52%"));
        assert_eq!(item.quarter.as_ref().unwrap().raw, Some(1759190400.0));
        assert_eq!(item.quarter.as_ref().unwrap().fmt.as_deref(), Some("2025-09-30"));
        assert_eq!(item.currency.as_deref(), Some("USD"));
        assert_eq!(item.period.as_deref(), Some("-4q"));
    }

    #[test]
    fn test_full_earnings() {
        let s = load_summary(AAPL_FIXTURE);
        let e = s.earnings.unwrap();
        assert_eq!(e.max_age, Some(86400));
        assert_eq!(e.financial_currency.as_deref(), Some("USD"));

        let chart = e.earnings_chart.unwrap();
        assert_eq!(chart.quarterly.len(), 4);
        let q = &chart.quarterly[0];
        assert_eq!(q.date.as_deref(), Some("3Q2025"));
        assert_eq!(q.actual, Some(1.85));
        assert_eq!(q.estimate, Some(1.76993));
        assert_eq!(q.fiscal_quarter.as_deref(), Some("4Q2025"));
        assert_eq!(q.calendar_quarter.as_deref(), Some("3Q2025"));
        assert_eq!(q.difference.as_deref(), Some("0.08"));
        assert_eq!(q.surprise_pct.as_deref(), Some("4.52"));
        assert_eq!(q.period_end_date, Some(1759190400));
        assert_eq!(q.reported_date, Some(1761856200));
        assert_eq!(chart.current_quarter_estimate, Some(1.97549));
        assert_eq!(chart.current_quarter_estimate_date.as_deref(), Some("3Q"));
        assert_eq!(chart.current_calendar_quarter.as_deref(), Some("3Q2026"));
        assert_eq!(chart.current_quarter_estimate_year, Some(2026));
        assert_eq!(chart.current_fiscal_quarter.as_deref(), Some("4Q2026"));
        assert_eq!(chart.current_period_end_date, Some(1790726400));
        assert_eq!(chart.earnings_date.as_ref().unwrap(), &vec![1793304000]);
        assert_eq!(chart.is_earnings_date_estimate, Some(true));

        let fin = e.financials_chart.unwrap();
        assert_eq!(fin.yearly.len(), 4);
        assert_eq!(fin.quarterly.len(), 4);
        let y = &fin.yearly[0];
        assert_eq!(y.date, Some(2022));
        assert_eq!(y.revenue, Some(394328000000.0));
        assert_eq!(y.earnings, Some(99803000000.0));
        assert_eq!(y.profit_margin, Some(0.2530964));
        let fq = &fin.quarterly[0];
        assert_eq!(fq.date.as_deref(), Some("2Q2025"));
        assert_eq!(fq.fiscal_quarter.as_deref(), Some("3Q2025"));
        assert_eq!(fq.revenue, Some(94036000000.0));
        assert_eq!(fq.earnings, Some(23434000000.0));
        assert_eq!(fq.profit_margin, Some(0.24920243));
    }

    #[test]
    fn test_full_upgrade_downgrade_history() {
        let s = load_summary(AAPL_FIXTURE);
        let udh = s.upgrade_downgrade_history.unwrap();
        assert_eq!(udh.max_age, Some(86400));
        assert_eq!(udh.history.len(), 10);
        let item = &udh.history[0];
        assert_eq!(item.epoch_grade_date, Some(1786365485));
        assert_eq!(item.firm.as_deref(), Some("Jefferies"));
        assert_eq!(item.to_grade.as_deref(), Some("Underperform"));
        assert_eq!(item.from_grade.as_deref(), Some("Hold"));
        assert_eq!(item.action.as_deref(), Some("down"));
        assert_eq!(item.price_target_action.as_deref(), Some("Lowers"));
        assert_eq!(item.current_price_target, Some(263.66));
        assert_eq!(item.prior_price_target, Some(285.56));
    }

    #[test]
    fn test_full_calendar_events() {
        let s = load_summary(AAPL_FIXTURE);
        let ce = s.calendar_events.unwrap();
        assert_eq!(ce.max_age, Some(1));
        assert_eq!(ce.ex_dividend_date, Some(1786320000));
        assert_eq!(ce.dividend_date, Some(1786579200));
        let er = ce.earnings.unwrap();
        assert_eq!(er.earnings_date.as_ref().unwrap(), &vec![1793304000]);
        assert_eq!(er.earnings_call_date.as_ref().unwrap(), &vec![1785441600]);
        assert_eq!(er.is_earnings_date_estimate, Some(true));
        assert_eq!(er.earnings_average, Some(1.97643));
        assert_eq!(er.earnings_low, Some(1.93));
        assert_eq!(er.earnings_high, Some(2.04));
        assert_eq!(er.revenue_average, Some(113256580210.0));
        assert_eq!(er.revenue_low, Some(112137000000.0));
        assert_eq!(er.revenue_high, Some(115068279000.0));
    }

    #[test]
    fn test_full_insider_holders() {
        let s = load_summary(AAPL_FIXTURE);
        let ih = s.insider_holders.unwrap();
        assert_eq!(ih.max_age, Some(1));
        assert_eq!(ih.holders.len(), 5);
        let h = &ih.holders[0];
        assert_eq!(h.max_age, Some(1));
        assert_eq!(h.name.as_deref(), Some("ADAMS KATHERINE L"));
        assert_eq!(h.relation.as_deref(), Some("General Counsel"));
        assert_eq!(h.url.as_deref(), Some(""));
        assert_eq!(h.transaction_description.as_deref(), Some("Stock Gift"));
        assert_eq!(h.latest_trans_date.as_ref().unwrap().raw, Some(1762905600.0));
        assert_eq!(
            h.latest_trans_date.as_ref().unwrap().fmt.as_deref(),
            Some("2025-11-12")
        );
        assert_eq!(h.position_direct.as_ref().unwrap().raw, Some(175408.0));
        assert_eq!(
            h.position_direct.as_ref().unwrap().fmt.as_deref(),
            Some("175.41k")
        );
        assert_eq!(
            h.position_direct.as_ref().unwrap().long_fmt.as_deref(),
            Some("175,408")
        );
        assert_eq!(h.position_direct_date.as_ref().unwrap().raw, Some(1762905600.0));
        assert!(h.position_indirect.is_none());
        assert!(h.position_indirect_date.is_none());
        assert!(h.shares.is_none());
        assert!(h.value.is_none());
        let cook = &ih.holders[1];
        assert_eq!(cook.name.as_deref(), Some("COOK TIMOTHY D"));
        assert_eq!(cook.position_direct.as_ref().unwrap().raw, Some(3280420.0));
        assert_eq!(
            cook.position_direct.as_ref().unwrap().fmt.as_deref(),
            Some("3.28M")
        );
    }

    #[test]
    fn test_full_insider_transactions() {
        let s = load_summary(AAPL_FIXTURE);
        let it = s.insider_transactions.unwrap();
        assert_eq!(it.max_age, Some(1));
        assert_eq!(it.transactions.len(), 10);
        let t = &it.transactions[0];
        assert_eq!(t.max_age, Some(1));
        assert_eq!(t.shares.as_ref().unwrap().raw, Some(116.0));
        assert_eq!(t.shares.as_ref().unwrap().long_fmt.as_deref(), Some("116"));
        assert_eq!(t.value.as_ref().unwrap().raw, Some(34236.0));
        assert_eq!(t.value.as_ref().unwrap().fmt.as_deref(), Some("34.24k"));
        assert_eq!(t.filer_url.as_deref(), Some(""));
        assert_eq!(
            t.transaction_text.as_deref(),
            Some("Sale at price 295.14 per share.")
        );
        assert_eq!(t.filer_name.as_deref(), Some("BORDERS BEN"));
        assert_eq!(t.filer_relation.as_deref(), Some("Officer"));
        assert_eq!(t.money_text.as_deref(), Some(""));
        assert_eq!(t.start_date.as_ref().unwrap().raw, Some(1781568000.0));
        assert_eq!(t.ownership.as_deref(), Some("D"));
        let t2 = &it.transactions[1];
        assert!(t2.value.is_none());
        assert_eq!(t2.shares.as_ref().unwrap().raw, Some(30104.0));
        assert_eq!(t2.filer_name.as_deref(), Some("NEWSTEAD JENNIFER"));
        assert_eq!(t2.filer_relation.as_deref(), Some("General Counsel"));
    }

    #[test]
    fn test_full_major_holders_breakdown() {
        let s = load_summary(AAPL_FIXTURE);
        let mhb = s.major_holders_breakdown.unwrap();
        assert_eq!(mhb.max_age, Some(1));
        assert_eq!(mhb.insiders_percent_held, Some(0.01647));
        assert_eq!(mhb.institutions_percent_held, Some(0.66289));
        assert_eq!(mhb.institutions_float_percent_held, Some(0.67399));
        assert_eq!(mhb.institutions_count, Some(7670));
    }

    #[test]
    fn test_full_institution_ownership() {
        let s = load_summary(AAPL_FIXTURE);
        let io = s.institution_ownership.unwrap();
        assert_eq!(io.max_age, Some(1));
        assert_eq!(io.ownership_list.len(), 5);
        let item = &io.ownership_list[0];
        assert_eq!(item.max_age, Some(1));
        assert_eq!(item.report_date.as_ref().unwrap().raw, Some(1782777600.0));
        assert_eq!(item.report_date.as_ref().unwrap().fmt.as_deref(), Some("2026-06-30"));
        assert_eq!(item.organization.as_deref(), Some("Blackrock Inc."));
        assert_eq!(item.pct_held.as_ref().unwrap().raw, Some(0.0797));
        assert_eq!(item.pct_held.as_ref().unwrap().fmt.as_deref(), Some("7.97%"));
        assert_eq!(item.position.as_ref().unwrap().raw, Some(1162996939.0));
        assert_eq!(item.position.as_ref().unwrap().long_fmt.as_deref(), Some("1,162,996,939"));
        assert_eq!(item.value.as_ref().unwrap().raw, Some(356653944437.0));
        assert_eq!(item.value.as_ref().unwrap().fmt.as_deref(), Some("356.65B"));
        assert_eq!(item.pct_change.as_ref().unwrap().raw, Some(0.016));
        assert_eq!(item.pct_change.as_ref().unwrap().fmt.as_deref(), Some("1.60%"));
    }

    #[test]
    fn test_full_fund_ownership() {
        let s = load_summary(AAPL_FIXTURE);
        let fo = s.fund_ownership.unwrap();
        assert_eq!(fo.max_age, Some(1));
        assert_eq!(fo.ownership_list.len(), 5);
        let item = &fo.ownership_list[0];
        assert_eq!(item.max_age, Some(1));
        assert_eq!(item.report_date.as_ref().unwrap().raw, Some(1774915200.0));
        assert_eq!(
            item.organization.as_deref(),
            Some("VANGUARD INDEX FUNDS-Vanguard Total Stock Market Index Fund")
        );
        assert_eq!(item.pct_held.as_ref().unwrap().raw, Some(0.0319));
        assert_eq!(item.position.as_ref().unwrap().raw, Some(466211410.0));
        assert_eq!(item.position.as_ref().unwrap().fmt.as_deref(), Some("466.21M"));
        assert_eq!(item.value.as_ref().unwrap().raw, Some(142972120340.0));
        assert_eq!(item.value.as_ref().unwrap().fmt.as_deref(), Some("142.97B"));
        assert_eq!(item.pct_change.as_ref().unwrap().raw, Some(0.004));
        assert_eq!(item.pct_change.as_ref().unwrap().fmt.as_deref(), Some("0.40%"));
    }

    #[test]
    fn test_full_net_share_purchase_activity() {
        let s = load_summary(AAPL_FIXTURE);
        let nsp = s.net_share_purchase_activity.unwrap();
        assert_eq!(nsp.max_age, Some(1));
        assert_eq!(nsp.period.as_deref(), Some("6m"));
        assert_eq!(nsp.buy_info_count, Some(11));
        assert_eq!(nsp.buy_info_shares, Some(434520));
        assert_eq!(nsp.buy_percent_insider_shares, Some(0.002));
        assert_eq!(nsp.sell_info_count, Some(7));
        assert_eq!(nsp.sell_info_shares, Some(397875));
        assert_eq!(nsp.sell_percent_insider_shares, Some(0.002));
        assert_eq!(nsp.net_info_count, Some(18));
        assert_eq!(nsp.net_info_shares, Some(36645));
        assert_eq!(nsp.net_percent_insider_shares, Some(0.0));
        assert_eq!(nsp.net_inst_shares_buying, Some(62874610));
        assert_eq!(nsp.net_inst_buying_percent, Some(0.0064999997));
        assert_eq!(nsp.total_insider_shares, Some(240366144));
    }

    #[test]
    fn test_full_sec_filings() {
        let s = load_summary(AAPL_FIXTURE);
        let sf = s.sec_filings.unwrap();
        assert_eq!(sf.max_age, Some(86400));
        assert_eq!(sf.filings.len(), 10);
        let f = &sf.filings[0];
        assert_eq!(f.max_age, Some(1));
        assert_eq!(f.date.as_deref(), Some("2026-07-31"));
        assert_eq!(f.epoch_date, Some(1785456000));
        assert_eq!(f.filing_type.as_deref(), Some("10-Q"));
        assert_eq!(f.title.as_deref(), Some("Periodic Financial Reports"));
        assert_eq!(
            f.edgar_url.as_deref(),
            Some("https://finance.yahoo.com/sec-filing/AAPL/0000320193-26-000020_320193")
        );
        let exhibits = f.exhibits.as_ref().unwrap();
        assert_eq!(exhibits.len(), 4);
        assert_eq!(exhibits[0].exhibit_type.as_deref(), Some("EX-31.1"));
        assert_eq!(
            exhibits[0].url.as_deref(),
            Some("https://cdn.yahoofinance.com/prod/sec-filings/0000320193/000032019326000020/a10-qexhibit31106272026.htm")
        );
        assert_eq!(exhibits[1].exhibit_type.as_deref(), Some("EX-31.2"));
        assert_eq!(exhibits[3].exhibit_type.as_deref(), Some("10-Q"));
    }

    #[test]
    fn test_full_fund_profile() {
        let s = load_summary(VOO_FIXTURE);
        let fp = s.fund_profile.unwrap();
        assert_eq!(fp.max_age, Some(1));
        assert_eq!(
            fp.style_box_url.as_deref(),
            Some("https://s.yimg.com/lq/i/fi/3_0stylelargeeq2.gif")
        );
        assert_eq!(fp.family.as_deref(), Some("Vanguard"));
        assert_eq!(fp.category_name.as_deref(), Some("Large Blend"));
        assert_eq!(fp.legal_type.as_deref(), Some("Exchange Traded Fund"));
        let mi = fp.management_info.unwrap();
        assert!(mi.manager_name.is_none());
        assert!(mi.manager_bio.is_none());
        let fees = fp.fees_expenses_investment.unwrap();
        assert_eq!(fees.annual_report_expense_ratio, Some(0.00029999999));
        assert_eq!(fees.annual_holdings_turnover, Some(0.02));
        assert_eq!(fees.total_net_assets, Some(486952.38));
        assert!(fees.projection_values.unwrap().raw.is_none());
        let fees_cat = fp.fees_expenses_investment_cat.unwrap();
        assert_eq!(fees_cat.annual_report_expense_ratio, Some(0.0072176997));
        assert_eq!(fees_cat.annual_holdings_turnover, Some(0.9461));
        assert_eq!(fees_cat.total_net_assets, Some(486952.38));
        assert!(fees_cat.projection_values.is_none());
        assert_eq!(fp.brokerages.unwrap().len(), 0);
    }

    #[test]
    fn test_full_top_holdings() {
        let s = load_summary(VOO_FIXTURE);
        let th = s.top_holdings.unwrap();
        assert_eq!(th.max_age, Some(1));
        assert_eq!(th.cash_position, Some(0.0022));
        assert_eq!(th.stock_position, Some(0.9957));
        assert_eq!(th.bond_position, Some(0.0));
        assert_eq!(th.other_position, Some(0.002));
        assert_eq!(th.preferred_position, Some(0.0));
        assert_eq!(th.convertible_position, Some(0.0));
        assert_eq!(th.holdings.len(), 10);
        assert_eq!(th.holdings[0].symbol.as_deref(), Some("NVDA"));
        assert_eq!(th.holdings[0].holding_name.as_deref(), Some("NVIDIA Corp"));
        assert_eq!(th.holdings[0].holding_percent, Some(0.074991));
        assert_eq!(th.holdings[1].symbol.as_deref(), Some("AAPL"));
        assert_eq!(th.holdings[1].holding_percent, Some(0.065766804));
        let eq = th.equity_holdings.unwrap();
        assert_eq!(eq.price_to_earnings, Some(0.03716));
        assert_eq!(eq.price_to_book, Some(0.18538));
        assert_eq!(eq.price_to_sales, Some(0.269));
        assert_eq!(eq.price_to_cashflow, Some(0.05028));
        let bonds = th.bond_holdings.unwrap();
        assert!(bonds.price_to_earnings.is_none());
        assert!(bonds.price_to_book.is_none());
        assert!(bonds.price_to_sales.is_none());
        assert!(bonds.price_to_cashflow.is_none());
        assert_eq!(th.bond_ratings.len(), 1);
        assert_eq!(th.bond_ratings[0].get("us_government"), Some(&0.0));
        assert_eq!(th.sector_weightings.len(), 11);
        assert_eq!(th.sector_weightings[0].get("realestate"), Some(&0.0183));
        assert_eq!(th.sector_weightings[4].get("technology"), Some(&0.3861));
        assert_eq!(
            th.sector_weightings[10].get("healthcare"),
            Some(&0.088999994)
        );
    }
}
