//! Type-state builder for `POST /positions/otc` (v2).
//!
//! ## Design
//!
//! Seven independent type parameters (one per mandatory field) enforce at
//! compile time that all mandatory fields have been supplied before `.send()`
//! becomes callable. Optional fields (`level`, stop/limit distances, etc.) are
//! set via infallible setters at any point.
//!
//! Type params: `CC`=`currency_code`, `Di`=`direction`, `E`=`epic`,
//! `X`=`expiry`, `G`=`guaranteed_stop`, `O`=`order_type`, `Si`=`size`.
//!
//! Note: `level` is left in the optional group — it is only required when
//! `order_type` is `LIMIT` or `QUOTE`, which is a runtime condition.
//!
//! ## Typical usage
//!
//! ```no_run
//! # async fn run(client: trading_ig::IgClient) -> trading_ig::Result<()> {
//! use trading_ig::models::common::{Currency, Direction, Epic, OrderType};
//!
//! let confirmation = client.dealing().positions().open()
//!     .epic(Epic::new("CS.D.GBPUSD.TODAY.IP"))
//!     .direction(Direction::Buy)
//!     .size(1.0)
//!     .order_type(OrderType::Market)
//!     .currency("GBP")
//!     .expiry("DFB")
//!     .guaranteed_stop(false)
//!     .with_stop_distance(20.0)
//!     .send()
//!     .await?;
//! # Ok(()) }
//! ```

use http::Method;
use serde::Serialize;
use tracing::instrument;

use crate::dealing::common::DealConfirmation;
use crate::dealing::positions::models::OpenPositionResponse;
use crate::error::Result;
use crate::models::common::{Currency, Direction, Epic, OrderType, TimeInForce};

use super::api::PositionsApi;

// ---------------------------------------------------------------------------
// Marker types (shared with working_orders style)
// ---------------------------------------------------------------------------

/// Marker: this mandatory field has not yet been set.
#[derive(Debug)]
pub struct Missing;

/// Marker: this mandatory field has been set to type `T`.
#[derive(Debug)]
pub struct Set<T>(pub(crate) T);

// ---------------------------------------------------------------------------
// Internal request body (serialised to JSON)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OpenPositionBody {
    pub currency_code: Currency,
    pub direction: Direction,
    pub epic: Epic,
    pub expiry: String,
    pub force_open: bool,
    pub guaranteed_stop: bool,
    pub order_type: OrderType,
    pub size: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_distance: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_level: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_distance: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_level: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trailing_stop: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trailing_stop_increment: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_in_force: Option<TimeInForce>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Builder struct
// ---------------------------------------------------------------------------

/// Type-state builder for `POST /positions/otc` (v2).
///
/// Type params: `CC`=`currency_code`, `Di`=`direction`, `E`=`epic`,
/// `X`=`expiry`, `G`=`guaranteed_stop`, `O`=`order_type`, `Si`=`size`.
///
/// Obtain via [`PositionsApi::open`]. Supply all mandatory fields (order
/// independent), then call `.send()`.
pub struct OpenPositionBuilder<'a, CC, Di, E, X, G, O, Si> {
    pub(super) api: &'a PositionsApi<'a>,
    // mandatory fields (type-state)
    pub(super) currency_code: CC,
    pub(super) direction: Di,
    pub(super) epic: E,
    pub(super) expiry: X,
    pub(super) guaranteed_stop: G,
    pub(super) order_type: O,
    pub(super) size: Si,
    // optional fields
    pub(super) force_open: bool,
    pub(super) level: Option<f64>,
    pub(super) limit_distance: Option<f64>,
    pub(super) limit_level: Option<f64>,
    pub(super) stop_distance: Option<f64>,
    pub(super) stop_level: Option<f64>,
    pub(super) trailing_stop: Option<bool>,
    pub(super) trailing_stop_increment: Option<f64>,
    pub(super) time_in_force: Option<TimeInForce>,
    pub(super) quote_id: Option<String>,
}

impl<CC, Di, E, X, G, O, Si> std::fmt::Debug
    for OpenPositionBuilder<'_, CC, Di, E, X, G, O, Si>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenPositionBuilder").finish_non_exhaustive()
    }
}

