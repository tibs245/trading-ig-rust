//! High-level streaming client returned by [`crate::IgClient::streaming`].
//!
//! Obtain a [`StreamingApi`] from [`crate::IgClient::streaming`], then call
//! [`StreamingApi::connect`] to open a Lightstreamer session and receive a
//! [`StreamingClient`].
//!
//! # Example
//!
//! ```no_run
//! # use trading_ig::{IgClient, Environment, Credentials};
//! # async fn run() -> trading_ig::Result<()> {
//! # let client = IgClient::builder()
//! #     .environment(Environment::Demo)
//! #     .api_key("key")
//! #     .credentials(Credentials::password("u", "p"))
//! #     .build()?;
//! client.session().login_v2().await?;
//! let stream = client.streaming().connect().await?;
//! let mut rx = stream.subscribe_market("CS.D.GBPUSD.TODAY.IP").await?;
//! while let Some(update) = rx.recv().await {
//!     println!("{} bid={:?}", update.epic, update.bid);
//! }
//! # Ok(()) }
//! ```

use tokio::sync::{mpsc, watch};
use tracing::instrument;

use crate::IgClient;
use crate::error::Result;
use crate::session::AuthTokens;
use crate::streaming::connection::LsConnection;
use crate::streaming::events::{
    AccountUpdate, CandleScale, ChartCandleUpdate, ChartTickUpdate, MarketUpdate, TradeUpdate,
};
use crate::streaming::subscription::{Registry, SubscriptionKind};

/// Subscription channel capacity.  Keep modest so a slow consumer causes
/// back-pressure rather than unbounded queue growth.
const CHANNEL_CAP: usize = 256;

// ---------------------------------------------------------------------------
// StreamingApi — accessor on IgClient
// ---------------------------------------------------------------------------

/// Entry point for streaming.  Obtain via [`crate::IgClient::streaming`].
#[derive(Debug)]
pub struct StreamingApi<'a> {
    pub(crate) client: &'a IgClient,
}

impl StreamingApi<'_> {
    /// Connect to the Lightstreamer streaming endpoint.
    ///
    /// The underlying session must already be authenticated before calling
    /// this method.  For a **v3 (OAuth)** session, call
    /// `client.session().read(true).await?` first so that CST/XST tokens
    /// are stored locally — Lightstreamer requires the
    /// `CST-<cst>|XST-<xst>` password format regardless of auth flavour.
    ///
    /// Returns a [`StreamingClient`] from which subscriptions can be made.
    #[instrument(skip_all, name = "streaming.connect")]
    pub async fn connect(&self) -> Result<StreamingClient> {
        let state = self.client.session.require_authenticated().await?;

        let account_id = state.account_id.ok_or_else(|| {
            crate::error::Error::Auth(
                "no account ID in session — call session().login() first".into(),
            )
        })?;
        let endpoint = state.lightstreamer_endpoint.ok_or_else(|| {
            crate::error::Error::Auth(
                "no Lightstreamer endpoint in session — call session().login() first".into(),
            )
        })?;

        // Build Lightstreamer password from CST/XST tokens.
        // OAuth sessions need CST/XST — caller must call read(true) first to
        // exchange the OAuth token for a CST/XST pair.
        let password = match state.tokens.as_ref() {
            Some(AuthTokens::Cst {
                cst,
                x_security_token,
            }) => format!("CST-{cst}|XST-{x_security_token}"),
            Some(AuthTokens::OAuth { .. }) => {
                return Err(crate::error::Error::Auth(
                    "OAuth session cannot be used directly for streaming. \
                     Call client.session().read(true).await? first to obtain \
                     CST/XST tokens, then call streaming().connect() again."
                        .into(),
                ));
            }
            None => return Err(crate::error::Error::Auth("no active session tokens".into())),
        };

        let registry = Registry::new();
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);

        let conn = LsConnection::create(
            &endpoint,
            &account_id,
            &password,
            registry.clone(),
            shutdown_tx.clone(),
        )
        .await?;

        Ok(StreamingClient {
            conn,
            registry,
            shutdown_tx,
        })
    }
}

// ---------------------------------------------------------------------------
// StreamingClient
// ---------------------------------------------------------------------------

/// A live Lightstreamer session with active subscriptions.
///
/// Obtained via [`StreamingApi::connect`].  All subscription methods return a
/// `tokio::sync::mpsc::Receiver<T>`.  Dropping the receiver automatically
/// cancels the subscription server-side the next time the server sends an
/// update for that item.
///
/// Call [`StreamingClient::disconnect`] to cleanly tear down the session.
#[derive(Debug)]
pub struct StreamingClient {
    conn: LsConnection,
    registry: Registry,
    shutdown_tx: watch::Sender<bool>,
}

impl StreamingClient {
    // ------------------------------------------------------------------
    // Market
    // ------------------------------------------------------------------

