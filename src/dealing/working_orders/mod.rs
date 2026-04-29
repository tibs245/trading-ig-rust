//! Working-orders domain: list (v1/v2), create (type-state builder),
//! update, and delete.
//!
//! Obtain an instance via [`crate::IgClient::dealing`] → [`super::DealingApi::working_orders`].

pub mod api;
pub mod create_working_order;
pub mod models;

pub use models::{
    DealConfirmation, DealStatus, MarketSnapshot, UpdateWorkingOrderRequest, WorkingOrderDataV1,
    WorkingOrderDataV2, WorkingOrderV1, WorkingOrderV2,
};

use crate::IgClient;

/// Typed accessor for the working-orders endpoints.
///
/// Created via `client.dealing().working_orders()`.
#[derive(Debug)]
pub struct WorkingOrdersApi<'a> {
    pub(crate) client: &'a IgClient,
}
