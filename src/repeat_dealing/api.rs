//! Repeat dealing API endpoint.

use http::Method;
use serde::Deserialize;
use tracing::{instrument, warn};

use crate::Result;
use crate::client::IgClient;
use crate::models::Epic;

use super::models::RepeatDealingWindow;

/// Typed accessor for the `/repeat-dealing-window` endpoint.
///
/// Obtain via [`IgClient::repeat_dealing`].
#[derive(Debug)]
pub struct RepeatDealingApi<'a> {
    pub(crate) client: &'a IgClient,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WindowEnvelope {
    repeat_dealing_windows: Vec<RepeatDealingWindow>,
}

/// Maximum number of attempts for the repeat-dealing retry loop.
const MAX_ATTEMPTS: u8 = 5;
/// Delay between retry attempts (mirrors the Python library's 1-second back-off).
const RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(1);

impl RepeatDealingApi<'_> {
    /// Fetch all active repeat-dealing windows.
    ///
    /// Retries up to 5 times with a 1-second delay on transient errors,
    /// mirroring the Python library's behaviour.
    #[instrument(skip(self))]
    pub async fn window(&self) -> Result<Vec<RepeatDealingWindow>> {
        self.fetch_with_retry("repeat-dealing-window").await
    }

    /// Fetch the repeat-dealing window for a specific epic.
    ///
    /// Retries up to 5 times with a 1-second delay on transient errors,
    /// mirroring the Python library's behaviour.
    #[instrument(skip(self), fields(epic = %epic))]
    pub async fn window_for(&self, epic: &Epic) -> Result<Vec<RepeatDealingWindow>> {
        let path = format!("repeat-dealing-window?epic={epic}");
        self.fetch_with_retry(&path).await
    }

    /// Perform the request with up to [`MAX_ATTEMPTS`] retries on failure.
    async fn fetch_with_retry(&self, path: &str) -> Result<Vec<RepeatDealingWindow>> {
        let mut last_err = None;
        for attempt in 1..=MAX_ATTEMPTS {
            match self
                .client
                .transport
                .request::<(), WindowEnvelope>(
                    Method::GET,
                    path,
                    Some(1),
                    None::<&()>,
                    &self.client.session,
                )
                .await
            {
                Ok(envelope) => return Ok(envelope.repeat_dealing_windows),
                Err(e) => {
                    warn!(attempt, "repeat-dealing-window request failed, will retry");
                    last_err = Some(e);
                    if attempt < MAX_ATTEMPTS {
                        tokio::time::sleep(RETRY_DELAY).await;
                    }
                }
            }
        }
        Err(last_err.expect("loop always runs at least once"))
    }
}
