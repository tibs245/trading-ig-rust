//! Typed streaming update events for each subscription kind.
//!
//! These structs are what callers receive from the
//! `tokio::sync::mpsc::Receiver<T>` channels returned by the subscription
//! helpers on [`crate::streaming::StreamingClient`].
//!
//! Fields are `Option<f64>` / `Option<String>` etc. because:
//! - Lightstreamer may send `#` (null) for any field at any time.
//! - The "unchanged" sentinel is resolved before the event is emitted, so by
//!   the time a caller sees an event every field either has a value or is
//!   `None`.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// MARKET:<epic>  — MERGE mode
// ---------------------------------------------------------------------------

/// A single update from a `MARKET:<epic>` subscription.
///
/// Fields mirror the IG Lightstreamer `MARKET` adapter fields.
/// `None` means the server sent `null` (or the field has never been populated).
#[derive(Debug, Clone, Default)]
pub struct MarketUpdate {
    /// The IG epic this update belongs to.
    pub epic: String,
    /// Best bid price.
    pub bid: Option<f64>,
    /// Best offer (ask) price.
    pub offer: Option<f64>,
    /// Today's high price.
    pub high: Option<f64>,
    /// Today's low price.
    pub low: Option<f64>,
    /// Mid price at open.
    pub mid_open: Option<f64>,
    /// Net change vs. previous close.
    pub change: Option<f64>,
    /// Percentage change vs. previous close.
    pub change_pct: Option<f64>,
    /// Server-side update timestamp (HH:MM:SS string).
    pub update_time: Option<String>,
    /// Whether price quotes are delayed (`true`) or live (`false`).
    pub market_delay: Option<bool>,
    /// Market state string (e.g. `"TRADEABLE"`, `"CLOSED"`).
    pub market_state: Option<String>,
}

/// Field indices for `MARKET:<epic>`.
pub(crate) const MARKET_FIELDS: &[&str] = &[
    "BID",
    "OFFER",
    "HIGH",
    "LOW",
    "MID_OPEN",
    "CHANGE",
    "CHANGE_PCT",
    "UPDATE_TIME",
    "MARKET_DELAY",
    "MARKET_STATE",
];

impl MarketUpdate {
    /// Construct from a raw field-value slice (in `MARKET_FIELDS` order).
    pub fn from_raw(epic: &str, state: &[Option<String>]) -> Self {
        let get = |i: usize| state.get(i).and_then(|v| v.as_deref());
        Self {
            epic: epic.to_owned(),
            bid: get(0).and_then(|s| s.parse().ok()),
            offer: get(1).and_then(|s| s.parse().ok()),
            high: get(2).and_then(|s| s.parse().ok()),
            low: get(3).and_then(|s| s.parse().ok()),
            mid_open: get(4).and_then(|s| s.parse().ok()),
            change: get(5).and_then(|s| s.parse().ok()),
            change_pct: get(6).and_then(|s| s.parse().ok()),
            update_time: get(7).map(str::to_owned),
            market_delay: get(8).and_then(|s| match s {
                "0" | "false" => Some(false),
                "1" | "true" => Some(true),
                _ => None,
            }),
            market_state: get(9).map(str::to_owned),
        }
    }
}

// ---------------------------------------------------------------------------
// CHART:<epic>:TICK  — DISTINCT mode
// ---------------------------------------------------------------------------

/// A single tick from a `CHART:<epic>:TICK` subscription.
#[derive(Debug, Clone, Default)]
pub struct ChartTickUpdate {
    /// The IG epic this update belongs to.
    pub epic: String,
    /// Bid price for the tick.
    pub bid: Option<f64>,
    /// Offer price for the tick.
    pub ofr: Option<f64>,
    /// Last traded price.
    pub ltp: Option<f64>,
    /// Last traded volume.
    pub ltv: Option<f64>,
    /// Total traded volume today.
    pub ttv: Option<f64>,
    /// UTC millisecond timestamp of the tick.
    pub utm: Option<i64>,
    /// Mid price at open today.
    pub day_open_mid: Option<f64>,
    /// Net change mid today.
    pub day_net_chg_mid: Option<f64>,
    /// Percentage change mid today.
    pub day_perc_chg_mid: Option<f64>,
    /// Today's high.
    pub day_high: Option<f64>,
    /// Today's low.
    pub day_low: Option<f64>,
}

/// Field indices for `CHART:<epic>:TICK`.
pub(crate) const CHART_TICK_FIELDS: &[&str] = &[
    "BID",
    "OFR",
    "LTP",
    "LTV",
    "TTV",
    "UTM",
    "DAY_OPEN_MID",
    "DAY_NET_CHG_MID",
    "DAY_PERC_CHG_MID",
    "DAY_HIGH",
    "DAY_LOW",
];

