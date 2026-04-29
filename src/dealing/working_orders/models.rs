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

// ---------------------------------------------------------------------------
// Polars conversion
// ---------------------------------------------------------------------------

#[cfg(feature = "polars")]
impl crate::dataframe::IntoDataFrame for Vec<WorkingOrderV2> {
    /// Convert a list of v2 working orders into a `polars::prelude::DataFrame`.
    ///
    /// Column layout:
    ///
    /// | column               | dtype     | nullable |
    /// | -------------------- | --------- | -------- |
    /// | `deal_id`            | `Utf8`    | no       |
    /// | `epic`               | `Utf8`    | no       |
    /// | `direction`          | `Utf8`    | no       |
    /// | `order_size`         | `Float64` | no       |
    /// | `order_level`        | `Float64` | no       |
    /// | `order_type`         | `Utf8`    | no       |
    /// | `time_in_force`      | `Utf8`    | no       |
    /// | `guaranteed_stop`    | `Boolean` | no       |
    /// | `currency_code`      | `Utf8`    | yes      |
    /// | `dma`                | `Boolean` | yes      |
    /// | `good_till_date`     | `Utf8`    | yes      |
    /// | `limit_distance`     | `Float64` | yes      |
    /// | `limit_level`        | `Float64` | yes      |
    /// | `stop_distance`      | `Float64` | yes      |
    /// | `stop_level`         | `Float64` | yes      |
    /// | `market_bid`         | `Float64` | yes      |
    /// | `market_offer`       | `Float64` | yes      |
    /// | `market_status`      | `Utf8`    | no       |
    fn to_dataframe(&self) -> crate::Result<polars::prelude::DataFrame> {
        use polars::prelude::*;

        let deal_id: Vec<&str> = self.iter().map(|w| w.order_data.deal_id.as_str()).collect();
        let epic: Vec<&str> = self.iter().map(|w| w.order_data.epic.as_str()).collect();
        let direction: Vec<&str> = self
            .iter()
            .map(|w| match w.order_data.direction {
                crate::models::common::Direction::Buy => "BUY",
                crate::models::common::Direction::Sell => "SELL",
            })
            .collect();
        let order_size: Vec<f64> = self.iter().map(|w| w.order_data.order_size).collect();
        let order_level: Vec<f64> = self.iter().map(|w| w.order_data.order_level).collect();
        let order_type: Vec<&str> = self
            .iter()
            .map(|w| order_type_str(w.order_data.order_type))
            .collect();
        let time_in_force: Vec<&str> = self
            .iter()
            .map(|w| time_in_force_str(w.order_data.time_in_force))
            .collect();
        let guaranteed_stop: Vec<bool> =
            self.iter().map(|w| w.order_data.guaranteed_stop).collect();
        let currency_code: Vec<Option<&str>> = self
            .iter()
            .map(|w| w.order_data.currency_code.as_ref().map(Currency::as_str))
            .collect();
        let dma: Vec<Option<bool>> = self.iter().map(|w| w.order_data.dma).collect();
        let good_till_date: Vec<Option<&str>> = self
            .iter()
            .map(|w| w.order_data.good_till_date.as_deref())
            .collect();
        let limit_distance: Vec<Option<f64>> =
            self.iter().map(|w| w.order_data.limit_distance).collect();
        let limit_level: Vec<Option<f64>> = self.iter().map(|w| w.order_data.limit_level).collect();
        let stop_distance: Vec<Option<f64>> =
            self.iter().map(|w| w.order_data.stop_distance).collect();
        let stop_level: Vec<Option<f64>> = self.iter().map(|w| w.order_data.stop_level).collect();
        let market_bid: Vec<Option<f64>> = self.iter().map(|w| w.market.bid).collect();
        let market_offer: Vec<Option<f64>> = self.iter().map(|w| w.market.offer).collect();
        let market_status: Vec<&str> = self
            .iter()
            .map(|w| market_status_str(w.market.market_status))
            .collect();

        DataFrame::new(vec![
            Column::new("deal_id".into(), deal_id),
            Column::new("epic".into(), epic),
            Column::new("direction".into(), direction),
            Column::new("order_size".into(), order_size),
            Column::new("order_level".into(), order_level),
            Column::new("order_type".into(), order_type),
            Column::new("time_in_force".into(), time_in_force),
            Column::new("guaranteed_stop".into(), guaranteed_stop),
            Column::new("currency_code".into(), currency_code),
            Column::new("dma".into(), dma),
            Column::new("good_till_date".into(), good_till_date),
            Column::new("limit_distance".into(), limit_distance),
            Column::new("limit_level".into(), limit_level),
            Column::new("stop_distance".into(), stop_distance),
            Column::new("stop_level".into(), stop_level),
            Column::new("market_bid".into(), market_bid),
            Column::new("market_offer".into(), market_offer),
            Column::new("market_status".into(), market_status),
        ])
        .map_err(|e| crate::Error::Config(format!("polars conversion failed: {e}")))
    }
}

#[cfg(feature = "polars")]
fn order_type_str(t: crate::models::common::OrderType) -> &'static str {
    use crate::models::common::OrderType;
    match t {
        OrderType::Limit => "LIMIT",
        OrderType::Market => "MARKET",
        OrderType::Quote => "QUOTE",
        OrderType::Stop => "STOP",
    }
}

#[cfg(feature = "polars")]
fn time_in_force_str(t: crate::models::common::TimeInForce) -> &'static str {
    use crate::models::common::TimeInForce;
    match t {
        TimeInForce::GoodTillCancelled => "GOOD_TILL_CANCELLED",
        TimeInForce::GoodTillDate => "GOOD_TILL_DATE",
        TimeInForce::ExecuteAndEliminate => "EXECUTE_AND_ELIMINATE",
        TimeInForce::FillOrKill => "FILL_OR_KILL",
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
