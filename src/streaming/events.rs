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
// PRICE:<accountId>:<epic>  — MERGE mode, data adapter `Pricing`
// ---------------------------------------------------------------------------

/// Number of ladder tiers exposed by the `PRICE` subscription.
pub const PRICE_LADDER_TIERS: usize = 5;

/// A single update from a `PRICE:<accountId>:<epic>` subscription.
///
/// Replaces [`MarketUpdate`]. Migration table: `_knowledge/api/streaming.md`.
#[derive(Debug, Clone, Default)]
pub struct PriceUpdate {
    /// The IG epic this update belongs to.
    pub epic: String,
    /// The IG account identifier the subscription was opened for.
    pub account_id: String,
    /// Best bid price — tier 1 of the bid ladder (`BIDPRICE1`).
    pub bid: Option<f64>,
    /// Best offer/ask price — tier 1 of the ask ladder (`ASKPRICE1`).
    pub offer: Option<f64>,
    /// Full bid ladder, tiers 1–5 (`BIDPRICE1`..`BIDPRICE5`).
    pub bid_prices: [Option<f64>; PRICE_LADDER_TIERS],
    /// Full ask ladder, tiers 1–5 (`ASKPRICE1`..`ASKPRICE5`).
    pub ask_prices: [Option<f64>; PRICE_LADDER_TIERS],
    /// Size available at each bid tier (`BIDSIZE1`..`BIDSIZE5`).
    ///
    /// All-`None` means the instrument has no ladder configured.
    pub bid_sizes: [Option<f64>; PRICE_LADDER_TIERS],
    /// Size available at each ask tier (`ASKSIZE1`..`ASKSIZE5`).
    ///
    /// All-`None` means the instrument has no ladder configured.
    pub ask_sizes: [Option<f64>; PRICE_LADDER_TIERS],
    /// Opening mid price (`MID_OPEN`).
    pub mid_open: Option<f64>,
    /// Intraday high price (`HIGH`).
    pub high: Option<f64>,
    /// Intraday low price (`LOW`).
    pub low: Option<f64>,
    /// Price change vs. open (`NET_CHG`).
    pub change: Option<f64>,
    /// Percentage change vs. open (`NET_CHG_PCT`).
    pub change_pct: Option<f64>,
    /// Server-side update timestamp in **UTC milliseconds** (`TIMESTAMP`).
    pub timestamp: Option<i64>,
    /// Whether price quotes are delayed (`true`) or live (`false`) (`DELAY`).
    pub delayed: Option<bool>,
    /// Dealing status (`DLG_FLAG`) — e.g. `"DEAL"`, `"CLOSED"`, `"SUSPEND"`.
    pub dlg_flag: Option<String>,
    /// ID of the bid quote to reference when trading (`BIDQUOTEID`).
    pub bid_quote_id: Option<String>,
    /// ID of the ask quote to reference when trading (`ASKQUOTEID`).
    pub ask_quote_id: Option<String>,
    /// Currency of the default ladder (`CURRENCY0`).
    ///
    /// Only guaranteed to be populated when a ladder exists.
    pub currency: Option<String>,
}

/// Field indices for `PRICE:<accountId>:<epic>`.
///
/// The order here **is** the wire order: `PriceUpdate::from_raw` indexes into
/// the merged field state positionally, so the two must stay in lock-step.
pub(crate) const PRICE_FIELDS: &[&str] = &[
    "BIDPRICE1",
    "BIDPRICE2",
    "BIDPRICE3",
    "BIDPRICE4",
    "BIDPRICE5",
    "ASKPRICE1",
    "ASKPRICE2",
    "ASKPRICE3",
    "ASKPRICE4",
    "ASKPRICE5",
    "BIDSIZE1",
    "BIDSIZE2",
    "BIDSIZE3",
    "BIDSIZE4",
    "BIDSIZE5",
    "ASKSIZE1",
    "ASKSIZE2",
    "ASKSIZE3",
    "ASKSIZE4",
    "ASKSIZE5",
    "MID_OPEN",
    "HIGH",
    "LOW",
    "NET_CHG",
    "NET_CHG_PCT",
    "TIMESTAMP",
    "DELAY",
    "DLG_FLAG",
    "BIDQUOTEID",
    "ASKQUOTEID",
    "CURRENCY0",
];

