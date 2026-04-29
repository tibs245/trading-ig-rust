//! Dealing domain — working orders (and positions, added post-merge).
//!
//! Obtain a [`DealingApi`] via [`crate::IgClient::dealing`].

pub mod working_orders;

use crate::IgClient;
use working_orders::WorkingOrdersApi;

/// Typed accessor for dealing sub-domains.
///
/// Created via `client.dealing()`.
#[derive(Debug)]
pub struct DealingApi<'a> {
    pub(crate) client: &'a IgClient,
}

impl<'a> DealingApi<'a> {
    /// Access working-order endpoints.
    pub fn working_orders(&self) -> WorkingOrdersApi<'a> {
        WorkingOrdersApi { client: self.client }
    }
}
