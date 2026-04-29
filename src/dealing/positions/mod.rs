//! Open positions domain (`/positions`, `/positions/otc`, `/confirms`).
//!
//! Entry point: [`PositionsApi`], obtained via
//! `client.dealing().positions()`.

pub mod api;
pub mod models;
pub(super) mod open_position;

pub use api::PositionsApi;
pub use models::{
    ClosePositionRequest, DealConfirmation, DealStatus, MarketSnapshot, PositionV1, PositionV2,
    UpdatePositionRequest,
};
pub use open_position::{MissingMandatory, OpenPositionBuilder, WithMandatories};
