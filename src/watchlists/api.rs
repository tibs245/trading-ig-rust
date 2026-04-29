//! Endpoint implementations for the watchlists domain.

use http::Method;
use serde::Deserialize;

use crate::Result;
use crate::models::common::Epic;

use super::WatchlistsApi;
use super::models::{
    AddMarketResponse, CreateWatchlistRequest, CreateWatchlistResponse, DeleteWatchlistResponse,
    MarketSummary, RemoveMarketResponse, WatchlistSummary,
};

impl WatchlistsApi<'_> {
    /// List all watchlists for the authenticated account.
    ///
    /// Calls `GET /watchlists` (v1).
    #[tracing::instrument(skip_all)]
    pub async fn list(&self) -> Result<Vec<WatchlistSummary>> {
        #[derive(Deserialize)]
        struct Envelope {
            watchlists: Vec<WatchlistSummary>,
        }
        let body: Envelope = self
            .client
            .transport
            .request(
                Method::GET,
                "watchlists",
                Some(1),
                None::<&()>,
                &self.client.session,
            )
            .await?;
        Ok(body.watchlists)
    }

    /// Create a new watchlist with an optional initial set of epics.
    ///
    /// Calls `POST /watchlists` (v1).
    ///
    /// Check [`CreateWatchlistResponse::status`] — a
    /// [`CreateWatchlistStatus::SuccessNotAllInstrumentsAdded`] result is
    /// still HTTP 200 but indicates that one or more requested epics were not
    /// added.
    #[tracing::instrument(skip_all)]
    pub async fn create(&self, req: CreateWatchlistRequest) -> Result<CreateWatchlistResponse> {
        self.client
            .transport
            .request(
                Method::POST,
                "watchlists",
                Some(1),
                Some(&req),
                &self.client.session,
            )
            .await
    }

    /// List the markets (instruments) contained in a watchlist.
    ///
    /// Calls `GET /watchlists/{id}` (v1).
    #[tracing::instrument(skip_all, fields(watchlist_id = %watchlist_id))]
    pub async fn markets(&self, watchlist_id: &str) -> Result<Vec<MarketSummary>> {
        #[derive(Deserialize)]
        struct Envelope {
            markets: Vec<MarketSummary>,
        }
        let path = format!("watchlists/{watchlist_id}");
        let body: Envelope = self
            .client
            .transport
            .request(
                Method::GET,
                &path,
                Some(1),
                None::<&()>,
                &self.client.session,
            )
            .await?;
        Ok(body.markets)
    }

    /// Add a market (instrument) to an existing watchlist.
    ///
    /// Calls `PUT /watchlists/{id}` (v1).
    ///
    /// # Errors
    ///
    /// Returns `Error::Api` if the watchlist is system-defined (`editable:
    /// false`) or if the epic is not recognised by IG.
    #[tracing::instrument(skip_all, fields(watchlist_id = %watchlist_id, epic = %epic))]
    pub async fn add_market(&self, watchlist_id: &str, epic: &Epic) -> Result<AddMarketResponse> {
        #[derive(serde::Serialize)]
        struct Body<'b> {
            epic: &'b Epic,
        }
        let path = format!("watchlists/{watchlist_id}");
        self.client
            .transport
            .request(
                Method::PUT,
                &path,
                Some(1),
                Some(&Body { epic }),
                &self.client.session,
            )
            .await
    }

    /// Remove a market (instrument) from a watchlist.
    ///
    /// Calls `DELETE /watchlists/{id}/{epic}` (v1).
    ///
    /// Epics may contain dots (e.g. `CS.D.GBPUSD.TODAY.IP`); the path is
    /// built with `format!()` and the reqwest URL parser handles any needed
    /// percent-encoding.
    ///
    /// # Errors
    ///
    /// Returns `Error::Api` if the watchlist is system-defined or the epic is
    /// not in the watchlist.
    #[tracing::instrument(skip_all, fields(watchlist_id = %watchlist_id, epic = %epic))]
    pub async fn remove_market(
        &self,
        watchlist_id: &str,
        epic: &Epic,
    ) -> Result<RemoveMarketResponse> {
        let path = format!("watchlists/{watchlist_id}/{epic}");
        self.client
            .transport
            .request(
                Method::DELETE,
                &path,
                Some(1),
                None::<&()>,
                &self.client.session,
            )
            .await
    }

    /// Delete a watchlist entirely.
    ///
    /// Calls `DELETE /watchlists/{id}` (v1).
    ///
    /// # Errors
    ///
    /// Returns `Error::Api` if the watchlist is system-defined (`deleteable:
    /// false`).
    #[tracing::instrument(skip_all, fields(watchlist_id = %watchlist_id))]
    pub async fn delete(&self, watchlist_id: &str) -> Result<DeleteWatchlistResponse> {
        let path = format!("watchlists/{watchlist_id}");
        self.client
            .transport
            .request(
                Method::DELETE,
                &path,
                Some(1),
                None::<&()>,
                &self.client.session,
            )
            .await
    }
}
