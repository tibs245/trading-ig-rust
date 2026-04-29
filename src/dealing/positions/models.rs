//! Request and response types for the `dealing/positions` domain.

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

// DealConfirmation and DealStatus are re-exported from dealing::positions via dealing::common.
use crate::models::common::{
    DealId, DealReference, Direction, Epic, MarketSnapshot, OrderType, TimeInForce,
};

// MarketSnapshot is defined in models::common (unified across all domains).

// ---------------------------------------------------------------------------
// V1 position
// ---------------------------------------------------------------------------

/// Raw position sub-object as returned by the v1 list endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PositionSubV1 {
    contract_size: f64,
    controlled_risk: bool,
    created_date: String, // v1 format: "YYYY:MM:DD-HH:MM:SS"
    deal_id: DealId,
    deal_reference: DealReference,
    direction: Direction,
    level: f64,
    limit_level: Option<f64>,
    size: f64,
    stop_level: Option<f64>,
    trailing_step: Option<f64>,
    trailing_stop_distance: Option<f64>,
    currency: Option<String>,
    limited_risk_premium: Option<serde_json::Value>,
}

/// A position entry as returned by `GET /positions` v1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionV1 {
    pub deal_id: DealId,
    pub deal_reference: DealReference,
    pub direction: Direction,
    pub size: f64,
    pub level: f64,
    pub limit_level: Option<f64>,
    pub stop_level: Option<f64>,
    pub controlled_risk: bool,
    pub contract_size: f64,
    /// Created-date as an opaque string (v1 format `YYYY:MM:DD-HH:MM:SS`).
    pub created_date: String,
    pub trailing_step: Option<f64>,
    pub trailing_stop_distance: Option<f64>,
    pub market: MarketSnapshot,
}

/// Wire envelope for a single v1 list entry `{ "position": {…}, "market": {…} }`.
#[derive(Debug, Deserialize)]
struct PositionEntryV1 {
    position: PositionSubV1,
    market: MarketSnapshot,
}

impl From<PositionEntryV1> for PositionV1 {
    fn from(e: PositionEntryV1) -> Self {
        Self {
            deal_id: e.position.deal_id,
            deal_reference: e.position.deal_reference,
            direction: e.position.direction,
            size: e.position.size,
            level: e.position.level,
            limit_level: e.position.limit_level,
            stop_level: e.position.stop_level,
            controlled_risk: e.position.controlled_risk,
            contract_size: e.position.contract_size,
            created_date: e.position.created_date,
            trailing_step: e.position.trailing_step,
            trailing_stop_distance: e.position.trailing_stop_distance,
            market: e.market,
        }
    }
}

/// Wire envelope for `GET /positions` v1: `{ "positions": [ … ] }`.
#[derive(Debug, Deserialize)]
pub(super) struct PositionsEnvelopeV1 {
    positions: Vec<PositionEntryV1>,
}

impl PositionsEnvelopeV1 {
    pub(super) fn into_vec(self) -> Vec<PositionV1> {
        self.positions.into_iter().map(PositionV1::from).collect()
    }
}

// ---------------------------------------------------------------------------
// V2 position
// ---------------------------------------------------------------------------

/// Raw position sub-object as returned by the v2 list/get endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PositionSubV2 {
    contract_size: f64,
    controlled_risk: bool,
    created_date: String, // v2 format: "YYYY/MM/DD HH:MM:SS:mmm"
    #[serde(rename = "createdDateUTC")]
    created_date_utc: Option<String>,
    deal_id: DealId,
    deal_reference: DealReference,
    direction: Direction,
    level: f64,
    limit_level: Option<f64>,
    size: f64,
    stop_level: Option<f64>,
    trailing_step: Option<f64>,
    trailing_stop_distance: Option<f64>,
    currency: Option<String>,
    limited_risk_premium: Option<serde_json::Value>,
}

/// A position entry as returned by `GET /positions` v2 or `GET /positions/{dealId}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionV2 {
    pub deal_id: DealId,
    pub deal_reference: DealReference,
    pub direction: Direction,
    pub size: f64,
    pub level: f64,
    pub limit_level: Option<f64>,
    pub stop_level: Option<f64>,
    pub controlled_risk: bool,
    pub contract_size: f64,
    /// Created-date as an opaque string (v2 format `YYYY/MM/DD HH:MM:SS`).
    pub created_date: String,
    pub created_date_utc: Option<NaiveDateTime>,
    pub trailing_step: Option<f64>,
    pub trailing_stop_distance: Option<f64>,
    pub market: MarketSnapshot,
}

