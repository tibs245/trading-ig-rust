//! Shared types used across both dealing sub-domains (positions and working orders).

use serde::{Deserialize, Serialize};

use crate::models::common::{DealId, DealReference, Direction, Epic, OrderType, TimeInForce};

/// Outcome status of a deal confirmation.
///
/// Used by both the positions and working-orders confirmation endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DealStatus {
    Accepted,
    Rejected,
    #[serde(other)]
    Unknown,
}

/// Full confirmation of a deal after open, update, close, or working-order
/// create/update/delete.
///
/// Returned by `GET /confirms/{dealReference}` (v1). This is a unified superset
/// of the fields returned by the positions and working-orders confirmation
/// endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DealConfirmation {
    pub deal_reference: DealReference,
    pub deal_id: Option<DealId>,
    pub deal_status: DealStatus,
    /// Populated when `deal_status` is `Rejected`.
    pub reason: Option<String>,
    pub direction: Option<Direction>,
    pub epic: Option<Epic>,
    /// Expiry code (positions only).
    pub expiry: Option<String>,
    pub level: Option<f64>,
    pub size: Option<f64>,
    pub order_type: Option<OrderType>,
    pub status: Option<String>,
    pub guaranteed_stop: Option<bool>,
    /// Whether a trailing stop is attached (positions only).
    pub trailing_stop: Option<bool>,
    /// Profit/loss realised on the deal (positions only).
    pub profit: Option<f64>,
    /// Currency of the profit figure (positions only).
    pub profit_currency: Option<String>,
    pub date: Option<String>,
    /// Affected deals list (positions only).
    pub affected_deals: Option<Vec<serde_json::Value>>,
    /// ISO currency code (working orders only).
    pub currency: Option<String>,
    /// Stop distance in pips (working orders only).
    pub stop_distance: Option<f64>,
    /// Stop level (working orders only).
    pub stop_level: Option<f64>,
    /// Limit distance in pips (working orders only).
    pub limit_distance: Option<f64>,
    /// Absolute limit level (working orders only).
    pub limit_level: Option<f64>,
    /// Good-till date string (working orders only).
    pub good_till_date: Option<String>,
    /// Good-till date in ISO-8601 (working orders only).
    pub good_till_date_iso: Option<String>,
    /// Time in force (working orders only).
    pub time_in_force: Option<TimeInForce>,
}
