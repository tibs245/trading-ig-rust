//! Dealing domain — positions and working orders.
//!
//! Entry point: [`DealingApi`], obtained via `client.dealing()`.

pub mod positions;

use crate::client::IgClient;
use positions::PositionsApi;

/// Typed accessor for all dealing endpoints.
///
/// Obtain via [`crate::IgClient::dealing`].
#[derive(Debug)]
pub struct DealingApi<'a> {
    client: &'a IgClient,
}

impl<'a> DealingApi<'a> {
    pub(crate) fn new(client: &'a IgClient) -> Self {
        Self { client }
    }

    /// Access the open-positions sub-API.
    pub fn positions(&self) -> PositionsApi<'_> {
        PositionsApi {
            client: self.client,
        }
    }
}