impl ChartTickUpdate {
    pub fn from_raw(epic: &str, state: &[Option<String>]) -> Self {
        let get = |i: usize| state.get(i).and_then(|v| v.as_deref());
        Self {
            epic: epic.to_owned(),
            bid: get(0).and_then(|s| s.parse().ok()),
            ofr: get(1).and_then(|s| s.parse().ok()),
            ltp: get(2).and_then(|s| s.parse().ok()),
            ltv: get(3).and_then(|s| s.parse().ok()),
            ttv: get(4).and_then(|s| s.parse().ok()),
            utm: get(5).and_then(|s| s.parse().ok()),
            day_open_mid: get(6).and_then(|s| s.parse().ok()),
            day_net_chg_mid: get(7).and_then(|s| s.parse().ok()),
            day_perc_chg_mid: get(8).and_then(|s| s.parse().ok()),
            day_high: get(9).and_then(|s| s.parse().ok()),
            day_low: get(10).and_then(|s| s.parse().ok()),
        }
    }
}

// ---------------------------------------------------------------------------
// CHART:<epic>:<scale>  — MERGE mode
// ---------------------------------------------------------------------------

/// Candle scale for `CHART:<epic>:<scale>` subscriptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandleScale {
    /// One-minute candle.
    OneMinute,
    /// Five-minute candle.
    FiveMinute,
    /// One-hour candle.
    Hour,
}

impl CandleScale {
    /// Return the wire-level scale string used in the Lightstreamer item name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OneMinute => "1MINUTE",
            Self::FiveMinute => "5MINUTE",
            Self::Hour => "HOUR",
        }
    }
}

impl std::fmt::Display for CandleScale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A candle update from a `CHART:<epic>:<scale>` subscription.
#[derive(Debug, Clone, Default)]
pub struct ChartCandleUpdate {
    /// The IG epic this update belongs to.
    pub epic: String,
    /// Candle scale.
    pub scale: Option<CandleScale>,
    /// Offer open price.
    pub ofr_open: Option<f64>,
    /// Offer high price.
    pub ofr_high: Option<f64>,
    /// Offer low price.
    pub ofr_low: Option<f64>,
    /// Offer close price.
    pub ofr_close: Option<f64>,
    /// Bid open price.
    pub bid_open: Option<f64>,
    /// Bid high price.
    pub bid_high: Option<f64>,
    /// Bid low price.
    pub bid_low: Option<f64>,
    /// Bid close price.
    pub bid_close: Option<f64>,
    /// Last-traded-price open.
    pub ltp_open: Option<f64>,
    /// Last-traded-price high.
    pub ltp_high: Option<f64>,
    /// Last-traded-price low.
    pub ltp_low: Option<f64>,
    /// Last-traded-price close.
    pub ltp_close: Option<f64>,
    /// Whether the candle is complete (`1`) or still forming (`0`).
    pub cons_end: Option<bool>,
    /// Number of ticks in this candle.
    pub cons_tick_count: Option<i64>,
    /// UTC millisecond timestamp.
    pub utm: Option<i64>,
}

/// Field indices for `CHART:<epic>:<scale>`.
pub(crate) const CHART_CANDLE_FIELDS: &[&str] = &[
    "OFR_OPEN",
    "OFR_HIGH",
    "OFR_LOW",
    "OFR_CLOSE",
    "BID_OPEN",
    "BID_HIGH",
    "BID_LOW",
    "BID_CLOSE",
    "LTP_OPEN",
    "LTP_HIGH",
    "LTP_LOW",
    "LTP_CLOSE",
    "CONS_END",
    "CONS_TICK_COUNT",
    "UTM",
];

impl ChartCandleUpdate {
    pub fn from_raw(epic: &str, scale: CandleScale, state: &[Option<String>]) -> Self {
        let get = |i: usize| state.get(i).and_then(|v| v.as_deref());
        let pf = |i: usize| get(i).and_then(|s| s.parse::<f64>().ok());
        let pi = |i: usize| get(i).and_then(|s| s.parse::<i64>().ok());
        Self {
            epic: epic.to_owned(),
            scale: Some(scale),
            ofr_open: pf(0),
            ofr_high: pf(1),
            ofr_low: pf(2),
            ofr_close: pf(3),
            bid_open: pf(4),
            bid_high: pf(5),
            bid_low: pf(6),
            bid_close: pf(7),
            ltp_open: pf(8),
            ltp_high: pf(9),
            ltp_low: pf(10),
            ltp_close: pf(11),
            cons_end: get(12).and_then(|s| match s {
                "1" => Some(true),
                "0" => Some(false),
                _ => None,
            }),
            cons_tick_count: pi(13),
            utm: pi(14),
        }
    }
}

// ---------------------------------------------------------------------------
// ACCOUNT:<accountId>  — MERGE mode
// ---------------------------------------------------------------------------

