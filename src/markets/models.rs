//! Request and response types for the markets domain.

use serde::{Deserialize, Deserializer, Serialize};

use crate::models::common::Epic;

// ---------------------------------------------------------------------------
// Enumerations (InstrumentType and MarketStatus are in models::common)
// ---------------------------------------------------------------------------

// Re-export for ergonomic use within this module.
pub use crate::models::common::{InstrumentType, MarketStatus};

/// Filter for the bulk-fetch (`GET /markets?epics=…`) endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MarketDetailFilter {
    /// Return full instrument + dealing rules + snapshot detail.
    All,
    /// Return only the snapshot (pricing) data.
    SnapshotOnly,
}

impl MarketDetailFilter {
    /// Returns the string expected by the `filter` query parameter.
    pub fn as_query_str(self) -> &'static str {
        match self {
            Self::All => "ALL",
            Self::SnapshotOnly => "SNAPSHOT_ONLY",
        }
    }
}

// ---------------------------------------------------------------------------
// Lightweight market summary (search / navigation)
// ---------------------------------------------------------------------------

/// Lightweight market entry returned by search and navigation endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketSummary {
    /// IG market identifier.
    pub epic: Epic,
    /// Human-readable instrument name.
    pub instrument_name: String,
    /// Instrument classification.
    pub instrument_type: InstrumentType,
    /// Expiry code (`"DFB"`, a date string, or `"-"`).
    pub expiry: String,
    /// Current bid price (absent when market is closed).
    pub bid: Option<f64>,
    /// Current offer / ask price (absent when market is closed).
    pub offer: Option<f64>,
    /// Current trading status.
    pub market_status: MarketStatus,
    /// Whether live streaming prices are available.
    pub streaming_prices_available: bool,
    /// High price for the day.
    pub high: Option<f64>,
    /// Low price for the day.
    pub low: Option<f64>,
    /// Net price change since market open.
    pub net_change: Option<f64>,
    /// Percentage price change since market open.
    pub percentage_change: Option<f64>,
    /// Price update timestamp (local exchange time).
    pub update_time: Option<String>,
    /// Price update timestamp (UTC).
    pub update_time_utc: Option<String>,
    /// Delay time in minutes (0 for real-time).
    pub delay_time: Option<i32>,
    /// Scaling factor applied to prices.
    pub scaling_factor: Option<i32>,
}

// ---------------------------------------------------------------------------
// Full market details (single + bulk endpoints)
// ---------------------------------------------------------------------------

/// Unit / denomination for a dealing rule or instrument measure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DealingRuleValue {
    /// Measurement unit (e.g. `"POINTS"`, `"PERCENTAGE"`, `"AMOUNT"`).
    pub unit: String,
    /// The numeric value.
    pub value: f64,
}

/// Currency information for an instrument.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstrumentCurrency {
    /// ISO-4217 currency code.
    pub code: String,
    /// Human-readable currency name.
    pub name: String,
    /// Currency symbol.
    pub symbol: String,
    /// Whether this is the default currency for the instrument.
    pub is_default: bool,
}

/// Full instrument details for a market.
///
/// This is the `instrument` sub-object of `MarketDetails`.  Not flattened
/// because it contains ≥10 conceptually distinct fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instrument {
    /// IG market identifier.
    pub epic: Epic,
    /// Human-readable instrument name.
    ///
    /// Optional: the gold CFE `MarketDetails` payload omits `name` entirely;
    /// a non-`Option` field rejected it with "missing field name".
    #[serde(default)]
    pub name: Option<String>,
    /// Current trading status.
    pub status: Option<MarketStatus>,
    /// Instrument type classification.
    #[serde(rename = "type")]
    pub instrument_type: InstrumentType,
    /// Expiry code.
    pub expiry: String,
    /// Contract lot size.
    pub lot_size: Option<f64>,
    /// Quantity unit (e.g. `"AMOUNT"`, `"CONTRACTS"`, `"SHARES"`).
    pub unit: Option<String>,
    /// Daily high price.
    pub high: Option<f64>,
    /// Daily low price.
    pub low: Option<f64>,
    /// Percentage price change since open.
    pub percentage_change: Option<f64>,
    /// Net price change since open.
    pub net_change: Option<f64>,
    /// Current bid price.
    pub bid: Option<f64>,
    /// Current offer price.
    pub offer: Option<f64>,
    /// Price update timestamp (local).
    pub update_time: Option<String>,
    /// Price update timestamp (UTC).
    pub update_time_utc: Option<String>,
    /// Price delay in minutes.
    pub delay_time: Option<i32>,
    /// Whether streaming prices are available.
    pub streaming_prices_available: bool,
    /// Current market status.
    pub market_status: Option<MarketStatus>,
    /// Scaling factor for prices.
    pub scaling_factor: Option<i32>,
    /// Currencies in which this instrument can be denominated.
    #[serde(default)]
    pub currencies: Vec<InstrumentCurrency>,
    /// Margin factor (the numeric value).
    pub margin_factor: Option<f64>,
    /// Unit for `margin_factor` (e.g. `"PERCENTAGE"`).
    pub margin_factor_unit: Option<String>,
    /// Slippage factor for this instrument.
    pub slippage_factor: Option<DealingRuleValue>,
    /// Additional premium charged for controlled-risk orders.
    pub limited_risk_premium: Option<DealingRuleValue>,
    /// Special information strings (e.g. ex-dividend notices).
    #[serde(default)]
    pub special_info: Vec<String>,
}

