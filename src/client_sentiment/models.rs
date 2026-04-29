//! Types for the client sentiment domain.

use serde::{Deserialize, Serialize};

/// Client sentiment for a single market — the proportion of IG clients
/// currently long vs short.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sentiment {
    /// The IG market identifier (e.g. `"CC.D.LCO.UNC.IP"`).
    ///
    /// Note: this is **not** the same as an [`crate::models::Epic`]. It is
    /// the umbrella market identifier used by the sentiment API.
    pub market_id: String,
    /// Percentage of clients currently holding a long position (0–100).
    pub long_position_percentage: f64,
    /// Percentage of clients currently holding a short position (0–100).
    pub short_position_percentage: f64,
}

// ---------------------------------------------------------------------------
// Polars conversion
// ---------------------------------------------------------------------------

#[cfg(feature = "polars")]
impl crate::dataframe::IntoDataFrame for Vec<Sentiment> {
    /// Convert a list of sentiment records into a `polars::prelude::DataFrame`.
    ///
    /// Column layout:
    ///
    /// | column                      | dtype     | nullable |
    /// | --------------------------- | --------- | -------- |
    /// | `market_id`                 | `Utf8`    | no       |
    /// | `long_position_percentage`  | `Float64` | no       |
    /// | `short_position_percentage` | `Float64` | no       |
    fn to_dataframe(&self) -> crate::Result<polars::prelude::DataFrame> {
        use polars::prelude::*;

        let market_id: Vec<&str> = self.iter().map(|s| s.market_id.as_str()).collect();
        let long_position_percentage: Vec<f64> =
            self.iter().map(|s| s.long_position_percentage).collect();
        let short_position_percentage: Vec<f64> =
            self.iter().map(|s| s.short_position_percentage).collect();

        DataFrame::new(vec![
            Column::new("market_id".into(), market_id),
            Column::new("long_position_percentage".into(), long_position_percentage),
            Column::new(
                "short_position_percentage".into(),
                short_position_percentage,
            ),
        ])
        .map_err(|e| crate::Error::Config(format!("polars conversion failed: {e}")))
    }
}
