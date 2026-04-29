//! Types for the operations (application management) domain.

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// Status of an API application key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApplicationStatus {
    /// The API key is active and can be used.
    Enabled,
    /// The API key has been disabled. Re-enable via the IG web UI.
    Disabled,
    /// The API key has been revoked and cannot be re-enabled.
    Revoked,
}

/// Metadata about an IG API application (key).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Application {
    /// Human-readable name of the application.
    pub name: String,
    /// The API key string.
    pub api_key: String,
    /// Current status of the key.
    pub status: ApplicationStatus,
    /// Non-trading requests per minute allowance for the account.
    pub allowance_account_overall: u32,
    /// Trading requests per minute allowance for the account.
    pub allowance_account_trading: u32,
    /// Non-trading requests per minute allowance for this application.
    pub allowance_application_overall: u32,
    /// Maximum number of concurrent Lightstreamer subscriptions.
    pub concurrent_subscriptions_limit: u32,
    /// Whether equity trading is permitted with this key.
    pub allow_equities: bool,
    /// Whether quote orders are permitted with this key.
    pub allow_quote_orders: bool,
    /// When the application/key was created.
    pub created_date: NaiveDateTime,
}

/// Request body for [`crate::operations::OperationsApi::update_application`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateApplicationRequest {
    /// The API key to update.
    pub api_key: String,
    /// New desired status.
    pub status: ApplicationStatus,
    /// New non-trading per-minute allowance for the account.
    pub allowance_account_overall: u32,
    /// New trading per-minute allowance for the account.
    pub allowance_account_trading: u32,
}
