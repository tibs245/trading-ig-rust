//! Request and response types for the history domain.
//!
//! Covers `/history/activity` (v1 + v3) and `/history/transactions` (v1 + v2).

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::models::common::{Currency, DealId, Epic};

// ────────────────────────────────────────────────────────────────────────────
// Activity v3 — request
// ────────────────────────────────────────────────────────────────────────────

/// Request parameters for `GET /history/activity` (v3).
///
/// All fields are optional; use `Default::default()` for a request with no
/// filters (returns the most recent activities up to `page_size` items per
/// page, auto-followed until exhausted).
///
/// # FIQL filter
///
/// The `filter` field accepts a raw FIQL expression string. Supported
/// operators:
/// - `==` equals (e.g. `type==POSITION`)
/// - `!=` not equals
/// - `,` OR (e.g. `status==ACCEPTED,status==REJECTED`)
/// - `;` AND (e.g. `type==POSITION;status==ACCEPTED`)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityRequest {
    /// Earliest date/time to include (ISO-8601: `YYYY-MM-DDTHH:MM:SS`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<NaiveDateTime>,

    /// Latest date/time to include.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<NaiveDateTime>,

    /// If `true`, expand sub-objects in the response (`ActivityDetails`).
    pub detailed: bool,

    /// Filter by a specific deal ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_id: Option<DealId>,

    /// Raw FIQL filter expression (see struct-level docs for operators).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,

    /// Number of results per page. IG accepts 10–500; default is 50.
    pub page_size: u32,
}

impl Default for ActivityRequest {
    fn default() -> Self {
        Self {
            from: None,
            to: None,
            detailed: false,
            deal_id: None,
            filter: None,
            page_size: 50,
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Activity v3 — response
// ────────────────────────────────────────────────────────────────────────────

/// A single activity record returned by `GET /history/activity` (v3).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    /// Activity timestamp (ISO-8601).
    pub date: NaiveDateTime,

    /// Market identifier.
    pub epic: Epic,

    /// Deal period (e.g. `"DFB"`, `"-"`).
    pub period: String,

    /// Unique deal identifier.
    pub deal_id: DealId,

    /// Channel through which the activity originated.
    pub channel: ActivityChannel,

    /// Type of activity.
    #[serde(rename = "type")]
    pub activity_type: ActivityType,

    /// Outcome of the activity.
    pub status: ActivityStatus,

    /// Human-readable description.
    pub description: String,

    /// Expanded sub-objects; only populated when `detailed = true`.
    pub details: Option<ActivityDetails>,
}

/// Channel through which an activity originated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActivityChannel {
    /// Web platform.
    Web,
    /// Mobile app.
    Mobile,
    /// Dealer (phone trade).
    Dealer,
    /// Automated API call.
    PublicWebApi,
    /// System-generated (e.g. auto-close on margin call).
    System,
    /// Unclassified.
    #[serde(other)]
    Unknown,
}

/// Type of activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActivityType {
    /// Position open / close / update.
    Position,
    /// Working order create / update / delete.
    WorkingOrder,
    /// Account-level edit.
    Edit,
    /// System-generated event.
    System,
    #[serde(other)]
    Unknown,
}

/// Outcome status of an activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActivityStatus {
    /// Activity was accepted and executed.
    Accepted,
    /// Activity was rejected.
    Rejected,
    /// Status cannot be determined.
    Unknown,
}

/// Expanded details for an [`Activity`] when `detailed = true`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityDetails {
    /// Deal actions that make up this activity.
    pub actions: Vec<ActivityAction>,

    /// Market name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_name: Option<String>,

    /// Currency of the trade.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,

    /// Trade direction (`BUY` / `SELL`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,

    /// Deal size.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<f64>,

    /// Limit level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_level: Option<f64>,

    /// Stop level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_level: Option<f64>,

    /// Whether the stop is a trailing stop.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trailing_stop: Option<bool>,

    /// Guaranteed stop indicator.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guaranteed_stop: Option<bool>,
}

/// An individual action within an [`ActivityDetails`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityAction {
    /// Type code for this action (e.g. `"POSITION_OPENED"`).
    pub action_type: String,

    /// Affected deal reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affected_deal_id: Option<String>,
}

// ────────────────────────────────────────────────────────────────────────────
// Activity v1 — response
// ────────────────────────────────────────────────────────────────────────────

