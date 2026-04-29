//! Endpoint methods for the `dealing/positions` domain.

use http::Method;
use tracing::instrument;

use crate::client::IgClient;
use crate::dealing::common::DealConfirmation;
use crate::dealing::positions::models::{
    ClosePositionRequest, ClosePositionResponse, PositionV1, PositionV2, PositionsEnvelopeV1,
    PositionsEnvelopeV2, UpdatePositionRequest, UpdatePositionResponse,
};
use crate::dealing::positions::open_position::{Missing, OpenPositionBuilder};
use crate::error::Result;
use crate::models::common::DealReference;

/// Typed accessor for `dealing/positions` endpoints.
///
/// Obtain via [`crate::dealing::DealingApi::positions`].
#[derive(Debug)]
pub struct PositionsApi<'a> {
    pub(crate) client: &'a IgClient,
}

impl<'a> PositionsApi<'a> {
    // -----------------------------------------------------------------------
    // List endpoints
    // -----------------------------------------------------------------------

    /// List all open positions — v1 response shape.
    ///
    /// `GET /positions` with `Version: 1`.
    #[instrument(skip_all, name = "dealing.positions.list_v1")]
    pub async fn list_v1(&self) -> Result<Vec<PositionV1>> {
        let env: PositionsEnvelopeV1 = self
            .client
            .transport
            .request(
                Method::GET,
                "positions",
                Some(1),
                None::<&()>,
                &self.client.session,
            )
            .await?;
        Ok(env.into_vec())
    }

    /// List all open positions — v2 response shape (recommended).
    ///
    /// `GET /positions` with `Version: 2`.
    #[instrument(skip_all, name = "dealing.positions.list_v2")]
    pub async fn list_v2(&self) -> Result<Vec<PositionV2>> {
        let env: PositionsEnvelopeV2 = self
            .client
            .transport
            .request(
                Method::GET,
                "positions",
                Some(2),
                None::<&()>,
                &self.client.session,
            )
            .await?;
        Ok(env.into_vec())
    }

    /// Alias for [`Self::list_v2`] — the canonical recent version.
    pub async fn list(&self) -> Result<Vec<PositionV2>> {
        self.list_v2().await
    }

    // -----------------------------------------------------------------------
    // Single position
    // -----------------------------------------------------------------------

    /// Fetch a single open position by its `dealId`.
    ///
    /// `GET /positions/{dealId}` with `Version: 2`.
    #[instrument(skip_all, name = "dealing.positions.get", fields(deal_id = %deal_id))]
    pub async fn get(&self, deal_id: impl AsRef<str> + std::fmt::Display) -> Result<PositionV2> {
        let path = format!("positions/{}", deal_id.as_ref());
        let env: PositionsEnvelopeV2 = self
            .client
            .transport
            .request(
                Method::GET,
                &path,
                Some(2),
                None::<&()>,
                &self.client.session,
            )
            .await?;
        // Single-position endpoint wraps in the same envelope shape as the
        // list but with exactly one entry. If the server ever returns zero
        // entries for a direct deal-id lookup we surface that as an API error
        // rather than panicking.
        env.into_vec().into_iter().next().ok_or_else(|| {
            crate::error::Error::InvalidInput(
                "positions/{dealId} returned an empty positions array".into(),
            )
        })
    }

    // -----------------------------------------------------------------------
    // Open (type-state builder)
    // -----------------------------------------------------------------------

    /// Begin constructing an open-position request.
    ///
    /// Returns an [`OpenPositionBuilder`] with all mandatory fields unset.
    /// Supply all mandatory fields (`currency`, `direction`, `epic`, `expiry`,
    /// `guaranteed_stop`, `order_type`, `size` — in any order) before calling
    /// `.send()`. The compiler enforces this at compile time.
    ///
    /// `POST /positions/otc` with `Version: 2`, followed by auto-fetching
    /// `GET /confirms/{dealReference}`.
    pub fn open(
        &'a self,
    ) -> OpenPositionBuilder<'a, Missing, Missing, Missing, Missing, Missing, Missing, Missing>
    {
        OpenPositionBuilder::new(self)
    }

    // -----------------------------------------------------------------------
    // Update
    // -----------------------------------------------------------------------

    /// Update an existing open position.
    ///
    /// `PUT /positions/otc/{dealId}` with `Version: 2`.
    /// Returns a [`DealConfirmation`] fetched after the server assigns a
    /// `dealReference`.
    #[instrument(skip_all, name = "dealing.positions.update", fields(deal_id = %deal_id))]
    pub async fn update(
        &self,
        deal_id: impl AsRef<str> + std::fmt::Display,
        req: UpdatePositionRequest,
    ) -> Result<DealConfirmation> {
        let path = format!("positions/otc/{}", deal_id.as_ref());
        let resp: UpdatePositionResponse = self
            .client
            .transport
            .request(
                Method::PUT,
                &path,
                Some(2),
                Some(&req),
                &self.client.session,
            )
            .await?;
        self.confirm(&resp.deal_reference).await
    }

    // -----------------------------------------------------------------------
    // Close
    // -----------------------------------------------------------------------

    /// Close an open position.
    ///
    /// `DELETE /positions/otc` with `Version: 1` (body included in the
    /// DELETE request — reqwest supports this natively).
    /// Returns a [`DealConfirmation`] fetched after the server assigns a
    /// `dealReference`.
    #[instrument(skip_all, name = "dealing.positions.close")]
    pub async fn close(&self, req: ClosePositionRequest) -> Result<DealConfirmation> {
        let resp: ClosePositionResponse = self
            .client
            .transport
            .request(
                Method::DELETE,
                "positions/otc",
                Some(1),
                Some(&req),
                &self.client.session,
            )
            .await?;
        self.confirm(&resp.deal_reference).await
    }

    // -----------------------------------------------------------------------
    // Confirm (with retry)
    // -----------------------------------------------------------------------

    /// Fetch the deal confirmation for a given `dealReference`.
    ///
    /// `GET /confirms/{dealReference}` with `Version: 1`.
    ///
    /// Mirrors the Python `trading-ig` retry behaviour: attempts up to **5×**
    /// with a **1 s back-off** between attempts. The confirmation endpoint
    /// sometimes returns a non-200 immediately after a deal is submitted;
    /// the retries accommodate this propagation delay.
    ///
    /// # Errors
    ///
    /// Returns the last error encountered after all 5 attempts have been
    /// exhausted.
    #[instrument(skip_all, name = "dealing.positions.confirm", fields(deal_reference = %deal_reference))]
    pub async fn confirm(&self, deal_reference: &DealReference) -> Result<DealConfirmation> {
        let path = format!("confirms/{}", deal_reference.as_str());
        let mut last_err: Option<crate::error::Error> = None;

        for attempt in 0..5u8 {
            if attempt > 0 {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
            match self
                .client
                .transport
                .request::<(), DealConfirmation>(
                    Method::GET,
                    &path,
                    Some(1),
                    None::<&()>,
                    &self.client.session,
                )
                .await
            {
                Ok(c) => return Ok(c),
                Err(e) => {
                    tracing::warn!(
                        attempt,
                        error = %e,
                        "deal confirmation not yet available, will retry"
                    );
                    last_err = Some(e);
                }
            }
        }

        Err(last_err.unwrap_or_else(|| {
            crate::error::Error::InvalidInput(
                "confirm: exhausted retries with no error recorded".into(),
            )
        }))
    }
}
