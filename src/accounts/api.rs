//! Endpoint implementations for the accounts domain.

use http::Method;
use serde::Deserialize;
use tracing::instrument;

use crate::Result;

use super::AccountsApi;
use super::models::{Account, AccountPreferences, UpdatePreferences};

impl AccountsApi<'_> {
    /// List all accounts associated with the authenticated client.
    ///
    /// Calls `GET /accounts` (v1).
    ///
    /// # Errors
    ///
    /// Returns `Error::Auth` if there is no active session, or `Error::Api`
    /// if IG returns a non-2xx response.
    #[instrument(skip_all)]
    pub async fn list(&self) -> Result<Vec<Account>> {
        #[derive(Deserialize)]
        struct Envelope {
            accounts: Vec<Account>,
        }
        let envelope: Envelope = self
            .client
            .transport
            .request(Method::GET, "accounts", Some(1), None::<&()>, &self.client.session)
            .await?;
        Ok(envelope.accounts)
    }

    /// Retrieve the current trading preferences for the active account.
    ///
    /// Calls `GET /accounts/preferences` (v1).
    ///
    /// # Errors
    ///
    /// Returns `Error::Auth` if there is no active session, or `Error::Api`
    /// if IG returns a non-2xx response.
    #[instrument(skip_all)]
    pub async fn preferences(&self) -> Result<AccountPreferences> {
        self.client
            .transport
            .request(
                Method::GET,
                "accounts/preferences",
                Some(1),
                None::<&()>,
                &self.client.session,
            )
            .await
    }

    /// Update the trading preferences for the active account.
    ///
    /// Calls `PUT /accounts/preferences` (v1). IG expects the boolean to be
    /// serialised as a string (`"true"` / `"false"`); this method handles that
    /// conversion transparently.
    ///
    /// # Errors
    ///
    /// Returns `Error::Auth` if there is no active session, or `Error::Api`
    /// if IG returns a non-2xx response.
    #[instrument(skip_all, fields(trailing_stops_enabled = %req.trailing_stops_enabled))]
    pub async fn update_preferences(&self, req: UpdatePreferences) -> Result<AccountPreferences> {
        let wire = req.to_wire();
        self.client
            .transport
            .request(
                Method::PUT,
                "accounts/preferences",
                Some(1),
                Some(&wire),
                &self.client.session,
            )
            .await
    }
}
