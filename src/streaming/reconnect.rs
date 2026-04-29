//! Auto-reconnect policy for the Lightstreamer streaming client.
//!
//! Configure via [`StreamingApi::connect_with`] to control how the streaming
//! client reacts when the server sends `END` or the connection drops
//! unrecoverably.

use std::time::Duration;

// ---------------------------------------------------------------------------
// AutoReconnect policy
// ---------------------------------------------------------------------------

/// Policy that controls whether and how the streaming client reconnects after
/// a server-side `END` or an unrecoverable connection error.
///
/// Pass an `AutoReconnect` value to [`crate::streaming::client::StreamingApi::connect_with`].
/// The [`Default`] implementation enables auto-reconnect with sensible defaults.
///
/// # Example
///
/// ```
/// # use trading_ig::streaming::AutoReconnect;
/// # use std::time::Duration;
/// let policy = AutoReconnect {
///     enabled: true,
///     max_attempts: Some(3),
///     initial_backoff: Duration::from_secs(2),
///     max_backoff: Duration::from_mins(1),
///     backoff_multiplier: 2.0,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct AutoReconnect {
    /// Whether to attempt a full reconnect on `END` or unrecoverable error.
    ///
    /// When `false` the read-loop exits and all subscriber channels close,
    /// matching the pre-reconnect behaviour.
    pub enabled: bool,

    /// Maximum number of consecutive reconnect attempts before giving up.
    ///
    /// `None` means retry indefinitely.
    pub max_attempts: Option<u32>,

    /// Back-off delay before the *first* reconnect attempt.
    pub initial_backoff: Duration,

    /// Upper bound on the back-off delay (after exponential growth).
    pub max_backoff: Duration,

    /// Multiplier applied to the back-off after each failed attempt.
    ///
    /// A value of `2.0` gives classic exponential back-off; `1.0` gives a
    /// constant delay.
    pub backoff_multiplier: f64,
}

impl Default for AutoReconnect {
    /// Enabled, 5 attempts, 1 s initial back-off growing to 30 s × 2.
    fn default() -> Self {
        Self {
            enabled: true,
            max_attempts: Some(5),
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }
}

impl AutoReconnect {
    /// Compute the back-off for attempt `n` (1-based).
    ///
    /// Returns `initial_backoff * multiplier^(n-1)`, capped at `max_backoff`.
    pub(crate) fn backoff_for_attempt(&self, attempt: u32) -> Duration {
        // Use f64 arithmetic to avoid overflow on large attempt counts.
        let base_secs = self.initial_backoff.as_secs_f64();
        let factor = self
            .backoff_multiplier
            .powi(attempt.saturating_sub(1).cast_signed());
        let raw = base_secs * factor;
        let capped = raw.min(self.max_backoff.as_secs_f64());
        Duration::from_secs_f64(capped)
    }
}

// ---------------------------------------------------------------------------
// StreamingEvent — lifecycle events emitted on the optional event channel
// ---------------------------------------------------------------------------

/// Lifecycle events emitted on the channel returned by
/// [`crate::streaming::client::StreamingApi::connect_with`].
///
/// Users can `select!` on this channel alongside their subscription receivers
/// to react to reconnect success and final failure.
#[derive(Debug, Clone)]
pub enum StreamingEvent {
    /// The streaming client successfully reconnected after a session loss.
    Reconnected {
        /// 1-based attempt number that succeeded.
        attempt: u32,
    },
    /// All reconnect attempts were exhausted; the streaming session is gone.
    ReconnectFailed {
        /// Total attempts made.
        attempts: u32,
        /// Human-readable description of the final error.
        error: String,
    },
    /// The streaming session was cleanly terminated (either by calling
    /// [`crate::streaming::StreamingClient::disconnect`] or by `END` with
    /// `AutoReconnect::enabled = false`).
    Disconnected {
        /// Server-supplied reason, if any.
        reason: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_is_sensible() {
        let p = AutoReconnect::default();
        assert!(p.enabled);
        assert_eq!(p.max_attempts, Some(5));
        assert_eq!(p.initial_backoff, Duration::from_secs(1));
        assert_eq!(p.max_backoff, Duration::from_secs(30));
        assert!((p.backoff_multiplier - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn backoff_grows_and_caps() {
        let p = AutoReconnect {
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            ..AutoReconnect::default()
        };
        // attempt 1 → 1 s
        assert_eq!(p.backoff_for_attempt(1), Duration::from_secs(1));
        // attempt 2 → 2 s
        assert_eq!(p.backoff_for_attempt(2), Duration::from_secs(2));
        // attempt 3 → 4 s
        assert_eq!(p.backoff_for_attempt(3), Duration::from_secs(4));
        // attempt 4 → 8 s
        assert_eq!(p.backoff_for_attempt(4), Duration::from_secs(8));
        // attempt 5 → 16 s
        assert_eq!(p.backoff_for_attempt(5), Duration::from_secs(16));
        // attempt 6 → 32 s, capped to 30 s
        assert_eq!(p.backoff_for_attempt(6), Duration::from_secs(30));
        // further attempts stay capped
        assert_eq!(p.backoff_for_attempt(10), Duration::from_secs(30));
    }

    #[test]
    fn disabled_policy_compiles() {
        let p = AutoReconnect {
            enabled: false,
            ..AutoReconnect::default()
        };
        assert!(!p.enabled);
    }
}