/// Wire envelope for a single v2 entry `{ "position": {…}, "market": {…} }`.
#[derive(Debug, Deserialize)]
struct PositionEntryV2 {
    position: PositionSubV2,
    market: MarketSnapshot,
}

fn parse_optional_utc(s: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f"))
        .ok()
}

impl From<PositionEntryV2> for PositionV2 {
    fn from(e: PositionEntryV2) -> Self {
        let created_date_utc = e
            .position
            .created_date_utc
            .as_deref()
            .and_then(parse_optional_utc);
        Self {
            deal_id: e.position.deal_id,
            deal_reference: e.position.deal_reference,
            direction: e.position.direction,
            size: e.position.size,
            level: e.position.level,
            limit_level: e.position.limit_level,
            stop_level: e.position.stop_level,
            controlled_risk: e.position.controlled_risk,
            contract_size: e.position.contract_size,
            created_date: e.position.created_date,
            created_date_utc,
            trailing_step: e.position.trailing_step,
            trailing_stop_distance: e.position.trailing_stop_distance,
            market: e.market,
        }
    }
}

/// Wire envelope for `GET /positions` v2: `{ "positions": [ … ] }`.
#[derive(Debug, Deserialize)]
pub(super) struct PositionsEnvelopeV2 {
    positions: Vec<PositionEntryV2>,
}

impl PositionsEnvelopeV2 {
    pub(super) fn into_vec(self) -> Vec<PositionV2> {
        self.positions.into_iter().map(PositionV2::from).collect()
    }
}

// DealStatus and DealConfirmation are defined in dealing::common.
// They are re-exported from positions::mod for backwards compatibility.

// ---------------------------------------------------------------------------
// Update position request
// ---------------------------------------------------------------------------

/// Request body for `PUT /positions/otc/{dealId}` (v2).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePositionRequest {
    /// Whether the stop is guaranteed (mandatory field).
    pub guaranteed_stop: bool,
    pub limit_level: Option<f64>,
    pub stop_level: Option<f64>,
    pub trailing_stop: Option<bool>,
    pub trailing_stop_distance: Option<f64>,
    pub trailing_stop_increment: Option<f64>,
}

impl UpdatePositionRequest {
    /// Convenience constructor with only the mandatory field.
    pub fn new(guaranteed_stop: bool) -> Self {
        Self {
            guaranteed_stop,
            limit_level: None,
            stop_level: None,
            trailing_stop: None,
            trailing_stop_distance: None,
            trailing_stop_increment: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Close position request
// ---------------------------------------------------------------------------

/// Request body for `DELETE /positions/otc` (v1).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClosePositionRequest {
    pub deal_id: Option<DealId>,
    pub direction: Direction,
    pub epic: Option<Epic>,
    pub expiry: Option<String>,
    pub level: Option<f64>,
    pub order_type: OrderType,
    pub quote_id: Option<String>,
    pub size: f64,
    pub time_in_force: Option<TimeInForce>,
}

// ---------------------------------------------------------------------------
// Internal: deal-reference-only response from POST /positions/otc
// ---------------------------------------------------------------------------

/// Wire response from `POST /positions/otc`: just a `dealReference`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OpenPositionResponse {
    pub deal_reference: DealReference,
}

/// Wire response from `PUT /positions/otc/{dealId}`: just a `dealReference`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct UpdatePositionResponse {
    pub deal_reference: DealReference,
}

/// Wire response from `DELETE /positions/otc`: just a `dealReference`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ClosePositionResponse {
    pub deal_reference: DealReference,
}

// ---------------------------------------------------------------------------
// Polars conversion
// ---------------------------------------------------------------------------

