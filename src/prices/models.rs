//! Request and response types for the historical prices domain.

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// Candlestick resolution supported by the IG prices API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Resolution {
    /// One-second bars.
    Second,
    /// One-minute bars.
    Minute,
    /// Two-minute bars.
    Minute2,
    /// Three-minute bars.
    Minute3,
    /// Five-minute bars.
    Minute5,
    /// Ten-minute bars.
    Minute10,
    /// Fifteen-minute bars.
    Minute15,
    /// Thirty-minute bars.
    Minute30,
    /// One-hour bars.
    Hour,
    /// Two-hour bars.
    Hour2,
    /// Three-hour bars.
    Hour3,
    /// Four-hour bars.
    Hour4,
    /// Daily bars.
    Day,
    /// Weekly bars.
    Week,
    /// Monthly bars.
    Month,
}

impl std::fmt::Display for Resolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Serialise to the IG wire string.
        let s = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(ToOwned::to_owned))
            .unwrap_or_else(|| format!("{self:?}").to_ascii_uppercase());
        f.write_str(&s)
    }
}

/// Request parameters for `GET /prices/{epic}` v3.
///
/// All fields are optional. Build from `Default` and set only what you need:
///
/// ```ignore
/// let req = HistoricalPricesRequest {
///     resolution: Some(Resolution::Hour),
///     from: Some(start),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct HistoricalPricesRequest {
    /// Bar resolution. Defaults to `MINUTE` server-side when not supplied.
    pub resolution: Option<Resolution>,
    /// Start of the date range (ISO format sent to the API).
    pub from: Option<NaiveDateTime>,
    /// End of the date range (ISO format sent to the API).
    pub to: Option<NaiveDateTime>,
    /// Maximum number of data points to return.
    pub max: Option<u32>,
    /// Number of points per page (default 20 server-side).
    pub page_size: Option<u32>,
    /// Page number to fetch (1-based). Leave `None` for page 1.
    pub page_number: Option<u32>,
}

// ── Response types ──────────────────────────────────────────────────────────

/// Bid/ask/last-traded triple for one side of a price candle.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceCandle {
    /// Bid price.
    pub bid: Option<f64>,
    /// Ask (offer) price.
    pub ask: Option<f64>,
    /// Last traded price (equity markets only).
    pub last_traded: Option<f64>,
}

/// One OHLC bar returned by the IG historical-prices API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PricePoint {
    /// Snapshot time in IG's v2 string format (`YYYY/MM/DD HH:MM:SS`).
    pub snapshot_time: String,
    /// Snapshot time in UTC ISO-8601 format.
    ///
    /// IG uses the non-standard capitalisation `snapshotTimeUTC` (capital UTC)
    /// in the JSON payload, so we rename explicitly rather than relying on
    /// the struct-level `camelCase` rename rule.
    #[serde(rename = "snapshotTimeUTC")]
    pub snapshot_time_utc: NaiveDateTime,
    /// Opening prices for this bar.
    pub open_price: PriceCandle,
    /// Closing prices for this bar.
    pub close_price: PriceCandle,
    /// High prices for this bar.
    pub high_price: PriceCandle,
    /// Low prices for this bar.
    pub low_price: PriceCandle,
    /// Last-traded volume (equity markets only).
    pub last_traded_volume: Option<u64>,
}

/// Pagination state embedded in every prices response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageData {
    /// Current page number (1-based).
    pub page_number: u32,
    /// Number of points per page.
    pub page_size: u32,
    /// Total number of available pages.
    pub total_pages: u32,
}

/// API allowance data returned with each prices response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceAllowance {
    /// Number of price data points remaining in the current allowance period.
    pub remaining_allowance: u32,
    /// Total price data points permitted per allowance period.
    pub total_allowance: u32,
    /// Seconds until the current allowance resets.
    pub allowance_expiry: u64,
}

