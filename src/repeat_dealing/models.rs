//! Types for the repeat dealing domain.

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::models::Epic;

/// The time window during which a recently traded instrument can be
/// re-traded with the same dealing characteristics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepeatDealingWindow {
    /// The instrument epic.
    pub epic: Epic,
    /// Start of the repeat-dealing window (UTC).
    pub valid_from: NaiveDateTime,
    /// End of the repeat-dealing window (UTC).
    pub valid_to: NaiveDateTime,
}