/// A single activity record returned by `GET /history/activity/{ms}` or
/// `GET /history/activity/{from}/{to}` (v1).
///
/// The v1 schema differs from [`Activity`] (v3): it uses different field
/// names, different date formats, and lacks the `channel` / `details` fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityV1 {
    /// Activity channel.
    pub channel: String,

    /// Trade date in IG v1 format (`YYYY:MM:DD-HH:MM:SS`).
    #[serde(with = "ig_v1_dt")]
    pub date: NaiveDateTime,

    /// Deal identifier.
    pub deal_id: DealId,

    /// Activity description.
    pub description: String,

    /// Market instrument details.
    pub details: Option<String>,

    /// Market epic.
    pub epic: Epic,

    /// Deal period.
    pub period: String,

    /// Activity status.
    pub status: String,

    /// Activity type.
    #[serde(rename = "type")]
    pub activity_type: String,
}

// ────────────────────────────────────────────────────────────────────────────
// Transactions v2 — request + response
// ────────────────────────────────────────────────────────────────────────────

/// Transaction type filter for `GET /history/transactions`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionType {
    /// All transactions.
    All,
    /// All deal-related transactions.
    AllDeal,
    /// Deposits only.
    Deposit,
    /// Withdrawals only.
    Withdrawal,
}

/// Request parameters for `GET /history/transactions` (v2).
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionsRequest {
    /// Transaction type filter.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub trans_type: Option<TransactionType>,

    /// Earliest date/time (ISO-8601).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<NaiveDateTime>,

    /// Latest date/time (ISO-8601).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<NaiveDateTime>,

    /// Maximum span in seconds; IG limits to its configured max.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_span_seconds: Option<u32>,

    /// Number of results per page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<u32>,

    /// Page number to retrieve (1-based).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_number: Option<u32>,
}

/// Response envelope for `GET /history/transactions` (v2).
///
/// Contains the transactions for the requested page plus paging metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionsResponse {
    /// Transactions on this page.
    pub transactions: Vec<Transaction>,

    /// Pagination metadata.
    pub metadata: TransactionsMetadata,
}

/// Paging metadata returned by the transactions v2 endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionsMetadata {
    /// Page data: current page, page size, total pages, total records.
    pub page_data: PageData,
}

/// Pagination detail block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageData {
    /// Current (1-based) page number.
    pub page_number: u32,

    /// Number of items per page.
    pub page_size: u32,

    /// Total number of pages.
    pub total_pages: u32,
}

/// A single transaction record.
///
/// IG returns most numeric fields (`profit_and_loss`, `size`, `open_level`,
/// `close_level`) as strings, sometimes with a currency symbol prefix (e.g.
/// `"EUR1234.56"` or `"1234.56"`). They are kept as `String` here to avoid
/// data loss. Use the helper methods to parse the numeric value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    /// Local date string (format varies; keep as `String`).
    pub date: String,

    /// UTC timestamp (ISO-8601).
    pub date_utc: NaiveDateTime,

    /// Instrument / market name.
    pub instrument_name: String,

    /// Deal expiry period.
    pub period: String,

    /// Profit / loss including currency symbol (e.g. `"EUR1234.56"`).
    pub profit_and_loss: String,

    /// Transaction type string (e.g. `"TRADE"`, `"DIVIDEND"`).
    pub transaction_type: String,

    /// Unique transaction reference.
    pub reference: String,

    /// Opening level as a string (may include currency symbol).
    pub open_level: String,

    /// Closing level as a string (may include currency symbol).
    pub close_level: String,

    /// Trade size as a string (signed; may include currency symbol).
    pub size: String,

    /// ISO-4217 currency code.
    pub currency: Currency,

    /// True if this is a cash (non-deal) transaction.
    pub cash_transaction: bool,
}

impl Transaction {
    /// Parse the numeric profit/loss value, stripping any leading currency
    /// symbol characters.
    ///
    /// Returns `None` if the string cannot be parsed as `f64` after stripping.
    pub fn profit_and_loss_value(&self) -> Option<f64> {
        parse_ig_numeric(&self.profit_and_loss)
    }

    /// Parse the numeric open level, stripping any leading currency symbol.
    pub fn open_level_value(&self) -> Option<f64> {
        parse_ig_numeric(&self.open_level)
    }

    /// Parse the numeric close level, stripping any leading currency symbol.
    pub fn close_level_value(&self) -> Option<f64> {
        parse_ig_numeric(&self.close_level)
    }

    /// Parse the numeric size, stripping any leading currency symbol.
    pub fn size_value(&self) -> Option<f64> {
        parse_ig_numeric(&self.size)
    }
}

/// Strip any leading non-digit, non-sign characters (e.g. currency codes like
/// `"EUR"` or `"GBP"`) and parse the remainder as `f64`.
fn parse_ig_numeric(s: &str) -> Option<f64> {
    // Find the first character that could start a floating-point number.
    let start = s.find(|c: char| c == '-' || c == '+' || c.is_ascii_digit())?;
    s[start..].parse::<f64>().ok()
}