#[cfg(feature = "polars")]
impl crate::dataframe::IntoDataFrame for Vec<PositionV2> {
    /// Convert a list of v2 positions into a `polars::prelude::DataFrame`.
    ///
    /// Column layout:
    ///
    /// | column                   | dtype      | nullable |
    /// | ------------------------ | ---------- | -------- |
    /// | `deal_id`                | `Utf8`     | no       |
    /// | `deal_reference`         | `Utf8`     | no       |
    /// | `direction`              | `Utf8`     | no       |
    /// | `size`                   | `Float64`  | no       |
    /// | `level`                  | `Float64`  | no       |
    /// | `limit_level`            | `Float64`  | yes      |
    /// | `stop_level`             | `Float64`  | yes      |
    /// | `controlled_risk`        | `Boolean`  | no       |
    /// | `contract_size`          | `Float64`  | no       |
    /// | `created_date`           | `Utf8`     | no       |
    /// | `created_date_utc`       | `Datetime` | yes      |
    /// | `trailing_step`          | `Float64`  | yes      |
    /// | `trailing_stop_distance` | `Float64`  | yes      |
    /// | `market_epic`            | `Utf8`     | yes      |
    /// | `market_bid`             | `Float64`  | yes      |
    /// | `market_offer`           | `Float64`  | yes      |
    /// | `market_status`          | `Utf8`     | no       |
    fn to_dataframe(&self) -> crate::Result<polars::prelude::DataFrame> {
        use polars::prelude::*;

        let deal_id: Vec<&str> = self.iter().map(|p| p.deal_id.as_str()).collect();
        let deal_reference: Vec<&str> = self.iter().map(|p| p.deal_reference.as_str()).collect();
        let direction: Vec<&str> = self
            .iter()
            .map(|p| match p.direction {
                crate::models::common::Direction::Buy => "BUY",
                crate::models::common::Direction::Sell => "SELL",
            })
            .collect();
        let size: Vec<f64> = self.iter().map(|p| p.size).collect();
        let level: Vec<f64> = self.iter().map(|p| p.level).collect();
        let limit_level: Vec<Option<f64>> = self.iter().map(|p| p.limit_level).collect();
        let stop_level: Vec<Option<f64>> = self.iter().map(|p| p.stop_level).collect();
        let controlled_risk: Vec<bool> = self.iter().map(|p| p.controlled_risk).collect();
        let contract_size: Vec<f64> = self.iter().map(|p| p.contract_size).collect();
        let created_date: Vec<&str> = self.iter().map(|p| p.created_date.as_str()).collect();
        let created_date_utc: Vec<Option<NaiveDateTime>> =
            self.iter().map(|p| p.created_date_utc).collect();
        let trailing_step: Vec<Option<f64>> = self.iter().map(|p| p.trailing_step).collect();
        let trailing_stop_distance: Vec<Option<f64>> =
            self.iter().map(|p| p.trailing_stop_distance).collect();
        let market_epic: Vec<Option<&str>> = self
            .iter()
            .map(|p| {
                p.market
                    .epic
                    .as_ref()
                    .map(crate::models::common::Epic::as_str)
            })
            .collect();
        let market_bid: Vec<Option<f64>> = self.iter().map(|p| p.market.bid).collect();
        let market_offer: Vec<Option<f64>> = self.iter().map(|p| p.market.offer).collect();
        let market_status: Vec<&str> = self
            .iter()
            .map(|p| market_status_str(p.market.market_status))
            .collect();

        let created_date_utc_series = Series::new("created_date_utc".into(), created_date_utc);

        DataFrame::new(vec![
            Column::new("deal_id".into(), deal_id),
            Column::new("deal_reference".into(), deal_reference),
            Column::new("direction".into(), direction),
            Column::new("size".into(), size),
            Column::new("level".into(), level),
            Column::new("limit_level".into(), limit_level),
            Column::new("stop_level".into(), stop_level),
            Column::new("controlled_risk".into(), controlled_risk),
            Column::new("contract_size".into(), contract_size),
            Column::new("created_date".into(), created_date),
            created_date_utc_series.into(),
            Column::new("trailing_step".into(), trailing_step),
            Column::new("trailing_stop_distance".into(), trailing_stop_distance),
            Column::new("market_epic".into(), market_epic),
            Column::new("market_bid".into(), market_bid),
            Column::new("market_offer".into(), market_offer),
            Column::new("market_status".into(), market_status),
        ])
        .map_err(|e| crate::Error::Config(format!("polars conversion failed: {e}")))
    }
}

#[cfg(feature = "polars")]
fn market_status_str(s: crate::models::common::MarketStatus) -> &'static str {
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