/// Metadata block returned alongside every historical-prices response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PricesMetadata {
    /// Pagination state.
    pub page_data: PageData,
    /// Current API allowance.
    pub allowance: PriceAllowance,
}

/// Top-level historical-prices response (v2 and v3 share this shape).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalPrices {
    /// Instrument category (e.g. `"CURRENCIES"`, `"INDICES"`).
    pub instrument_type: String,
    /// Individual OHLC bars.
    pub prices: Vec<PricePoint>,
    /// Pagination and allowance metadata.
    pub metadata: PricesMetadata,
}

// ---------------------------------------------------------------------------
// Polars conversion
// ---------------------------------------------------------------------------

#[cfg(feature = "polars")]
impl crate::dataframe::IntoDataFrame for HistoricalPrices {
    /// Convert historical prices into a `polars::prelude::DataFrame` with one row per
    /// [`PricePoint`].
    ///
    /// Column layout:
    ///
    /// | column                   | dtype      | nullable |
    /// | ------------------------ | ---------- | -------- |
    /// | `snapshot_time`          | `Utf8`     | no       |
    /// | `snapshot_time_utc`      | `Datetime` | no       |
    /// | `open_bid`               | `Float64`  | yes      |
    /// | `open_ask`               | `Float64`  | yes      |
    /// | `high_bid`               | `Float64`  | yes      |
    /// | `high_ask`               | `Float64`  | yes      |
    /// | `low_bid`                | `Float64`  | yes      |
    /// | `low_ask`                | `Float64`  | yes      |
    /// | `close_bid`              | `Float64`  | yes      |
    /// | `close_ask`              | `Float64`  | yes      |
    /// | `last_traded_volume`     | `UInt64`   | yes      |
    fn to_dataframe(&self) -> crate::Result<polars::prelude::DataFrame> {
        use polars::prelude::*;

        let snapshot_time: Vec<&str> = self
            .prices
            .iter()
            .map(|p| p.snapshot_time.as_str())
            .collect();
        let snapshot_time_utc: Vec<NaiveDateTime> =
            self.prices.iter().map(|p| p.snapshot_time_utc).collect();
        let open_bid: Vec<Option<f64>> = self.prices.iter().map(|p| p.open_price.bid).collect();
        let open_ask: Vec<Option<f64>> = self.prices.iter().map(|p| p.open_price.ask).collect();
        let high_bid: Vec<Option<f64>> = self.prices.iter().map(|p| p.high_price.bid).collect();
        let high_ask: Vec<Option<f64>> = self.prices.iter().map(|p| p.high_price.ask).collect();
        let low_bid: Vec<Option<f64>> = self.prices.iter().map(|p| p.low_price.bid).collect();
        let low_ask: Vec<Option<f64>> = self.prices.iter().map(|p| p.low_price.ask).collect();
        let close_bid: Vec<Option<f64>> = self.prices.iter().map(|p| p.close_price.bid).collect();
        let close_ask: Vec<Option<f64>> = self.prices.iter().map(|p| p.close_price.ask).collect();
        let last_traded_volume: Vec<Option<u64>> =
            self.prices.iter().map(|p| p.last_traded_volume).collect();

        let snapshot_time_utc_series = Series::new("snapshot_time_utc".into(), snapshot_time_utc);

        DataFrame::new(vec![
            Column::new("snapshot_time".into(), snapshot_time),
            snapshot_time_utc_series.into(),
            Column::new("open_bid".into(), open_bid),
            Column::new("open_ask".into(), open_ask),
            Column::new("high_bid".into(), high_bid),
            Column::new("high_ask".into(), high_ask),
            Column::new("low_bid".into(), low_bid),
            Column::new("low_ask".into(), low_ask),
            Column::new("close_bid".into(), close_bid),
            Column::new("close_ask".into(), close_ask),
            Column::new("last_traded_volume".into(), last_traded_volume),
        ])
        .map_err(|e| crate::Error::Config(format!("polars conversion failed: {e}")))
    }
}
