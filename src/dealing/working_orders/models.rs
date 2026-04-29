//! Request and response types for the working-orders domain.

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::models::common::{
    Currency, DealId, Direction, Epic, MarketSnapshot, OrderType, TimeInForce,
};

// DealConfirmation and DealStatus are defined in dealing::common.
// MarketSnapshot is defined in models::common.

// ── v1 working-order types ────────────────────────────────────────────────────

/// A working order returned by `GET /workingorders` v1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingOrderV1 {
    /// Order data fields (v1 schema).
    #[serde(flatten)]
    pub order_data: WorkingOrderDataV1,
    /// Snapshot of the market this order is on.
    pub market: MarketSnapshot,
}

/// The `workingOrderData` sub-object in the v1 list response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkingOrderDataV1 {
    /// Date the order was created.
    pub created_date: Option<String>,
    /// ISO date the order was created.
    pub created_date_utc: Option<String>,
    /// Currency code.
    pub currency_code: Option<String>,
    /// Unique deal identifier.
    pub deal_id: DealId,
    /// Direction of the order.
    pub direction: Direction,
    /// Whether this is a DMA order.
    pub dma: Option<bool>,
    /// EPIC of the market.
    pub epic: Epic,
    /// Good-till value (v1 uses `goodTill` rather than `goodTillDate`).
    pub good_till: Option<String>,
    /// Whether the stop is guaranteed.
    pub controlled_risk: Option<bool>,
    /// Price level for the order.
    pub order_level: f64,
    /// Size of the order.
    pub order_size: f64,
    /// Type of the order (`LIMIT`, `STOP`, etc.).
    pub order_type: OrderType,
    /// Contingent limit price.
    pub contingent_limit: Option<f64>,
    /// Contingent stop price.
    pub contingent_stop: Option<f64>,
    /// Trailing stop distance.
    pub trailing_stop_distance: Option<f64>,
    /// Trailing stop increment.
    pub trailing_stop_increment: Option<f64>,
    /// Trailing trigger distance.
    pub trailing_trigger_distance: Option<f64>,
    /// Trailing trigger increment.
    pub trailing_trigger_increment: Option<f64>,
    /// Request type string.
    pub request_type: Option<String>,
    /// Time in force.
    pub time_in_force: Option<TimeInForce>,
    /// Limit distance.
    pub limit_distance: Option<f64>,
    /// Stop distance.
    pub stop_distance: Option<f64>,
}

// ── v2 working-order types ────────────────────────────────────────────────────

/// A working order returned by `GET /workingorders` v2 (canonical version).
///
/// The IG envelope wraps `workingOrderData` and `marketData` sub-objects;
/// this struct flattens them for ergonomics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingOrderV2 {
    /// Order data fields.
    #[serde(flatten)]
    pub order_data: WorkingOrderDataV2,
    /// Snapshot of the market.
    pub market: MarketSnapshot,
}

/// The `workingOrderData` sub-object in the v2 list response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkingOrderDataV2 {
    /// Date the order was created (v2 string format).
    pub created_date: Option<String>,
    /// ISO date the order was created.
    pub created_date_utc: Option<String>,
    /// Currency code (ISO-4217).
    pub currency_code: Option<Currency>,
    /// Unique deal identifier.
    pub deal_id: DealId,
    /// Direction of the order.
    pub direction: Direction,
    /// Whether this is a DMA order.
    pub dma: Option<bool>,
    /// EPIC of the market.
    pub epic: Epic,
    /// Good-till date string (v2).
    pub good_till_date: Option<String>,
    /// Good-till date in ISO-8601.
    pub good_till_date_iso: Option<String>,
    /// Whether a guaranteed stop is attached.
    pub guaranteed_stop: bool,
    /// Limit distance.
    pub limit_distance: Option<f64>,
    /// Limit level.
    pub limit_level: Option<f64>,
    /// Price level the order triggers at.
    pub order_level: f64,
    /// Size of the order.
    pub order_size: f64,
    /// Type of the order.
    pub order_type: OrderType,
    /// Stop distance.
    pub stop_distance: Option<f64>,
    /// Stop level.
    pub stop_level: Option<f64>,
    /// Time-in-force for the order.
    pub time_in_force: TimeInForce,
}

// ── Raw envelope types (private) ─────────────────────────────────────────────

/// Wire envelope for v1 list entries.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WorkingOrderEntryV1Raw {
    pub working_order_data: WorkingOrderDataV1,
    pub market_data: MarketSnapshot,
}

/// Wire envelope for v2 list entries.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WorkingOrderEntryV2Raw {
    pub working_order_data: WorkingOrderDataV2,
    pub market_data: MarketSnapshot,
}

// ── Update request ────────────────────────────────────────────────────────────

/// Request body for `PUT /workingorders/otc/{dealId}` (v2).
///
/// All fields are required by the IG API — there is no partial update.
/// Use `None` for optional stop/limit fields when they are not set.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWorkingOrderRequest {
    /// Good-till date; serialised as `YYYY/MM/DD HH:MM:SS` or `null`.
    #[serde(
        serialize_with = "serialize_good_till_date",
        deserialize_with = "deserialize_good_till_date_opt"
    )]
    pub good_till_date: Option<NaiveDateTime>,
    /// The order trigger level.
    pub level: f64,
    /// Limit distance in pips.
    pub limit_distance: Option<f64>,
    /// Absolute limit level.
    pub limit_level: Option<f64>,
    /// Stop distance in pips.
    pub stop_distance: Option<f64>,
    /// Absolute stop level.
    pub stop_level: Option<f64>,
    /// Whether the stop is guaranteed.
    pub guaranteed_stop: bool,
    /// Time in force.
    pub time_in_force: TimeInForce,
    /// Order type; wire key is `"type"` per the IG API spec.
    #[serde(rename = "type")]
    pub order_type: OrderType,
}

#[allow(clippy::ref_option)]
fn serialize_good_till_date<S>(dt: &Option<NaiveDateTime>, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match dt {
        Some(dt) => s.serialize_str(&crate::time::format(*dt, crate::time::ApiVersion::V2)),
        None => s.serialize_none(),
    }
}

fn deserialize_good_till_date_opt<'de, D>(d: D) -> Result<Option<NaiveDateTime>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(d)?;
    match opt {
        None => Ok(None),
        Some(s) => crate::time::parse(&s, crate::time::ApiVersion::V2)
            .map(Some)
            .map_err(serde::de::Error::custom),
    }
}
