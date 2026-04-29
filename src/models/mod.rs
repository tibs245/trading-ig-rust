//! Shared types used across multiple domain modules.
//!
//! Domain-specific request/response structs live with their domain
//! (`accounts/models.rs`, `markets/models.rs`, …). Anything reused by two or
//! more domains belongs here.

pub mod common;

pub use common::{Currency, DealId, DealReference, Direction, Epic, InstrumentType, MarketSnapshot, MarketStatus, OrderType, TimeInForce};
