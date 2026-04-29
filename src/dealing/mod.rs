//! Dealing domain — positions and working orders.
//!
//! Entry point: [`DealingApi`], obtained via `client.dealing()`.

pub mod common;
pub mod positions;
pub mod working_orders;

pub use common::{DealConfirmation, DealStatus};

use crate::client::IgClient;
use positions::PositionsApi;
use working_orders::WorkingOrdersApi;

/// Typed accessor for all dealing endpoints.
///
/// Obtain via [`crate::IgClient::dealing`].
#[derive(Debug)]
pub struct DealingApi<'a> {
    pub(crate) client: &'a IgClient,
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

    /// Access working-order endpoints.
    pub fn working_orders(&self) -> WorkingOrdersApi<'a> {
        WorkingOrdersApi { client: self.client }
    }
}
