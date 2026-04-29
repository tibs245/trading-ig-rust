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