/// Fully-specified builder — the only state where `.send()` is callable.
pub type ReadyOpenPositionBuilder<'a> = OpenPositionBuilder<
    'a,
    Set<Currency>,
    Set<Direction>,
    Set<Epic>,
    Set<String>,
    Set<bool>,
    Set<OrderType>,
    Set<f64>,
>;

// ---------------------------------------------------------------------------
// Constructor
// ---------------------------------------------------------------------------

impl<'a>
    OpenPositionBuilder<
        'a,
        Missing,
        Missing,
        Missing,
        Missing,
        Missing,
        Missing,
        Missing,
    >
{
    pub(super) fn new(api: &'a PositionsApi<'a>) -> Self {
        Self {
            api,
            currency_code: Missing,
            direction: Missing,
            epic: Missing,
            expiry: Missing,
            guaranteed_stop: Missing,
            order_type: Missing,
            size: Missing,
            force_open: false,
            level: None,
            limit_distance: None,
            limit_level: None,
            stop_distance: None,
            stop_level: None,
            trailing_stop: None,
            trailing_stop_increment: None,
            time_in_force: None,
            quote_id: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Mandatory setters
// ---------------------------------------------------------------------------

impl<'a, Di, E, X, G, O, Si>
    OpenPositionBuilder<'a, Missing, Di, E, X, G, O, Si>
{
    /// Set the ISO-4217 currency code.
    pub fn currency(
        self,
        c: impl Into<Currency>,
    ) -> OpenPositionBuilder<'a, Set<Currency>, Di, E, X, G, O, Si> {
        OpenPositionBuilder {
            api: self.api,
            currency_code: Set(c.into()),
            direction: self.direction,
            epic: self.epic,
            expiry: self.expiry,
            guaranteed_stop: self.guaranteed_stop,
            order_type: self.order_type,
            size: self.size,
            force_open: self.force_open,
            level: self.level,
            limit_distance: self.limit_distance,
            limit_level: self.limit_level,
            stop_distance: self.stop_distance,
            stop_level: self.stop_level,
            trailing_stop: self.trailing_stop,
            trailing_stop_increment: self.trailing_stop_increment,
            time_in_force: self.time_in_force,
            quote_id: self.quote_id,
        }
    }
}

impl<'a, CC, E, X, G, O, Si>
    OpenPositionBuilder<'a, CC, Missing, E, X, G, O, Si>
{
    /// Set the trade direction.
    pub fn direction(
        self,
        d: Direction,
    ) -> OpenPositionBuilder<'a, CC, Set<Direction>, E, X, G, O, Si> {
        OpenPositionBuilder {
            api: self.api,
            currency_code: self.currency_code,
            direction: Set(d),
            epic: self.epic,
            expiry: self.expiry,
            guaranteed_stop: self.guaranteed_stop,
            order_type: self.order_type,
            size: self.size,
            force_open: self.force_open,
            level: self.level,
            limit_distance: self.limit_distance,
            limit_level: self.limit_level,
            stop_distance: self.stop_distance,
            stop_level: self.stop_level,
            trailing_stop: self.trailing_stop,
            trailing_stop_increment: self.trailing_stop_increment,
            time_in_force: self.time_in_force,
            quote_id: self.quote_id,
        }
    }
}

impl<'a, CC, Di, X, G, O, Si>
    OpenPositionBuilder<'a, CC, Di, Missing, X, G, O, Si>
{
    /// Set the instrument epic.
    pub fn epic(
        self,
        e: impl Into<Epic>,
    ) -> OpenPositionBuilder<'a, CC, Di, Set<Epic>, X, G, O, Si> {
        OpenPositionBuilder {
            api: self.api,
            currency_code: self.currency_code,
            direction: self.direction,
            epic: Set(e.into()),
            expiry: self.expiry,
            guaranteed_stop: self.guaranteed_stop,
            order_type: self.order_type,
            size: self.size,
            force_open: self.force_open,
            level: self.level,
            limit_distance: self.limit_distance,
            limit_level: self.limit_level,
            stop_distance: self.stop_distance,
            stop_level: self.stop_level,
            trailing_stop: self.trailing_stop,
            trailing_stop_increment: self.trailing_stop_increment,
            time_in_force: self.time_in_force,
            quote_id: self.quote_id,
        }
    }
}