// ────────────────────────────────────────────────────────────────────────────
// Polars conversions
// ────────────────────────────────────────────────────────────────────────────

#[cfg(feature = "polars")]
impl crate::dataframe::IntoDataFrame for Vec<Activity> {
    /// Convert a list of v3 activities into a `polars::prelude::DataFrame`.
    ///
    /// Column layout:
    ///
    /// | column          | dtype      | nullable |
    /// | --------------- | ---------- | -------- |
    /// | `date`          | `Datetime` | no       |
    /// | `epic`          | `Utf8`     | no       |
    /// | `period`        | `Utf8`     | no       |
    /// | `deal_id`       | `Utf8`     | no       |
    /// | `channel`       | `Utf8`     | no       |
    /// | `activity_type` | `Utf8`     | no       |
    /// | `status`        | `Utf8`     | no       |
    /// | `description`   | `Utf8`     | no       |
    fn to_dataframe(&self) -> crate::Result<polars::prelude::DataFrame> {
        use polars::prelude::*;

        let date: Vec<chrono::NaiveDateTime> = self.iter().map(|a| a.date).collect();
        let epic: Vec<&str> = self.iter().map(|a| a.epic.as_str()).collect();
        let period: Vec<&str> = self.iter().map(|a| a.period.as_str()).collect();
        let deal_id: Vec<&str> = self.iter().map(|a| a.deal_id.as_str()).collect();
        let channel: Vec<&str> = self
            .iter()
            .map(|a| activity_channel_str(a.channel))
            .collect();
        let activity_type: Vec<&str> = self
            .iter()
            .map(|a| activity_type_str(a.activity_type))
            .collect();
        let status: Vec<&str> = self.iter().map(|a| activity_status_str(a.status)).collect();
        let description: Vec<&str> = self.iter().map(|a| a.description.as_str()).collect();

        let date_series = Series::new("date".into(), date);

        DataFrame::new(vec![
            date_series.into(),
            Column::new("epic".into(), epic),
            Column::new("period".into(), period),
            Column::new("deal_id".into(), deal_id),
            Column::new("channel".into(), channel),
            Column::new("activity_type".into(), activity_type),
            Column::new("status".into(), status),
            Column::new("description".into(), description),
        ])
        .map_err(|e| crate::Error::Config(format!("polars conversion failed: {e}")))
    }
}

#[cfg(feature = "polars")]
fn activity_channel_str(c: ActivityChannel) -> &'static str {
    match c {
        ActivityChannel::Web => "WEB",
        ActivityChannel::Mobile => "MOBILE",
        ActivityChannel::Dealer => "DEALER",
        ActivityChannel::PublicWebApi => "PUBLIC_WEB_API",
        ActivityChannel::System => "SYSTEM",
        ActivityChannel::Unknown => "UNKNOWN",
    }
}

#[cfg(feature = "polars")]
fn activity_type_str(t: ActivityType) -> &'static str {
    match t {
        ActivityType::Position => "POSITION",
        ActivityType::WorkingOrder => "WORKING_ORDER",
        ActivityType::Edit => "EDIT",
        ActivityType::System => "SYSTEM",
        ActivityType::Unknown => "UNKNOWN",
    }
}

#[cfg(feature = "polars")]
fn activity_status_str(s: ActivityStatus) -> &'static str {
    match s {
        ActivityStatus::Accepted => "ACCEPTED",
        ActivityStatus::Rejected => "REJECTED",
        ActivityStatus::Unknown => "UNKNOWN",
    }
}

