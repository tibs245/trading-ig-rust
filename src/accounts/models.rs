//! Request and response types for the accounts domain.

use serde::{Deserialize, Serialize};

/// Account type enum, as returned by IG.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AccountType {
    /// CFD account.
    Cfd,
    /// Physical share dealing account.
    Physical,
    /// Spread-bet account.
    Spreadbet,
}

/// Balance snapshot for a single account.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountBalance {
    /// Net account value.
    pub balance: f64,
    /// Total deposited margin.
    pub deposit: f64,
    /// Unrealised profit/loss across open positions.
    pub profit_loss: f64,
    /// Cash available to trade without opening new positions.
    pub available_cash: f64,
}

/// A single IG account.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)] // mirrors the IG API shape
pub struct Account {
    /// Unique account identifier (e.g. `"ABC123"`).
    pub account_id: String,
    /// Human-readable alias, if set.
    pub account_alias: Option<String>,
    /// CFD, physical, or spread-bet.
    pub account_type: AccountType,
    /// Display name of the account.
    pub account_name: String,
    /// Whether funds can be transferred *to* the Master Account.
    ///
    /// IG uses the abbreviation `MA` (all-caps) in the JSON key.
    #[serde(rename = "canTransferToMA")]
    pub can_transfer_to_ma: bool,
    /// Whether funds can be transferred *from* the Master Account.
    ///
    /// IG uses the abbreviation `MA` (all-caps) in the JSON key.
    #[serde(rename = "canTransferFromMA")]
    pub can_transfer_from_ma: bool,
    /// `true` if this is the currently logged-in account.
    pub default_account: bool,
    /// `true` if this account is marked as preferred.
    pub preferred: bool,
    /// Current balance details.
    pub balance: AccountBalance,
}

/// Account trading preferences.
///
/// Decorated with `#[serde(default)]` so that IG can add more fields in
/// future without breaking deserialisation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AccountPreferences {
    /// Whether trailing stops are enabled for this account.
    pub trailing_stops_enabled: bool,
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// Request body for `PUT /accounts/preferences`.
///
/// IG expects the boolean as a **string** (`"true"` / `"false"`), not a JSON
/// boolean — this wrapper handles that serialisation.
#[derive(Debug, Clone)]
pub struct UpdatePreferences {
    /// Whether trailing stops should be enabled.
    pub trailing_stops_enabled: bool,
}

/// Internal wire representation for `UpdatePreferences`. IG requires the
/// bool to be sent as a JSON string (e.g. `"trailingStopsEnabled": "true"`).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct UpdatePreferencesWire<'a> {
    pub trailing_stops_enabled: &'a str,
}

impl UpdatePreferences {
    pub(super) fn to_wire(&self) -> UpdatePreferencesWire<'_> {
        UpdatePreferencesWire {
            trailing_stops_enabled: if self.trailing_stops_enabled {
                "true"
            } else {
                "false"
            },
        }
    }
}
