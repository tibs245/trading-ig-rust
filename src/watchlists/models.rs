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
