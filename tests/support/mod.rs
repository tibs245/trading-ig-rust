//! Reusable test helpers shared across integration tests.
//!
//! - [`mock_server::IgMockServer`] wraps `wiremock::MockServer` and exposes a
//!   builder-style API for mounting IG-shaped responses from JSON fixtures.
//! - [`fixtures::load`] reads a fixture from `tests/fixtures/json/`.
//! - [`matchers`] provides reusable wiremock matchers for IG headers
//!   (X-IG-API-KEY, Version, CST, Authorization).

#![allow(dead_code)] // helpers used selectively per test file

pub mod fixtures;
pub mod matchers;
pub mod mock_server;