/// Market-order and trailing-stop preference strings as returned by IG.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DealingRules {
    /// Minimum distance between the current price and a step order.
    pub min_step_distance: DealingRuleValue,
    /// Minimum deal size.
    pub min_deal_size: DealingRuleValue,
    /// Minimum stop distance for controlled-risk orders.
    pub min_controlled_risk_stop_distance: DealingRuleValue,
    /// Minimum stop or limit distance for normal orders.
    pub min_normal_stop_or_limit_distance: DealingRuleValue,
    /// Maximum stop or limit distance.
    pub max_stop_or_limit_distance: DealingRuleValue,
    /// Minimum gap between a controlled-risk stop and the bid/offer.
    pub controlled_risk_spacing: DealingRuleValue,
    /// Whether market orders are permitted on this instrument.
    pub market_order_preference: String,
    /// Whether trailing stops are permitted.
    pub trailing_stops_preference: String,
}

// MarketSnapshot is now defined in models::common and re-exported here.
pub use crate::models::common::MarketSnapshot;

/// Full market details — the canonical response from `GET /markets/{epic}` (v3)
/// and each entry in `GET /markets?epics=…` (v2).
///
/// Sub-objects (`instrument`, `dealing_rules`, `snapshot`) are kept intact
/// because each has ≥10 conceptually distinct fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketDetails {
    /// Detailed instrument information.
    pub instrument: Instrument,
    /// Dealing rules and constraints.
    pub dealing_rules: DealingRules,
    /// Live price snapshot.
    pub snapshot: MarketSnapshot,
}

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

/// A child node in the market navigation hierarchy (has an `id` and `name`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigationChild {
    /// IG node identifier.
    pub id: String,
    /// Human-readable node name.
    pub name: String,
}

/// A node in the IG market navigation hierarchy.
///
/// Top-level nodes (from `GET /marketnavigation`) typically have only `nodes`.
/// Leaf nodes (from `GET /marketnavigation/{id}`) typically have only
/// `markets`.  Both fields default to empty `Vec` when absent or `null`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigationNode {
    /// Markets available at this node level.
    #[serde(default, deserialize_with = "null_as_empty_vec")]
    pub markets: Vec<MarketSummary>,
    /// Child navigation nodes.
    #[serde(default, deserialize_with = "null_as_empty_vec")]
    pub nodes: Vec<NavigationChild>,
}

/// Deserialise `null` as an empty `Vec`, in addition to the normal `[]`.
fn null_as_empty_vec<'de, T, D>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Ok(Option::<Vec<T>>::deserialize(deserializer)?.unwrap_or_default())
}

// ---------------------------------------------------------------------------
// Polars conversion
// ---------------------------------------------------------------------------