/// An update from an `ACCOUNT:<accountId>` subscription.
#[derive(Debug, Clone, Default)]
pub struct AccountUpdate {
    /// The account ID this update belongs to.
    pub account_id: String,
    /// Profit and loss (unrealised).
    pub pnl: Option<f64>,
    /// Total deposit.
    pub deposit: Option<f64>,
    /// Available cash.
    pub available_cash: Option<f64>,
    /// Funds (equity - margin).
    pub funds: Option<f64>,
    /// Total margin in use.
    pub margin: Option<f64>,
    /// Limited-risk margin.
    pub margin_lr: Option<f64>,
    /// Non-limited-risk margin.
    pub margin_nlr: Option<f64>,
    /// Amount available to deal.
    pub available_to_deal: Option<f64>,
    /// Equity value.
    pub equity: Option<f64>,
    /// Equity used (percentage).
    pub equity_used: Option<f64>,
}

/// Field indices for `ACCOUNT:<accountId>`.
pub(crate) const ACCOUNT_FIELDS: &[&str] = &[
    "PNL",
    "DEPOSIT",
    "AVAILABLE_CASH",
    "FUNDS",
    "MARGIN",
    "MARGIN_LR",
    "MARGIN_NLR",
    "AVAILABLE_TO_DEAL",
    "EQUITY",
    "EQUITY_USED",
];

impl AccountUpdate {
    pub fn from_raw(account_id: &str, state: &[Option<String>]) -> Self {
        let get = |i: usize| state.get(i).and_then(|v| v.as_deref());
        let pf = |i: usize| get(i).and_then(|s| s.parse::<f64>().ok());
        Self {
            account_id: account_id.to_owned(),
            pnl: pf(0),
            deposit: pf(1),
            available_cash: pf(2),
            funds: pf(3),
            margin: pf(4),
            margin_lr: pf(5),
            margin_nlr: pf(6),
            available_to_deal: pf(7),
            equity: pf(8),
            equity_used: pf(9),
        }
    }
}

// ---------------------------------------------------------------------------
// TRADE:<accountId>  — DISTINCT mode
// ---------------------------------------------------------------------------

/// Nested type for a trade `CONFIRMS` JSON payload.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeConfirm {
    /// IG deal reference.
    pub deal_reference: Option<String>,
    /// IG deal ID.
    pub deal_id: Option<String>,
    /// Affected epic.
    pub epic: Option<String>,
    /// Status code (e.g. `"AMENDED"`, `"CLOSED"`, `"DELETED"`, `"OPEN"`, `"PARTIALLY_CLOSED"`).
    pub status: Option<String>,
    /// Deal status (e.g. `"ACCEPTED"`, `"REJECTED"`).
    pub deal_status: Option<String>,
    /// Any extra fields from the payload.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Nested type for an open-position update (`OPU`) JSON payload.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenPositionUpdate {
    /// IG deal ID.
    pub deal_id: Option<String>,
    /// Deal status.
    pub deal_status: Option<String>,
    /// Direction (`BUY` / `SELL`).
    pub direction: Option<String>,
    /// Epic.
    pub epic: Option<String>,
    /// Level at which the position was opened.
    pub level: Option<f64>,
    /// Size of the position.
    pub size: Option<f64>,
    /// Current price.
    pub price: Option<f64>,
    /// Status string.
    pub status: Option<String>,
    /// Any extra fields from the payload.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Nested type for a working-order update (`WOU`) JSON payload.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkingOrderUpdate {
    /// IG deal ID.
    pub deal_id: Option<String>,
    /// Deal status.
    pub deal_status: Option<String>,
    /// Epic.
    pub epic: Option<String>,
    /// Target level.
    pub level: Option<f64>,
    /// Status string.
    pub status: Option<String>,
    /// Any extra fields from the payload.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// An update from a `TRADE:<accountId>` subscription.
///
/// `CONFIRMS`, `OPU`, and `WOU` fields are JSON-encoded strings on the wire;
/// they are decoded here into structured types.
#[derive(Debug, Clone)]
pub struct TradeUpdate {
    /// The account ID this update belongs to.
    pub account_id: String,
    /// Trade confirmation (deal accepted/rejected).
    pub confirms: Option<TradeConfirm>,
    /// Open-position update.
    pub opu: Option<OpenPositionUpdate>,
    /// Working-order update.
    pub wou: Option<WorkingOrderUpdate>,
}

/// Field indices for `TRADE:<accountId>`.
pub(crate) const TRADE_FIELDS: &[&str] = &["CONFIRMS", "OPU", "WOU"];

impl TradeUpdate {
    pub fn from_raw(account_id: &str, state: &[Option<String>]) -> Self {
        let parse_str = |i: usize| state.get(i).and_then(|v| v.as_deref());
        Self {
            account_id: account_id.to_owned(),
            confirms: parse_str(0).and_then(|s| serde_json::from_str(s).ok()),
            opu: parse_str(1).and_then(|s| serde_json::from_str(s).ok()),
            wou: parse_str(2).and_then(|s| serde_json::from_str(s).ok()),
        }
    }
}
