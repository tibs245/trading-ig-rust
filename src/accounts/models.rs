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

// ---------------------------------------------------------------------------
// Polars conversion
// ---------------------------------------------------------------------------

#[cfg(feature = "polars")]
impl crate::dataframe::IntoDataFrame for Vec<Account> {
    /// Convert a list of accounts into a `polars::prelude::DataFrame`.
    ///
    /// Column layout:
    ///
    /// | column              | dtype    | nullable |
    /// | ------------------- | -------- | -------- |
    /// | `account_id`        | `Utf8`   | no       |
    /// | `account_alias`     | `Utf8`   | yes      |
    /// | `account_type`      | `Utf8`   | no       |
    /// | `account_name`      | `Utf8`   | no       |
    /// | `can_transfer_to_ma`| `Boolean`| yes      |
    /// | `can_transfer_from_ma`| `Boolean`| yes    |
    /// | `default_account`   | `Boolean`| no       |
    /// | `preferred`         | `Boolean`| no       |
    /// | `balance`           | `Float64`| yes      |
    /// | `deposit`           | `Float64`| yes      |
    /// | `profit_loss`       | `Float64`| yes      |
    /// | `available_cash`    | `Float64`| yes      |
    fn to_dataframe(&self) -> crate::Result<polars::prelude::DataFrame> {
        use polars::prelude::*;

        let account_id: Vec<&str> = self.iter().map(|a| a.account_id.as_str()).collect();
        let account_alias: Vec<Option<&str>> =
            self.iter().map(|a| a.account_alias.as_deref()).collect();
        let account_type: Vec<&str> = self
            .iter()
            .map(|a| match a.account_type {
                AccountType::Cfd => "CFD",
                AccountType::Physical => "PHYSICAL",
                AccountType::Spreadbet => "SPREADBET",
            })
            .collect();
        let account_name: Vec<&str> = self.iter().map(|a| a.account_name.as_str()).collect();
        let can_transfer_to_ma: Vec<Option<bool>> =
            self.iter().map(|a| a.can_transfer_to_ma).collect();
        let can_transfer_from_ma: Vec<Option<bool>> =
            self.iter().map(|a| a.can_transfer_from_ma).collect();
        let default_account: Vec<bool> = self.iter().map(|a| a.default_account).collect();
        let preferred: Vec<bool> = self.iter().map(|a| a.preferred).collect();
        let balance: Vec<Option<f64>> = self.iter().map(|a| a.balance.balance).collect();
        let deposit: Vec<Option<f64>> = self.iter().map(|a| a.balance.deposit).collect();
        let profit_loss: Vec<Option<f64>> = self.iter().map(|a| a.balance.profit_loss).collect();
        let available_cash: Vec<Option<f64>> =
            self.iter().map(|a| a.balance.available_cash).collect();

        DataFrame::new(vec![
            Column::new("account_id".into(), account_id),
            Column::new("account_alias".into(), account_alias),
            Column::new("account_type".into(), account_type),
            Column::new("account_name".into(), account_name),
            Column::new("can_transfer_to_ma".into(), can_transfer_to_ma),
            Column::new("can_transfer_from_ma".into(), can_transfer_from_ma),
            Column::new("default_account".into(), default_account),
            Column::new("preferred".into(), preferred),
            Column::new("balance".into(), balance),
            Column::new("deposit".into(), deposit),
            Column::new("profit_loss".into(), profit_loss),
            Column::new("available_cash".into(), available_cash),
        ])
        .map_err(|e| crate::Error::Config(format!("polars conversion failed: {e}")))
    }
}
