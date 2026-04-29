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