#[cfg(feature = "polars")]
impl crate::dataframe::IntoDataFrame for Vec<Transaction> {
    /// Convert a list of transactions into a `polars::prelude::DataFrame`.
    ///
    /// Stringified numeric IG fields are retained as `Utf8` columns with the
    /// raw string values (which may include currency symbols, e.g.
    /// `"EUR1234.56"`). Alongside each, a `_value` `Float64` column is added
    /// with the parsed number (`null` when the string cannot be parsed).
    ///
    /// Column layout:
    ///
    /// | column                  | dtype      | nullable |
    /// | ----------------------- | ---------- | -------- |
    /// | `date`                  | `Utf8`     | no       |
    /// | `date_utc`              | `Datetime` | no       |
    /// | `instrument_name`       | `Utf8`     | no       |
    /// | `period`                | `Utf8`     | no       |
    /// | `transaction_type`      | `Utf8`     | no       |
    /// | `reference`             | `Utf8`     | no       |
    /// | `currency`              | `Utf8`     | no       |
    /// | `cash_transaction`      | `Boolean`  | no       |
    /// | `profit_and_loss`       | `Utf8`     | no       |
    /// | `profit_and_loss_value` | `Float64`  | yes      |
    /// | `open_level`            | `Utf8`     | no       |
    /// | `open_level_value`      | `Float64`  | yes      |
    /// | `close_level`           | `Utf8`     | no       |
    /// | `close_level_value`     | `Float64`  | yes      |
    /// | `size`                  | `Utf8`     | no       |
    /// | `size_value`            | `Float64`  | yes      |
    fn to_dataframe(&self) -> crate::Result<polars::prelude::DataFrame> {
        use polars::prelude::*;

        let date: Vec<&str> = self.iter().map(|t| t.date.as_str()).collect();
        let date_utc: Vec<chrono::NaiveDateTime> = self.iter().map(|t| t.date_utc).collect();
        let instrument_name: Vec<&str> = self.iter().map(|t| t.instrument_name.as_str()).collect();
        let period: Vec<&str> = self.iter().map(|t| t.period.as_str()).collect();
        let transaction_type: Vec<&str> =
            self.iter().map(|t| t.transaction_type.as_str()).collect();
        let reference: Vec<&str> = self.iter().map(|t| t.reference.as_str()).collect();
        let currency: Vec<&str> = self.iter().map(|t| t.currency.as_str()).collect();
        let cash_transaction: Vec<bool> = self.iter().map(|t| t.cash_transaction).collect();
        // Raw string columns (preserve currency symbol, sign, etc.)
        let profit_and_loss: Vec<&str> = self.iter().map(|t| t.profit_and_loss.as_str()).collect();
        let profit_and_loss_value: Vec<Option<f64>> = self
            .iter()
            .map(Transaction::profit_and_loss_value)
            .collect();
        let open_level: Vec<&str> = self.iter().map(|t| t.open_level.as_str()).collect();
        let open_level_value: Vec<Option<f64>> =
            self.iter().map(Transaction::open_level_value).collect();
        let close_level: Vec<&str> = self.iter().map(|t| t.close_level.as_str()).collect();
        let close_level_value: Vec<Option<f64>> =
            self.iter().map(Transaction::close_level_value).collect();
        let size: Vec<&str> = self.iter().map(|t| t.size.as_str()).collect();
        let size_value: Vec<Option<f64>> = self.iter().map(Transaction::size_value).collect();

        let date_utc_series = Series::new("date_utc".into(), date_utc);

        DataFrame::new(vec![
            Column::new("date".into(), date),
            date_utc_series.into(),
            Column::new("instrument_name".into(), instrument_name),
            Column::new("period".into(), period),
            Column::new("transaction_type".into(), transaction_type),
            Column::new("reference".into(), reference),
            Column::new("currency".into(), currency),
            Column::new("cash_transaction".into(), cash_transaction),
            Column::new("profit_and_loss".into(), profit_and_loss),
            Column::new("profit_and_loss_value".into(), profit_and_loss_value),
            Column::new("open_level".into(), open_level),
            Column::new("open_level_value".into(), open_level_value),
            Column::new("close_level".into(), close_level),
            Column::new("close_level_value".into(), close_level_value),
            Column::new("size".into(), size),
            Column::new("size_value".into(), size_value),
        ])
        .map_err(|e| crate::Error::Config(format!("polars conversion failed: {e}")))
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Internal pagination types
// ────────────────────────────────────────────────────────────────────────────

/// Envelope for the activity v3 response (one page).
#[derive(Debug, Deserialize)]
pub(crate) struct ActivityPage {
    pub activities: Vec<Activity>,
    pub metadata: ActivityMetadata,
}

/// Metadata block in the activity v3 response.
#[derive(Debug, Deserialize)]
pub(crate) struct ActivityMetadata {
    pub paging: ActivityPaging,
}

/// Paging block in the activity v3 metadata.
#[derive(Debug, Deserialize)]
pub(crate) struct ActivityPaging {
    /// URL of the next page; `null` or absent when there are no more pages.
    pub next: Option<String>,
    /// Number of items on the current page (informational).
    #[allow(dead_code)]
    pub size: u32,
}

/// Envelope for activity v1 responses.
#[derive(Debug, Deserialize)]
pub(crate) struct ActivityV1Envelope {
    pub activities: Vec<ActivityV1>,
}

// ────────────────────────────────────────────────────────────────────────────
// Date/time serde helpers for v1 format
// ────────────────────────────────────────────────────────────────────────────

mod ig_v1_dt {
    use chrono::NaiveDateTime;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(dt: &NaiveDateTime, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&crate::time::format(*dt, crate::time::ApiVersion::V1))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<NaiveDateTime, D::Error> {
        let raw = String::deserialize(d)?;
        crate::time::parse(&raw, crate::time::ApiVersion::V1).map_err(serde::de::Error::custom)
    }
}