/// Index of the first `BIDPRICE`/`ASKPRICE`/`BIDSIZE`/`ASKSIZE` tier in
/// [`PRICE_FIELDS`], so the ladder blocks are read positionally.
const PRICE_BID_PRICE_BASE: usize = 0;
const PRICE_ASK_PRICE_BASE: usize = 5;
const PRICE_BID_SIZE_BASE: usize = 10;
const PRICE_ASK_SIZE_BASE: usize = 15;

impl PriceUpdate {
    /// Construct from a raw field-value slice (in `PRICE_FIELDS` order).
    pub fn from_raw(account_id: &str, epic: &str, state: &[Option<String>]) -> Self {
        let get = |i: usize| state.get(i).and_then(|v| v.as_deref());
        let pf = |i: usize| get(i).and_then(|s| s.trim().parse::<f64>().ok());
        let ladder = |base: usize| std::array::from_fn(|t| pf(base + t));
        let owned = |i: usize| get(i).map(|s| s.trim().to_owned());

        let bid_prices: [Option<f64>; PRICE_LADDER_TIERS] = ladder(PRICE_BID_PRICE_BASE);
        let ask_prices: [Option<f64>; PRICE_LADDER_TIERS] = ladder(PRICE_ASK_PRICE_BASE);

        Self {
            epic: epic.to_owned(),
            account_id: account_id.to_owned(),
            bid: bid_prices[0],
            offer: ask_prices[0],
            bid_prices,
            ask_prices,
            bid_sizes: ladder(PRICE_BID_SIZE_BASE),
            ask_sizes: ladder(PRICE_ASK_SIZE_BASE),
            mid_open: pf(20),
            high: pf(21),
            low: pf(22),
            change: pf(23),
            change_pct: pf(24),
            timestamp: get(25).and_then(|s| s.trim().parse::<i64>().ok()),
            delayed: get(26).and_then(|s| parse_delay_flag(s.trim())),
            // IG's reference table renders the DLG_FLAG constants with trailing
            // spaces; trim so callers can match on the bare constant.
            dlg_flag: owned(27),
            bid_quote_id: owned(28),
            ask_quote_id: owned(29),
            currency: owned(30),
        }
    }
}