impl<'a, CC, Di, E, G, O, Si>
    OpenPositionBuilder<'a, CC, Di, E, Missing, G, O, Si>
{
    /// Set the instrument expiry (`"DFB"`, `"-"`, or a date string).
    pub fn expiry(
        self,
        e: impl Into<String>,
    ) -> OpenPositionBuilder<'a, CC, Di, E, Set<String>, G, O, Si> {
        OpenPositionBuilder {
            api: self.api,
            currency_code: self.currency_code,
            direction: self.direction,
            epic: self.epic,
            expiry: Set(e.into()),
            guaranteed_stop: self.guaranteed_stop,
            order_type: self.order_type,
            size: self.size,
            force_open: self.force_open,
            level: self.level,
            limit_distance: self.limit_distance,
            limit_level: self.limit_level,
            stop_distance: self.stop_distance,
            stop_level: self.stop_level,
            trailing_stop: self.trailing_stop,
            trailing_stop_increment: self.trailing_stop_increment,
            time_in_force: self.time_in_force,
            quote_id: self.quote_id,
        }
    }
}

impl<'a, CC, Di, E, X, O, Si>
    OpenPositionBuilder<'a, CC, Di, E, X, Missing, O, Si>
{
    /// Set whether the stop is guaranteed.
    pub fn guaranteed_stop(
        self,
        gs: bool,
    ) -> OpenPositionBuilder<'a, CC, Di, E, X, Set<bool>, O, Si> {
        OpenPositionBuilder {
            api: self.api,
            currency_code: self.currency_code,
            direction: self.direction,
            epic: self.epic,
            expiry: self.expiry,
            guaranteed_stop: Set(gs),
            order_type: self.order_type,
            size: self.size,
            force_open: self.force_open,
            level: self.level,
            limit_distance: self.limit_distance,
            limit_level: self.limit_level,
            stop_distance: self.stop_distance,
            stop_level: self.stop_level,
            trailing_stop: self.trailing_stop,
            trailing_stop_increment: self.trailing_stop_increment,
            time_in_force: self.time_in_force,
            quote_id: self.quote_id,
        }
    }
}

impl<'a, CC, Di, E, X, G, Si>
    OpenPositionBuilder<'a, CC, Di, E, X, G, Missing, Si>
{
    /// Set the order type.
    pub fn order_type(
        self,
        ot: OrderType,
    ) -> OpenPositionBuilder<'a, CC, Di, E, X, G, Set<OrderType>, Si> {
        OpenPositionBuilder {
            api: self.api,
            currency_code: self.currency_code,
            direction: self.direction,
            epic: self.epic,
            expiry: self.expiry,
            guaranteed_stop: self.guaranteed_stop,
            order_type: Set(ot),
            size: self.size,
            force_open: self.force_open,
            level: self.level,
            limit_distance: self.limit_distance,
            limit_level: self.limit_level,
            stop_distance: self.stop_distance,
            stop_level: self.stop_level,
            trailing_stop: self.trailing_stop,
            trailing_stop_increment: self.trailing_stop_increment,
            time_in_force: self.time_in_force,
            quote_id: self.quote_id,
        }
    }
}

impl<'a, CC, Di, E, X, G, O>
    OpenPositionBuilder<'a, CC, Di, E, X, G, O, Missing>
{
    /// Set the position size.
    pub fn size(
        self,
        s: f64,
    ) -> OpenPositionBuilder<'a, CC, Di, E, X, G, O, Set<f64>> {
        OpenPositionBuilder {
            api: self.api,
            currency_code: self.currency_code,
            direction: self.direction,
            epic: self.epic,
            expiry: self.expiry,
            guaranteed_stop: self.guaranteed_stop,
            order_type: self.order_type,
            size: Set(s),
            force_open: self.force_open,
            level: self.level,
            limit_distance: self.limit_distance,
            limit_level: self.limit_level,
            stop_distance: self.stop_distance,
            stop_level: self.stop_level,
            trailing_stop: self.trailing_stop,
            trailing_stop_increment: self.trailing_stop_increment,
            time_in_force: self.time_in_force,
            quote_id: self.quote_id,
        }
    }
}

