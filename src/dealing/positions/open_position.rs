//! Type-state builder for `POST /positions/otc` (v2).
//!
//! ## Design
//!
//! Two marker types gate the `.send()` method:
//!
//! - [`MissingMandatory`] — initial state; mandatory field setters return
//!   `Self`; `.send()` is **not** available.
//! - [`WithMandatories`] — reached after calling `.guaranteed_stop(bool)`,
//!   which validates at runtime that all other mandatory fields have also been
//!   set and transitions the type.  Optional setters are available on both
//!   states; `.send()` is **only** available on `WithMandatories`.
//!
//! Calling `.guaranteed_stop()` without first setting `epic`, `direction`,
//! `size`, `order_type`, `currency`, and `expiry` returns
//! `Err(Error::InvalidInput)`.  Callers who want explicit checking before the
//! state transition can use [`OpenPositionBuilder::try_send_ready`].
//!
//! ## Typical usage
//!
//! ```ignore
//! client.dealing().positions().open()
//!     .epic("CS.D.GBPUSD.TODAY.IP")
//!     .direction(Direction::Buy)
//!     .size(1.0)
//!     .order_type(OrderType::Market)
//!     .currency("GBP")
//!     .expiry("DFB")
//!     .guaranteed_stop(false)   // ← transitions to WithMandatories
//!     .with_stop_distance(20.0) // optional, on WithMandatories
//!     .send()
//!     .await?;
//! ```

use http::Method;
use serde::Serialize;
use tracing::instrument;

use crate::dealing::positions::models::{DealConfirmation, OpenPositionResponse};
use crate::error::{Error, Result};
use crate::models::common::{Currency, Direction, Epic, OrderType, TimeInForce};

use super::api::PositionsApi;

// ---------------------------------------------------------------------------
// Marker types
// ---------------------------------------------------------------------------

/// Marker: not all mandatory fields have been confirmed yet.
#[derive(Debug)]
pub struct MissingMandatory;

/// Marker: all mandatory fields are present — `.send()` is callable.
#[derive(Debug)]
pub struct WithMandatories;

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
// Builder struct (shared fields across both states via a common base)
// ---------------------------------------------------------------------------

/// Shared inner state for both builder states.
#[derive(Debug)]
struct Inner {
    currency_code: Option<Currency>,
    direction: Option<Direction>,
    epic: Option<Epic>,
    expiry: Option<String>,
    force_open: bool,
    guaranteed_stop: Option<bool>,
    order_type: Option<OrderType>,
    size: Option<f64>,
    level: Option<f64>,
    limit_distance: Option<f64>,
    limit_level: Option<f64>,
    stop_distance: Option<f64>,
    stop_level: Option<f64>,
    trailing_stop: Option<bool>,
    trailing_stop_increment: Option<f64>,
    time_in_force: Option<TimeInForce>,
    quote_id: Option<String>,
}