/// Parse the `0`/`1` (or `false`/`true`) delayed-price flag shared by the
/// `PRICE` `DELAY` and the deprecated `MARKET` `MARKET_DELAY` fields.
fn parse_delay_flag(s: &str) -> Option<bool> {
    match s {
        "0" | "false" => Some(false),
        "1" | "true" => Some(true),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// MARKET:<epic>  — MERGE mode — DEPRECATED by IG
// ---------------------------------------------------------------------------

/// A single update from a `MARKET:<epic>` subscription.
///
/// **DEPRECATED by IG** (EOL 1 May 2026, decommissioned 8 May 2026).
/// Migrate to [`PriceUpdate`].
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
            market_delay: get(8).and_then(parse_delay_flag),
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
    /// One-second candle.
    Second,
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
            Self::Second => "SECOND",
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
    /// Last traded volume.
    pub ltv: Option<f64>,
    /// Incremental volume.
    pub ttv: Option<f64>,
    /// Mid open price for the day.
    pub day_open_mid: Option<f64>,
    /// Change from open price to current (mid).
    pub day_net_chg_mid: Option<f64>,
    /// Daily percentage change (mid).
    pub day_perc_chg_mid: Option<f64>,
    /// Daily high price (mid).
    pub day_high: Option<f64>,
    /// Daily low price (mid).
    pub day_low: Option<f64>,
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
    "LTV",
    "TTV",
    "DAY_OPEN_MID",
    "DAY_NET_CHG_MID",
    "DAY_PERC_CHG_MID",
    "DAY_HIGH",
    "DAY_LOW",
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
            ltv: pf(15),
            ttv: pf(16),
            day_open_mid: pf(17),
            day_net_chg_mid: pf(18),
            day_perc_chg_mid: pf(19),
            day_high: pf(20),
            day_low: pf(21),
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
    /// Profit and loss on limited-risk positions.
    pub pnl_lr: Option<f64>,
    /// Profit and loss on non-limited-risk positions.
    pub pnl_nlr: Option<f64>,
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
    "PNL_LR",
    "PNL_NLR",
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
            pnl_lr: pf(10),
            pnl_nlr: pf(11),
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

#[cfg(test)]
mod tests {
    use super::*;

    // Each *_FIELDS array IS the wire order sent as LS_schema, and every
    // from_raw indexes it positionally. Reordering one without the other makes
    // `bid` return a ladder size — silently, with every other test still green.
    // These anchors are the only thing that catches that.

    #[test]
    fn price_field_order_is_pinned() {
        assert_eq!(PRICE_FIELDS.len(), 31);
        assert_eq!(PRICE_FIELDS[PRICE_BID_PRICE_BASE], "BIDPRICE1");
        assert_eq!(PRICE_FIELDS[PRICE_BID_PRICE_BASE + 4], "BIDPRICE5");
        assert_eq!(PRICE_FIELDS[PRICE_ASK_PRICE_BASE], "ASKPRICE1");
        assert_eq!(PRICE_FIELDS[PRICE_ASK_PRICE_BASE + 4], "ASKPRICE5");
        assert_eq!(PRICE_FIELDS[PRICE_BID_SIZE_BASE], "BIDSIZE1");
        assert_eq!(PRICE_FIELDS[PRICE_BID_SIZE_BASE + 4], "BIDSIZE5");
        assert_eq!(PRICE_FIELDS[PRICE_ASK_SIZE_BASE], "ASKSIZE1");
        assert_eq!(PRICE_FIELDS[PRICE_ASK_SIZE_BASE + 4], "ASKSIZE5");
        assert_eq!(
            &PRICE_FIELDS[20..],
            &[
                "MID_OPEN",
                "HIGH",
                "LOW",
                "NET_CHG",
                "NET_CHG_PCT",
                "TIMESTAMP",
                "DELAY",
                "DLG_FLAG",
                "BIDQUOTEID",
                "ASKQUOTEID",
                "CURRENCY0",
            ]
        );
    }

    #[test]
    fn market_field_order_is_pinned() {
        assert_eq!(
            MARKET_FIELDS,
            &[
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
            ]
        );
    }

    #[test]
    fn chart_candle_field_order_is_pinned() {
        assert_eq!(CHART_CANDLE_FIELDS.len(), 22);
        assert_eq!(CHART_CANDLE_FIELDS[0], "OFR_OPEN");
        assert_eq!(CHART_CANDLE_FIELDS[12], "CONS_END");
        assert_eq!(CHART_CANDLE_FIELDS[14], "UTM");
        assert_eq!(
            &CHART_CANDLE_FIELDS[15..],
            &[
                "LTV",
                "TTV",
                "DAY_OPEN_MID",
                "DAY_NET_CHG_MID",
                "DAY_PERC_CHG_MID",
                "DAY_HIGH",
                "DAY_LOW",
            ]
        );
    }

    #[test]
    fn chart_tick_field_order_is_pinned() {
        assert_eq!(CHART_TICK_FIELDS.len(), 11);
        assert_eq!(CHART_TICK_FIELDS[0], "BID");
        assert_eq!(CHART_TICK_FIELDS[5], "UTM");
        assert_eq!(CHART_TICK_FIELDS[10], "DAY_LOW");
    }

    #[test]
    fn account_field_order_is_pinned() {
        assert_eq!(ACCOUNT_FIELDS.len(), 12);
        assert_eq!(ACCOUNT_FIELDS[0], "PNL");
        assert_eq!(ACCOUNT_FIELDS[9], "EQUITY_USED");
        assert_eq!(ACCOUNT_FIELDS[10], "PNL_LR");
        assert_eq!(ACCOUNT_FIELDS[11], "PNL_NLR");
    }

    #[test]
    fn trade_field_order_is_pinned() {
        assert_eq!(TRADE_FIELDS, &["CONFIRMS", "OPU", "WOU"]);
    }
}