// ---------------------------------------------------------------------------
// Optional setters (available on any builder state)
// ---------------------------------------------------------------------------

impl<CC, Di, E, X, G, O, Si>
    OpenPositionBuilder<'_, CC, Di, E, X, G, O, Si>
{
    /// Override `force_open` (defaults to `false`).
    pub fn force_open(mut self, fo: bool) -> Self {
        self.force_open = fo;
        self
    }

    /// Set the entry level (required at runtime for `LIMIT` / `QUOTE` order types).
    pub fn level(mut self, l: f64) -> Self {
        self.level = Some(l);
        self
    }

    /// Set a limit as distance from the opening level.
    pub fn with_limit_distance(mut self, d: f64) -> Self {
        self.limit_distance = Some(d);
        self
    }

    /// Set a limit as an absolute level.
    pub fn with_limit_level(mut self, l: f64) -> Self {
        self.limit_level = Some(l);
        self
    }

    /// Set a stop as distance from the opening level.
    pub fn with_stop_distance(mut self, d: f64) -> Self {
        self.stop_distance = Some(d);
        self
    }

    /// Set a stop as an absolute level.
    pub fn with_stop_level(mut self, l: f64) -> Self {
        self.stop_level = Some(l);
        self
    }

    /// Enable/disable trailing stop.
    pub fn trailing_stop(mut self, ts: bool) -> Self {
        self.trailing_stop = Some(ts);
        self
    }

    /// Set trailing stop increment.
    pub fn trailing_stop_increment(mut self, tsi: f64) -> Self {
        self.trailing_stop_increment = Some(tsi);
        self
    }

    /// Set the time-in-force policy.
    pub fn time_in_force(mut self, tif: TimeInForce) -> Self {
        self.time_in_force = Some(tif);
        self
    }

    /// Set the quote ID (required at runtime for `QUOTE` order type).
    pub fn quote_id(mut self, qid: impl Into<String>) -> Self {
        self.quote_id = Some(qid.into());
        self
    }
}

// ---------------------------------------------------------------------------
// `.send()` — only available on the fully-specified builder
// ---------------------------------------------------------------------------

impl ReadyOpenPositionBuilder<'_> {
    /// Send the open-position request.
    ///
    /// Steps:
    /// 1. `POST /positions/otc` (Version: 2) → receives `{ dealReference }`.
    /// 2. Calls [`PositionsApi::confirm`] (with up to 5 retries × 1 s
    ///    back-off) to fetch the full [`DealConfirmation`].
    ///
    /// # Errors
    ///
    /// Returns `Error::Api` if IG rejects the open request or if the
    /// confirmation cannot be retrieved after 5 retries.
    #[instrument(skip_all, name = "dealing.positions.open")]
    pub async fn send(self) -> Result<DealConfirmation> {
        let api = self.api;
        let body = OpenPositionBody {
            currency_code: self.currency_code.0,
            direction: self.direction.0,
            epic: self.epic.0,
            expiry: self.expiry.0,
            force_open: self.force_open,
            guaranteed_stop: self.guaranteed_stop.0,
            order_type: self.order_type.0,
            size: self.size.0,
            level: self.level,
            limit_distance: self.limit_distance,
            limit_level: self.limit_level,
            stop_distance: self.stop_distance,
            stop_level: self.stop_level,
            trailing_stop: self.trailing_stop,
            trailing_stop_increment: self.trailing_stop_increment,
            time_in_force: self.time_in_force,
            quote_id: self.quote_id,
        };

        let resp: OpenPositionResponse = api
            .client
            .transport
            .request(
                Method::POST,
                "positions/otc",
                Some(2),
                Some(&body),
                &api.client.session,
            )
            .await?;

        api.confirm(&resp.deal_reference).await
    }
}