impl Inner {
    fn new() -> Self {
        Self {
            currency_code: None,
            direction: None,
            epic: None,
            expiry: None,
            force_open: false,
            guaranteed_stop: None,
            order_type: None,
            size: None,
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

    fn all_mandatories_set(&self) -> bool {
        self.currency_code.is_some()
            && self.direction.is_some()
            && self.epic.is_some()
            && self.expiry.is_some()
            && self.guaranteed_stop.is_some()
            && self.order_type.is_some()
            && self.size.is_some()
    }

    #[allow(clippy::unwrap_used)] // caller guarantees all_mandatories_set() is true
    fn into_body(self) -> OpenPositionBody {
        OpenPositionBody {
            currency_code: self.currency_code.unwrap(),
            direction: self.direction.unwrap(),
            epic: self.epic.unwrap(),
            expiry: self.expiry.unwrap(),
            force_open: self.force_open,
            guaranteed_stop: self.guaranteed_stop.unwrap(),
            order_type: self.order_type.unwrap(),
            size: self.size.unwrap(),
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
// Builder
// ---------------------------------------------------------------------------

/// Builder for `POST /positions/otc`.
///
/// Obtain via [`PositionsApi::open`]. Set all mandatory fields and call
/// `.guaranteed_stop(bool)` to transition to [`WithMandatories`], then
/// optionally set additional fields and call `.send()`.
pub struct OpenPositionBuilder<'a, State> {
    pub(super) api: &'a PositionsApi<'a>,
    inner: Inner,
    _state: std::marker::PhantomData<State>,
}

impl<'a> OpenPositionBuilder<'a, MissingMandatory> {
    pub(super) fn new(api: &'a PositionsApi<'a>) -> Self {
        Self {
            api,
            inner: Inner::new(),
            _state: std::marker::PhantomData,
        }
    }

    /// Set the ISO-4217 currency code.
    pub fn currency(mut self, c: impl Into<Currency>) -> Self {
        self.inner.currency_code = Some(c.into());
        self
    }

    /// Set the trade direction.
    pub fn direction(mut self, d: Direction) -> Self {
        self.inner.direction = Some(d);
        self
    }

    /// Set the instrument epic.
    pub fn epic(mut self, e: impl Into<Epic>) -> Self {
        self.inner.epic = Some(e.into());
        self
    }

    /// Set the instrument expiry (`"DFB"`, `"-"`, or a date string).
    pub fn expiry(mut self, e: impl Into<String>) -> Self {
        self.inner.expiry = Some(e.into());
        self
    }

    /// Override `force_open` (defaults to `false`).
    pub fn force_open(mut self, fo: bool) -> Self {
        self.inner.force_open = fo;
        self
    }

    /// Set the order type.
    pub fn order_type(mut self, ot: OrderType) -> Self {
        self.inner.order_type = Some(ot);
        self
    }

    /// Set the position size.
    pub fn size(mut self, s: f64) -> Self {
        self.inner.size = Some(s);
        self
    }

    // --- optional setters callable while still in MissingMandatory ---

    /// Set the entry level (required for `LIMIT` / `QUOTE` order types).
    pub fn level(mut self, l: f64) -> Self {
        self.inner.level = Some(l);
        self
    }

    /// Set a limit as distance from the opening level.
    pub fn with_limit_distance(mut self, d: f64) -> Self {
        self.inner.limit_distance = Some(d);
        self
    }

    /// Set a limit as an absolute level.
    pub fn with_limit_level(mut self, l: f64) -> Self {
        self.inner.limit_level = Some(l);
        self
    }

    /// Set a stop as distance from the opening level.
    pub fn with_stop_distance(mut self, d: f64) -> Self {
        self.inner.stop_distance = Some(d);
        self
    }

    /// Set a stop as an absolute level.
    pub fn with_stop_level(mut self, l: f64) -> Self {
        self.inner.stop_level = Some(l);
        self
    }

    /// Enable/disable trailing stop.
    pub fn trailing_stop(mut self, ts: bool) -> Self {
        self.inner.trailing_stop = Some(ts);
        self
    }

    /// Set trailing stop increment.
    pub fn trailing_stop_increment(mut self, tsi: f64) -> Self {
        self.inner.trailing_stop_increment = Some(tsi);
        self
    }

    /// Set the time-in-force policy.
    pub fn time_in_force(mut self, tif: TimeInForce) -> Self {
        self.inner.time_in_force = Some(tif);
        self
    }

    /// Set the quote ID (required for `QUOTE` order type).
    pub fn quote_id(mut self, qid: impl Into<String>) -> Self {
        self.inner.quote_id = Some(qid.into());
        self
    }

    /// Set whether the stop is guaranteed. **This is the final mandatory
    /// setter.** Transitions to [`WithMandatories`] after validating that
    /// all other mandatory fields (`epic`, `direction`, `size`, `order_type`,
    /// `currency`, `expiry`) have been set.
    ///
    /// # Errors
    ///
    /// Returns `Err(Error::InvalidInput)` if any other mandatory field is
    /// missing. All other setters are infallible — errors are deferred here.
    pub fn guaranteed_stop(
        mut self,
        gs: bool,
    ) -> std::result::Result<OpenPositionBuilder<'a, WithMandatories>, Error> {
        self.inner.guaranteed_stop = Some(gs);
        self.try_send_ready()
    }

    /// Explicitly attempt to transition to [`WithMandatories`].
    ///
    /// Returns an error if any mandatory field is still unset. Use this when
    /// you've set `guaranteed_stop` via a different setter path or want
    /// explicit error handling before the state transition.
    pub fn try_send_ready(
        self,
    ) -> std::result::Result<OpenPositionBuilder<'a, WithMandatories>, Error> {
        if self.inner.all_mandatories_set() {
            Ok(OpenPositionBuilder {
                api: self.api,
                inner: self.inner,
                _state: std::marker::PhantomData,
            })
        } else {
            Err(Error::InvalidInput(
                "not all mandatory fields set on OpenPositionBuilder \
                 (required: epic, direction, size, order_type, currency, expiry, guaranteed_stop)"
                    .into(),
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// WithMandatories — optional setters + send
// ---------------------------------------------------------------------------

impl OpenPositionBuilder<'_, WithMandatories> {
    /// Set the entry level (required for `LIMIT` / `QUOTE` order types).
    pub fn level(mut self, l: f64) -> Self {
        self.inner.level = Some(l);
        self
    }

    /// Set a limit as distance from the opening level.
    pub fn with_limit_distance(mut self, d: f64) -> Self {
        self.inner.limit_distance = Some(d);
        self
    }

    /// Set a limit as an absolute level.
    pub fn with_limit_level(mut self, l: f64) -> Self {
        self.inner.limit_level = Some(l);
        self
    }

    /// Set a stop as distance from the opening level.
    pub fn with_stop_distance(mut self, d: f64) -> Self {
        self.inner.stop_distance = Some(d);
        self
    }

    /// Set a stop as an absolute level.
    pub fn with_stop_level(mut self, l: f64) -> Self {
        self.inner.stop_level = Some(l);
        self
    }

    /// Enable/disable trailing stop.
    pub fn trailing_stop(mut self, ts: bool) -> Self {
        self.inner.trailing_stop = Some(ts);
        self
    }

    /// Set trailing stop increment.
    pub fn trailing_stop_increment(mut self, tsi: f64) -> Self {
        self.inner.trailing_stop_increment = Some(tsi);
        self
    }

    /// Set the time-in-force policy.
    pub fn time_in_force(mut self, tif: TimeInForce) -> Self {
        self.inner.time_in_force = Some(tif);
        self
    }

    /// Set the quote ID (required for `QUOTE` order type).
    pub fn quote_id(mut self, qid: impl Into<String>) -> Self {
        self.inner.quote_id = Some(qid.into());
        self
    }

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
        let body = self.inner.into_body();

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

// ---------------------------------------------------------------------------
// Debug impl
// ---------------------------------------------------------------------------

impl<State> std::fmt::Debug for OpenPositionBuilder<'_, State> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenPositionBuilder")
            .field("epic", &self.inner.epic)
            .field("direction", &self.inner.direction)
            .field("size", &self.inner.size)
            .field("order_type", &self.inner.order_type)
            .finish()
    }
}
