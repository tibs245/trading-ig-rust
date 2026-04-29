//! Async Rust client for the IG Markets REST and Lightstreamer streaming APIs.
//!
//! See the project [`README`] and [`docs/API_CATALOG.md`] for an overview of
//! the surface area being ported.
//!
//! [`README`]: https://github.com/ig-python/trading-ig-rust
//! [`docs/API_CATALOG.md`]: https://github.com/ig-python/trading-ig-rust/blob/main/docs/API_CATALOG.md

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod client;
pub mod config;
pub mod error;
pub mod markets;
pub mod models;
pub mod prices;
pub mod session;
pub mod time;

pub use client::{IgClient, IgClientBuilder};
pub use config::{Environment, IgConfig};
pub use error::{ApiError, Error, Result};
pub use session::{Credentials, SessionInfo};