#[cfg(feature = "polars")]
impl crate::dataframe::IntoDataFrame for Vec<MarketSummary> {
    /// Convert a list of market summaries into a `polars::prelude::DataFrame`.
    ///
    /// Used by both the `markets` (search / navigation) and `watchlists`
    /// domains, which both return `Vec<MarketSummary>`.
    ///
    /// Column layout:
    ///
    /// | column                        | dtype     | nullable |
    /// | ----------------------------- | --------- | -------- |
    /// | `epic`                        | `Utf8`    | no       |
    /// | `instrument_name`             | `Utf8`    | no       |
    /// | `instrument_type`             | `Utf8`    | no       |
    /// | `expiry`                      | `Utf8`    | no       |
    /// | `bid`                         | `Float64` | yes      |
    /// | `offer`                       | `Float64` | yes      |
    /// | `market_status`               | `Utf8`    | no       |
    /// | `streaming_prices_available`  | `Boolean` | no       |
    /// | `high`                        | `Float64` | yes      |
    /// | `low`                         | `Float64` | yes      |
    /// | `net_change`                  | `Float64` | yes      |
    /// | `percentage_change`           | `Float64` | yes      |
    /// | `update_time`                 | `Utf8`    | yes      |
    /// | `update_time_utc`             | `Utf8`    | yes      |
    fn to_dataframe(&self) -> crate::Result<polars::prelude::DataFrame> {
        use polars::prelude::*;

        let epic: Vec<&str> = self.iter().map(|m| m.epic.as_str()).collect();
        let instrument_name: Vec<&str> = self.iter().map(|m| m.instrument_name.as_str()).collect();
        let instrument_type: Vec<&str> = self
            .iter()
            .map(|m| instrument_type_str(m.instrument_type))
            .collect();
        let expiry: Vec<&str> = self.iter().map(|m| m.expiry.as_str()).collect();
        let bid: Vec<Option<f64>> = self.iter().map(|m| m.bid).collect();
        let offer: Vec<Option<f64>> = self.iter().map(|m| m.offer).collect();
        let market_status: Vec<&str> = self
            .iter()
            .map(|m| market_status_str(m.market_status))
            .collect();
        let streaming_prices_available: Vec<bool> =
            self.iter().map(|m| m.streaming_prices_available).collect();
        let high: Vec<Option<f64>> = self.iter().map(|m| m.high).collect();
        let low: Vec<Option<f64>> = self.iter().map(|m| m.low).collect();
        let net_change: Vec<Option<f64>> = self.iter().map(|m| m.net_change).collect();
        let percentage_change: Vec<Option<f64>> =
            self.iter().map(|m| m.percentage_change).collect();
        let update_time: Vec<Option<&str>> =
            self.iter().map(|m| m.update_time.as_deref()).collect();
        let update_time_utc: Vec<Option<&str>> =
            self.iter().map(|m| m.update_time_utc.as_deref()).collect();

        DataFrame::new(vec![
            Column::new("epic".into(), epic),
            Column::new("instrument_name".into(), instrument_name),
            Column::new("instrument_type".into(), instrument_type),
            Column::new("expiry".into(), expiry),
            Column::new("bid".into(), bid),
            Column::new("offer".into(), offer),
            Column::new("market_status".into(), market_status),
            Column::new(
                "streaming_prices_available".into(),
                streaming_prices_available,
            ),
            Column::new("high".into(), high),
            Column::new("low".into(), low),
            Column::new("net_change".into(), net_change),
            Column::new("percentage_change".into(), percentage_change),
            Column::new("update_time".into(), update_time),
            Column::new("update_time_utc".into(), update_time_utc),
        ])
        .map_err(|e| crate::Error::Config(format!("polars conversion failed: {e}")))
    }
}

#[cfg(feature = "polars")]
fn instrument_type_str(t: InstrumentType) -> &'static str {
    use crate::models::common::InstrumentType;
    match t {
        InstrumentType::Currencies => "CURRENCIES",
        InstrumentType::Shares => "SHARES",
        InstrumentType::Indices => "INDICES",
        InstrumentType::Commodities => "COMMODITIES",
        InstrumentType::Options => "OPTIONS",
        InstrumentType::Bonds => "BONDS",
        InstrumentType::Rates => "RATES",
        InstrumentType::Sectors => "SECTORS",
        InstrumentType::Funds => "FUNDS",
        InstrumentType::SprintMarkets => "SPRINT_MARKETS",
        InstrumentType::Unknown => "UNKNOWN",
    }
}

#[cfg(feature = "polars")]
fn market_status_str(s: MarketStatus) -> &'static str {
    use crate::models::common::MarketStatus;
    match s {
        MarketStatus::Tradeable => "TRADEABLE",
        MarketStatus::EditsOnly => "EDITS_ONLY",
        MarketStatus::Closed => "CLOSED",
        MarketStatus::Offline => "OFFLINE",
        MarketStatus::OnAuction => "ON_AUCTION",
        MarketStatus::OnAuctionNoEdits => "ON_AUCTION_NO_EDITS",
        MarketStatus::Suspended => "SUSPENDED",
        MarketStatus::Unknown => "UNKNOWN",
    }
}
