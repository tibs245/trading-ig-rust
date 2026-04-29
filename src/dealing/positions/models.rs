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
