//! Endpoint methods for the markets domain.

use http::Method;
use serde::Deserialize;
use tracing::instrument;

use crate::Result;
use crate::models::common::Epic;

use super::MarketsApi;
use super::models::{MarketDetailFilter, MarketDetails, MarketSummary, NavigationNode};

impl MarketsApi<'_> {
    /// Search for markets matching `search_term` (v1).
    ///
    /// Calls `GET /markets?searchTerm=<term>` at version 1.
    /// Returns up to ~30 lightweight [`MarketSummary`] entries.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Api`] when IG rejects the request (e.g.
    /// `error.public-api.failure.not.found`).
    #[instrument(skip_all, fields(search_term = %search_term))]
    pub async fn search(&self, search_term: &str) -> Result<Vec<MarketSummary>> {
        #[derive(Deserialize)]
        struct Envelope {
            markets: Vec<MarketSummary>,
        }

        let encoded = percent_encode(search_term);
        let path = format!("markets?searchTerm={encoded}");
        let envelope: Envelope = self
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
        Ok(envelope.markets)
    }

    /// Fetch full details for multiple markets in one call (v2).
    ///
    /// Calls `GET /markets?epics=<A,B,…>&filter=<filter>` at version 2.
    /// IG accepts at most 50 epics per request.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::InvalidInput`] when the epic list is empty.
    /// Returns [`crate::Error::Api`] for other API failures.
    #[instrument(skip_all, fields(epic_count = epics.len(), filter = ?filter))]
    pub async fn get_many(
        &self,
        epics: &[Epic],
        filter: MarketDetailFilter,
    ) -> Result<Vec<MarketDetails>> {
        if epics.is_empty() {
            return Err(crate::Error::InvalidInput(
                "epics list must not be empty".into(),
            ));
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Envelope {
            market_details: Vec<MarketDetails>,
        }

        let epic_list = epics.iter().map(Epic::as_str).collect::<Vec<_>>().join(",");
        let filter_str = filter.as_query_str();
        let path = format!("markets?epics={epic_list}&filter={filter_str}");

        let envelope: Envelope = self
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
        Ok(envelope.market_details)
    }

    /// Fetch full details for a single market by epic (v3).
    ///
    /// Calls `GET /markets/{epic}` at version 3.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Api`] with error code
    /// `error.public-api.failure.market.not-found` when the epic does not
    /// exist.
    #[instrument(skip_all, fields(epic = %epic))]
    pub async fn get(&self, epic: &Epic) -> Result<MarketDetails> {
        let path = format!("markets/{epic}");
        self.client
            .transport
            .request(
                Method::GET,
                &path,
                Some(3),
                None::<&()>,
                &self.client.session,
            )
            .await
    }

    /// Fetch the top-level market navigation hierarchy (v1).
    ///
    /// Calls `GET /marketnavigation` at version 1.
    /// The returned [`NavigationNode`] typically contains only `nodes`
    /// (child categories), not markets.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Api`] for API-level failures.
    #[instrument(skip_all)]
    pub async fn navigation(&self) -> Result<NavigationNode> {
        self.client
            .transport
            .request(
                Method::GET,
                "marketnavigation",
                Some(1),
                None::<&()>,
                &self.client.session,
            )
            .await
    }

    /// Fetch a sub-node in the market navigation hierarchy (v1).
    ///
    /// Calls `GET /marketnavigation/{node_id}` at version 1.
    /// Leaf nodes typically contain markets but no further child nodes.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Api`] for API-level failures.
    #[instrument(skip_all, fields(node_id = %node_id))]
    pub async fn navigation_node(&self, node_id: &str) -> Result<NavigationNode> {
        let path = format!("marketnavigation/{node_id}");
        self.client
            .transport
            .request(
                Method::GET,
                &path,
                Some(1),
                None::<&()>,
                &self.client.session,
            )
            .await
    }
}

/// Percent-encode a query-parameter value (RFC 3986 unreserved chars are safe).
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            other => {
                use std::fmt::Write as _;
                let _ = write!(out, "%{other:02X}");
            }
        }
    }
    out
}