    /// Subscribe to market price updates for `epic`.
    ///
    /// Returns a `Receiver<MarketUpdate>`.  Each received value is a snapshot
    /// of all changed fields merged with the previous state — no field is
    /// ever "missing".
    #[instrument(skip(self), fields(%epic))]
    pub async fn subscribe_market(&self, epic: &str) -> Result<mpsc::Receiver<MarketUpdate>> {
        use crate::streaming::events::MARKET_FIELDS;
        let (tx, rx) = mpsc::channel(CHANNEL_CAP);
        let idx = self.registry.register(SubscriptionKind::Market {
            epic: epic.to_owned(),
            tx,
        });
        let item = format!("MARKET:{epic}");
        let fields = MARKET_FIELDS.join(" ");
        self.conn
            .control("add", idx, &item, &fields, "MERGE")
            .await?;
        Ok(rx)
    }

    // ------------------------------------------------------------------
    // Chart tick
    // ------------------------------------------------------------------

    /// Subscribe to chart tick data for `epic`.
    ///
    /// Returns a `Receiver<ChartTickUpdate>`.  This is a `DISTINCT`-mode
    /// subscription — every message is a fresh tick, not a merge.
    #[instrument(skip(self), fields(%epic))]
    pub async fn subscribe_chart_tick(
        &self,
        epic: &str,
    ) -> Result<mpsc::Receiver<ChartTickUpdate>> {
        use crate::streaming::events::CHART_TICK_FIELDS;
        let (tx, rx) = mpsc::channel(CHANNEL_CAP);
        let idx = self.registry.register(SubscriptionKind::ChartTick {
            epic: epic.to_owned(),
            tx,
        });
        let item = format!("CHART:{epic}:TICK");
        let fields = CHART_TICK_FIELDS.join(" ");
        self.conn
            .control("add", idx, &item, &fields, "DISTINCT")
            .await?;
        Ok(rx)
    }

    // ------------------------------------------------------------------
    // Chart candle
    // ------------------------------------------------------------------

    /// Subscribe to OHLC candle data for `epic` at `scale`.
    ///
    /// Returns a `Receiver<ChartCandleUpdate>`.  This is a `MERGE`-mode
    /// subscription — fields are merged across updates for the current candle.
    #[instrument(skip(self), fields(%epic, scale = %scale))]
    pub async fn subscribe_chart_candle(
        &self,
        epic: &str,
        scale: CandleScale,
    ) -> Result<mpsc::Receiver<ChartCandleUpdate>> {
        use crate::streaming::events::CHART_CANDLE_FIELDS;
        let (tx, rx) = mpsc::channel(CHANNEL_CAP);
        let idx = self.registry.register(SubscriptionKind::ChartCandle {
            epic: epic.to_owned(),
            scale,
            tx,
        });
        let item = format!("CHART:{epic}:{}", scale.as_str());
        let fields = CHART_CANDLE_FIELDS.join(" ");
        self.conn
            .control("add", idx, &item, &fields, "MERGE")
            .await?;
        Ok(rx)
    }

    // ------------------------------------------------------------------
    // Account
    // ------------------------------------------------------------------

    /// Subscribe to account balance and margin updates.
    ///
    /// Returns a `Receiver<AccountUpdate>`.
    #[instrument(skip(self), fields(%account_id))]
    pub async fn subscribe_account(
        &self,
        account_id: &str,
    ) -> Result<mpsc::Receiver<AccountUpdate>> {
        use crate::streaming::events::ACCOUNT_FIELDS;
        let (tx, rx) = mpsc::channel(CHANNEL_CAP);
        let idx = self.registry.register(SubscriptionKind::Account {
            account_id: account_id.to_owned(),
            tx,
        });
        let item = format!("ACCOUNT:{account_id}");
        let fields = ACCOUNT_FIELDS.join(" ");
        self.conn
            .control("add", idx, &item, &fields, "MERGE")
            .await?;
        Ok(rx)
    }

    // ------------------------------------------------------------------
    // Trade
    // ------------------------------------------------------------------

    /// Subscribe to trade confirmations and working-order updates.
    ///
    /// Returns a `Receiver<TradeUpdate>`.
    #[instrument(skip(self), fields(%account_id))]
    pub async fn subscribe_trade(&self, account_id: &str) -> Result<mpsc::Receiver<TradeUpdate>> {
        use crate::streaming::events::TRADE_FIELDS;
        let (tx, rx) = mpsc::channel(CHANNEL_CAP);
        let idx = self.registry.register(SubscriptionKind::Trade {
            account_id: account_id.to_owned(),
            tx,
        });
        let item = format!("TRADE:{account_id}");
        let fields = TRADE_FIELDS.join(" ");
        self.conn
            .control("add", idx, &item, &fields, "DISTINCT")
            .await?;
        Ok(rx)
    }

    // ------------------------------------------------------------------
    // Lifecycle
    // ------------------------------------------------------------------

    /// Disconnect from Lightstreamer and stop the background read-loop task.
    ///
    /// After this call all pending `Receiver`s will no longer receive updates.
    ///
    /// The method signature is `async` for forward compatibility (future
    /// implementations may need to await a clean shutdown handshake with the
    /// server).
    #[allow(clippy::unused_async)]
    pub async fn disconnect(self) -> Result<()> {
        // Signal the background read-loop to stop.
        let _ = self.shutdown_tx.send(true);
        Ok(())
    }

    /// Return the current Lightstreamer session ID.
    pub fn session_id(&self) -> &str {
        &self.conn.session_id
    }
}
