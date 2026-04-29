//! Request and response types for the watchlists domain.

use serde::{Deserialize, Serialize};

use crate::models::common::Epic;

// ---------------------------------------------------------------------------
// Watchlist summary (returned by `list`)
// ---------------------------------------------------------------------------

/// A brief description of one watchlist returned by `GET /watchlists`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchlistSummary {
    /// Opaque watchlist identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Whether the authenticated user can add/remove instruments.
    pub editable: bool,
    /// Whether the authenticated user can delete this watchlist.
    pub deleteable: bool,
    /// `true` for system-defined watchlists (e.g. "Popular Markets").
    pub default_system_watchlist: bool,
}

// ---------------------------------------------------------------------------
// Create watchlist
// ---------------------------------------------------------------------------

/// Request body for `POST /watchlists`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWatchlistRequest {
    /// Display name for the new watchlist.
    pub name: String,
    /// Initial set of epics to add. May be empty.
    pub epics: Vec<Epic>,
}

/// Response from `POST /watchlists`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWatchlistResponse {
    /// Opaque identifier of the newly created watchlist.
    pub watchlist_id: String,
    /// `SUCCESS` or `SUCCESS_NOT_ALL_INSTRUMENTS_ADDED`.
    pub status: CreateWatchlistStatus,
}

/// Outcome status for `POST /watchlists`.
///
/// A `SuccessNotAllInstrumentsAdded` result is still HTTP 200 — it means the
/// watchlist was created but one or more of the requested epics were not
/// recognised or could not be added.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CreateWatchlistStatus {
    /// All requested instruments were added successfully.
    Success,
    /// Watchlist was created but not every epic was added.
    SuccessNotAllInstrumentsAdded,
}

// ---------------------------------------------------------------------------
// Market summary (returned by `markets`)
// ---------------------------------------------------------------------------

/// A brief market snapshot inside a watchlist, as returned by
/// `GET /watchlists/{id}`.
///
/// This type is defined locally in the `watchlists` domain. A similarly-named
/// type may exist in the `markets` domain; after Vague 1 is merged the two
/// can be unified in `src/models/common.rs` if they are identical.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketSummary {
    /// Full display name of the instrument.
    pub instrument_name: String,
    /// Expiry code, e.g. `"DFB"` or `"Dec-24"`.
    pub expiry: String,
    /// IG market identifier.
    pub epic: Epic,
    /// Instrument category (e.g. `"CURRENCIES"`, `"SHARES"`).
    pub instrument_type: String,
    /// Current bid price.
    pub bid: Option<f64>,
    /// Current offer/ask price.
    pub offer: Option<f64>,
    /// Today's high.
    pub high: Option<f64>,
    /// Today's low.
    pub low: Option<f64>,
    /// Percentage change from the previous close.
    pub percentage_change: Option<f64>,
    /// Absolute net change from the previous close.
    pub net_change: Option<f64>,
    /// Local time of the last price update (HH:MM:SS).
    pub update_time: Option<String>,
    /// UTC time of the last price update (HH:MM:SS).
    pub update_time_utc: Option<String>,
    /// Whether streaming prices are available for this instrument.
    pub streaming_prices_available: bool,
    /// Market status (e.g. `"TRADEABLE"`, `"CLOSED"`).
    pub market_status: String,
    /// Scaling factor applied to prices.
    pub scaling_factor: Option<i64>,
}

// ---------------------------------------------------------------------------
// Add market
// ---------------------------------------------------------------------------

/// Response from `PUT /watchlists/{id}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddMarketResponse {
    /// Outcome status string returned by IG (typically `"SUCCESS"`).
    pub status: String,
}

// ---------------------------------------------------------------------------
// Remove market / delete watchlist (unit responses)
// ---------------------------------------------------------------------------

/// Response from `DELETE /watchlists/{id}/{epic}`.
///
/// IG returns an empty body on success; this unit struct deserialises from
/// `null` (which the transport synthesises for empty responses).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveMarketResponse;

/// Response from `DELETE /watchlists/{id}`.
///
/// IG returns an empty body on success; this unit struct deserialises from
/// `null` (which the transport synthesises for empty responses).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteWatchlistResponse;
