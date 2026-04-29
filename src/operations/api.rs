//! Operations API endpoints — manage API application keys.

use http::Method;
use tracing::instrument;

use crate::Result;
use crate::client::IgClient;

use super::models::{Application, UpdateApplicationRequest};

/// Typed accessor for the `/operations/application` endpoints.
///
/// Obtain via [`IgClient::operations`].
#[derive(Debug)]
pub struct OperationsApi<'a> {
    pub(crate) client: &'a IgClient,
}

impl OperationsApi<'_> {
    /// List all API application keys associated with the current account.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Api`] with
    /// `error.public-api.failure.not.an.administrator` if the account does
    /// not have administrator privileges.
    #[instrument(skip(self))]
    pub async fn applications(&self) -> Result<Vec<Application>> {
        self.client
            .transport
            .request(
                Method::GET,
                "operations/application",
                Some(1),
                None::<&()>,
                &self.client.session,
            )
            .await
    }

    /// Update allowances or status for an API application key.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Api`] with
    /// `error.public-api.failure.not.an.administrator` if the account does
    /// not have administrator privileges.
    #[instrument(skip(self, req), fields(api_key = %req.api_key))]
    pub async fn update_application(&self, req: UpdateApplicationRequest) -> Result<Application> {
        self.client
            .transport
            .request(
                Method::PUT,
                "operations/application",
                Some(1),
                Some(&req),
                &self.client.session,
            )
            .await
    }

    /// Disable the API key currently in use.
    ///
    /// Re-enabling the key requires the IG web UI. Use with care.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Api`] with
    /// `error.public-api.failure.not.an.administrator` if the account does
    /// not have administrator privileges.
    #[instrument(skip(self))]
    pub async fn disable_current_key(&self) -> Result<Application> {
        self.client
            .transport
            .request(
                Method::PUT,
                "operations/application/disable",
                Some(1),
                None::<&()>,
                &self.client.session,
            )
            .await
    }
}
