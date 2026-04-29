//! Client sentiment API endpoints.

use http::Method;
use serde::Deserialize;
use tracing::instrument;

use crate::Result;
use crate::client::IgClient;

use super::models::Sentiment;

/// Typed accessor for the `/clientsentiment` endpoints.
///
/// Obtain via [`IgClient::client_sentiment`].
#[derive(Debug)]
pub struct ClientSentimentApi<'a> {
    pub(crate) client: &'a IgClient,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SentimentEnvelope {
    client_sentiments: Vec<Sentiment>,
}

impl ClientSentimentApi<'_> {
    /// Fetch client sentiment for a single market.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Api`] with `errorCode`
    /// `error.public-api.failure.market-not-found` if the market ID does not
    /// exist.
    #[instrument(skip(self), fields(market_id = %market_id))]
    pub async fn get(&self, market_id: &str) -> Result<Sentiment> {
        let path = format!("clientsentiment/{market_id}");
        self.client
            .transport
            .request::<(), Sentiment>(
                Method::GET,
                &path,
                Some(1),
                None::<&()>,
                &self.client.session,
            )
            .await
    }

    /// Fetch client sentiment for multiple markets in a single call.
    ///
    /// `market_ids` should be a slice of market identifiers (e.g.
    /// `["CC.D.LCO.UNC.IP", "IX.D.DAX.IFD.IP"]`).
    #[instrument(skip(self), fields(count = market_ids.len()))]
    pub async fn get_many(&self, market_ids: &[&str]) -> Result<Vec<Sentiment>> {
        let ids = market_ids.join(",");
        let path = format!("clientsentiment?marketIds={ids}");
        let envelope: SentimentEnvelope = self
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
        Ok(envelope.client_sentiments)
    }

    /// Fetch client sentiment for markets related to the given market.
    ///
    /// Returns sentiment for correlated instruments that IG considers
    /// "related" to `market_id`.
    #[instrument(skip(self), fields(market_id = %market_id))]
    pub async fn related(&self, market_id: &str) -> Result<Vec<Sentiment>> {
        let path = format!("clientsentiment/related/{market_id}");
        let envelope: SentimentEnvelope = self
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
        Ok(envelope.client_sentiments)
    }
}
