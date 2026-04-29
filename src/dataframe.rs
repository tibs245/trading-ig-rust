//! Conversion of tabular API responses into Polars `DataFrame`s.
//!
//! This module is only compiled when the `polars` cargo feature is enabled.
//!
//! # Usage
//!
//! ```rust,ignore
//! use trading_ig::dataframe::IntoDataFrame;
//!
//! let positions = client.dealing().positions().list_v2().await?;
//! let df = positions.to_dataframe()?;
//! println!("{df}");
//! ```
#![cfg(feature = "polars")]

use polars::prelude::*;

use crate::Result;

/// Convert a tabular IG API response into a Polars `DataFrame`.
///
/// Implement this trait on collection types (e.g. `Vec<Account>`,
/// `HistoricalPrices`) to enable one-step conversion to a
/// Polars `DataFrame` for further analysis.
pub trait IntoDataFrame {
    /// Convert this collection into a Polars `DataFrame`.
    ///
    /// Returns `Err` if the column lengths are mismatched (should not happen
    /// with a correct implementation) or if an internal polars error occurs.
    fn to_dataframe(&self) -> Result<DataFrame>;
}
