//! Endpoint methods for `dealing/working_orders`.

use http::Method;
use serde::Deserialize;
use tracing::instrument;

use crate::dealing::common::DealConfirmation;
use crate::dealing::working_orders::create_working_order::{
    CreateWorkingOrderBuilder, Missing, fetch_confirmation,
};
use crate::dealing::working_orders::models::{
    UpdateWorkingOrderRequest, WorkingOrderEntryV1Raw, WorkingOrderEntryV2Raw, WorkingOrderV1,
    WorkingOrderV2,
};
use crate::error::Result;
use crate::models::common::DealId;

use super::WorkingOrdersApi;

// ── Envelope types used inside the method bodies ──────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListEnvelopeV1 {
    working_orders: Vec<WorkingOrderEntryV1Raw>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListEnvelopeV2 {
    working_orders: Vec<WorkingOrderEntryV2Raw>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DealReferenceResponse {
    deal_reference: crate::models::common::DealReference,
}

// ── Implementation ────────────────────────────────────────────────────────────

impl WorkingOrdersApi<'_> {
    /// List all working orders using the v1 schema.
    ///
    /// Returns an entry per order with v1-specific fields (`goodTill`,
    /// `controlledRisk`, `contingentLimit`, etc.).
    #[instrument(skip_all, fields(version = 1))]
    pub async fn list_v1(&self) -> Result<Vec<WorkingOrderV1>> {
        let env: ListEnvelopeV1 = self
            .client
            .transport
            .request(
                Method::GET,
                "workingorders",
                Some(1),
                None::<&()>,
                &self.client.session,
            )
            .await?;

        Ok(env
            .working_orders
            .into_iter()
            .map(|e| WorkingOrderV1 {
                order_data: e.working_order_data,
                market: e.market_data,
            })
            .collect())
    }

    /// List all working orders using the v2 schema (canonical version).
    ///
    /// Returns an entry per order with v2-specific fields (`goodTillDate`,
    /// `goodTillDateISO`, `guaranteedStop`, etc.). This is the recommended
    /// version for new code.
    #[instrument(skip_all, fields(version = 2))]
    pub async fn list_v2(&self) -> Result<Vec<WorkingOrderV2>> {
        let env: ListEnvelopeV2 = self
            .client
            .transport
            .request(
                Method::GET,
                "workingorders",
                Some(2),
                None::<&()>,
                &self.client.session,
            )
            .await?;

        Ok(env
            .working_orders
            .into_iter()
            .map(|e| WorkingOrderV2 {
                order_data: e.working_order_data,
                market: e.market_data,
            })
            .collect())
    }

    /// List all working orders — alias for [`Self::list_v2`].
    ///
    /// Matches the Python `trading-ig` default of `version="2"`.
    pub async fn list(&self) -> Result<Vec<WorkingOrderV2>> {
        self.list_v2().await
    }

    /// Begin building a new working order.
    ///
    /// Returns a type-state builder; call the mandatory setters
    /// (`epic`, `direction`, `size`, `order_type`, `currency`, `expiry`,
    /// `level`, `time_in_force`, `guaranteed_stop`) then `.send().await?`.
    ///
    /// After the order is placed the builder automatically polls
    /// `GET /confirms/{ref}` up to 5 times (1 s apart) and returns a
    /// [`DealConfirmation`].
    pub fn create(
        &self,
    ) -> CreateWorkingOrderBuilder<
        '_,
        Missing,
        Missing,
        Missing,
        Missing,
        Missing,
        Missing,
        Missing,
        Missing,
        Missing,
    > {
        CreateWorkingOrderBuilder::new(self.client)
    }

    /// Update an existing working order.
    ///
    /// All fields in `req` are required by the IG API; use `None` for
    /// optional stop/limit values that are not set.
    ///
    /// Returns the [`DealConfirmation`] for the update.
    ///
    /// # Errors
    ///
    /// Returns `Error::Api` if IG rejects the update (e.g. the deal id is
    /// unknown, or a business-rule violation).
    #[instrument(skip_all, fields(deal_id = %deal_id))]
    pub async fn update(
        &self,
        deal_id: &DealId,
        req: UpdateWorkingOrderRequest,
    ) -> Result<DealConfirmation> {
        let path = format!("workingorders/otc/{}", deal_id.as_str());
        let resp: DealReferenceResponse = self
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

        fetch_confirmation(self.client, &resp.deal_reference).await
    }

    /// Delete (cancel) a working order.
    ///
    /// Sends `DELETE /workingorders/otc/{dealId}` (v2) and auto-fetches the
    /// deal confirmation (same 5 × 1 s retry policy as `create`).
    ///
    /// # Errors
    ///
    /// Returns `Error::Api` if IG cannot find or cancel the order.
    #[instrument(skip_all, fields(deal_id = %deal_id))]
    pub async fn delete(&self, deal_id: &DealId) -> Result<DealConfirmation> {
        let path = format!("workingorders/otc/{}", deal_id.as_str());
        let resp: DealReferenceResponse = self
            .client
            .transport
            .request::<(), _>(
                Method::DELETE,
                &path,
                Some(2),
                None::<&()>,
                &self.client.session,
            )
            .await?;

        fetch_confirmation(self.client, &resp.deal_reference).await
    }
}
