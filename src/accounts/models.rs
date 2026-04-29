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
///
/// All numeric fields are `Option<f64>` because IG omits any of them on
/// freshly created demo accounts (no funded positions → no unrealised P/L,
/// no available-cash row, etc.) and we want to deserialise those responses
/// rather than fail.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AccountBalance {
    /// Net account value.
    pub balance: Option<f64>,
    /// Total deposited margin.
    pub deposit: Option<f64>,
    /// Unrealised profit/loss across open positions.
    pub profit_loss: Option<f64>,
    /// Cash available to trade without opening new positions.
    pub available_cash: Option<f64>,
}

/// A single IG account.
///
/// Most fields are `Option`al because IG omits them on demo accounts that
/// lack the matching capability. Verified against live demo responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
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
    /// Optional — absent in live demo responses.
    #[serde(rename = "canTransferToMA")]
    pub can_transfer_to_ma: Option<bool>,
    /// Whether funds can be transferred *from* the Master Account.
    ///
    /// IG uses the abbreviation `MA` (all-caps) in the JSON key.
    /// Optional — absent in live demo responses.
    #[serde(rename = "canTransferFromMA")]
    pub can_transfer_from_ma: Option<bool>,
    /// `true` if this is the currently logged-in account.
    pub default_account: bool,
    /// `true` if this account is marked as preferred.
    pub preferred: bool,
    /// Current balance details. Optional because some account responses
    /// (sparse demo accounts) omit the entire `balance` block.
    pub balance: AccountBalance,
}

impl Default for Account {
    fn default() -> Self {
        Self {
            account_id: String::new(),
            account_alias: None,
            account_type: AccountType::Cfd,
            account_name: String::new(),
            can_transfer_to_ma: None,
            can_transfer_from_ma: None,
            default_account: false,
            preferred: false,
            balance: AccountBalance::default(),
        }
    }
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
