//! Type-state builder for `POST /workingorders/otc` (v2).
//!
//! All nine mandatory fields must be supplied before `.send()` compiles.
//! Optional fields are set via `with_*` methods and do not affect the type.
//!
//! # Example
//!
//! ```no_run
//! # async fn run(client: trading_ig::IgClient) -> trading_ig::Result<()> {
//! use trading_ig::models::common::{Currency, Direction, Epic, OrderType, TimeInForce};
//!
//! let confirmation = client.dealing().working_orders().create()
//!     .epic(Epic::new("CS.D.GBPUSD.TODAY.IP"))
//!     .direction(Direction::Buy)
//!     .size(1.0)
//!     .order_type(OrderType::Limit)
//!     .currency(Currency::new("GBP"))
//!     .expiry("DFB")
//!     .level(1.2345)
//!     .time_in_force(TimeInForce::GoodTillCancelled)
//!     .guaranteed_stop(false)
//!     .send()
//!     .await?;
//! # Ok(()) }
//! ```

use chrono::NaiveDateTime;
use http::Method;
use serde::Serialize;
use std::time::Duration;
use tracing::warn;

use crate::dealing::common::DealConfirmation;
use crate::error::Result;
use crate::models::common::{Currency, DealReference, Direction, Epic, OrderType, TimeInForce};

/// Marker: this mandatory field has not yet been set.
#[derive(Debug)]
pub struct Missing;
/// Marker: this mandatory field has been set to type `T`.
#[derive(Debug)]
pub struct Set<T>(pub(crate) T);

/// Type-state builder for creating a working order.
///
/// Type parameters track which mandatory fields have been supplied:
/// `E`=epic, `Di`=direction, `Si`=size, `O`=`order_type`, `C`=currency,
/// `X`=expiry, `L`=level, `T`=`time_in_force`, `G`=`guaranteed_stop`.
pub struct CreateWorkingOrderBuilder<'c, E, Di, Si, O, C, X, L, T, G> {
    pub(super) client: &'c crate::IgClient,
    // mandatory
    pub(super) epic: E,
    pub(super) direction: Di,
    pub(super) size: Si,
    pub(super) order_type: O,
    pub(super) currency: C,
    pub(super) expiry: X,
    pub(super) level: L,
    pub(super) time_in_force: T,
    pub(super) guaranteed_stop: G,
    // optional
    pub(super) force_open: Option<bool>,
    pub(super) good_till_date: Option<NaiveDateTime>,
    pub(super) limit_distance: Option<f64>,
    pub(super) limit_level: Option<f64>,
    pub(super) stop_distance: Option<f64>,
    pub(super) stop_level: Option<f64>,
    pub(super) deal_reference: Option<DealReference>,
}

impl<E, Di, Si, O, C, X, L, T, G> std::fmt::Debug
    for CreateWorkingOrderBuilder<'_, E, Di, Si, O, C, X, L, T, G>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreateWorkingOrderBuilder").finish_non_exhaustive()
    }
}

/// The fully-specified builder alias — the only state where `.send()` is callable.
pub type ReadyBuilder<'c> = CreateWorkingOrderBuilder<
    'c,
    Set<Epic>,
    Set<Direction>,
    Set<f64>,
    Set<OrderType>,
    Set<Currency>,
    Set<String>,
    Set<f64>,
    Set<TimeInForce>,
    Set<bool>,
>;

// ── Constructor (all Missing) ─────────────────────────────────────────────────

impl<'c>
    CreateWorkingOrderBuilder<
        'c,
        Missing,
        Missing,
        Missing,
        Missing,
        Missing,
        Missing,
        Missing,
        Missing,
        Missing,
    >
{
    pub(super) fn new(client: &'c crate::IgClient) -> Self {
        Self {
            client,
            epic: Missing,
            direction: Missing,
            size: Missing,
            order_type: Missing,
            currency: Missing,
            expiry: Missing,
            level: Missing,
            time_in_force: Missing,
            guaranteed_stop: Missing,
            force_open: None,
            good_till_date: None,
            limit_distance: None,
            limit_level: None,
            stop_distance: None,
            stop_level: None,
            deal_reference: None,
        }
    }
}

// ── Mandatory setters ─────────────────────────────────────────────────────────

// Each setter moves `self`, replaces one type param from `Missing` to `Set<T>`,
// and passes all other fields through unchanged.

impl<'c, Di, Si, O, C, X, L, T, G>
    CreateWorkingOrderBuilder<'c, Missing, Di, Si, O, C, X, L, T, G>
{
    /// Set the market EPIC.
    pub fn epic(
        self,
        epic: impl Into<Epic>,
    ) -> CreateWorkingOrderBuilder<'c, Set<Epic>, Di, Si, O, C, X, L, T, G> {
        CreateWorkingOrderBuilder {
            client: self.client,
            epic: Set(epic.into()),
            direction: self.direction,
            size: self.size,
            order_type: self.order_type,
            currency: self.currency,
            expiry: self.expiry,
            level: self.level,
            time_in_force: self.time_in_force,
            guaranteed_stop: self.guaranteed_stop,
            force_open: self.force_open,
            good_till_date: self.good_till_date,
            limit_distance: self.limit_distance,
            limit_level: self.limit_level,
            stop_distance: self.stop_distance,
            stop_level: self.stop_level,
            deal_reference: self.deal_reference,
        }
    }
}

impl<'c, E, Si, O, C, X, L, T, G>
    CreateWorkingOrderBuilder<'c, E, Missing, Si, O, C, X, L, T, G>
{
    /// Set the deal direction.
    pub fn direction(
        self,
        direction: Direction,
    ) -> CreateWorkingOrderBuilder<'c, E, Set<Direction>, Si, O, C, X, L, T, G> {
        CreateWorkingOrderBuilder {
            client: self.client,
            epic: self.epic,
            direction: Set(direction),
            size: self.size,
            order_type: self.order_type,
            currency: self.currency,
            expiry: self.expiry,
            level: self.level,
            time_in_force: self.time_in_force,
            guaranteed_stop: self.guaranteed_stop,
            force_open: self.force_open,
            good_till_date: self.good_till_date,
            limit_distance: self.limit_distance,
            limit_level: self.limit_level,
            stop_distance: self.stop_distance,
            stop_level: self.stop_level,
            deal_reference: self.deal_reference,
        }
    }
}

impl<'c, E, Di, O, C, X, L, T, G>
    CreateWorkingOrderBuilder<'c, E, Di, Missing, O, C, X, L, T, G>
{
    /// Set the order size.
    pub fn size(self, size: f64) -> CreateWorkingOrderBuilder<'c, E, Di, Set<f64>, O, C, X, L, T, G> {
        CreateWorkingOrderBuilder {
            client: self.client,
            epic: self.epic,
            direction: self.direction,
            size: Set(size),
            order_type: self.order_type,
            currency: self.currency,
            expiry: self.expiry,
            level: self.level,
            time_in_force: self.time_in_force,
            guaranteed_stop: self.guaranteed_stop,
            force_open: self.force_open,
            good_till_date: self.good_till_date,
            limit_distance: self.limit_distance,
            limit_level: self.limit_level,
            stop_distance: self.stop_distance,
            stop_level: self.stop_level,
            deal_reference: self.deal_reference,
        }
    }
}

impl<'c, E, Di, Si, C, X, L, T, G>
    CreateWorkingOrderBuilder<'c, E, Di, Si, Missing, C, X, L, T, G>
{
    /// Set the order type (maps to wire key `"type"`).
    pub fn order_type(
        self,
        order_type: OrderType,
    ) -> CreateWorkingOrderBuilder<'c, E, Di, Si, Set<OrderType>, C, X, L, T, G> {
        CreateWorkingOrderBuilder {
            client: self.client,
            epic: self.epic,
            direction: self.direction,
            size: self.size,
            order_type: Set(order_type),
            currency: self.currency,
            expiry: self.expiry,
            level: self.level,
            time_in_force: self.time_in_force,
            guaranteed_stop: self.guaranteed_stop,
            force_open: self.force_open,
            good_till_date: self.good_till_date,
            limit_distance: self.limit_distance,
            limit_level: self.limit_level,
            stop_distance: self.stop_distance,
            stop_level: self.stop_level,
            deal_reference: self.deal_reference,
        }
    }
}

impl<'c, E, Di, Si, O, X, L, T, G>
    CreateWorkingOrderBuilder<'c, E, Di, Si, O, Missing, X, L, T, G>
{
    /// Set the currency code (ISO-4217).
    pub fn currency(
        self,
        currency: impl Into<Currency>,
    ) -> CreateWorkingOrderBuilder<'c, E, Di, Si, O, Set<Currency>, X, L, T, G> {
        CreateWorkingOrderBuilder {
            client: self.client,
            epic: self.epic,
            direction: self.direction,
            size: self.size,
            order_type: self.order_type,
            currency: Set(currency.into()),
            expiry: self.expiry,
            level: self.level,
            time_in_force: self.time_in_force,
            guaranteed_stop: self.guaranteed_stop,
            force_open: self.force_open,
            good_till_date: self.good_till_date,
            limit_distance: self.limit_distance,
            limit_level: self.limit_level,
            stop_distance: self.stop_distance,
            stop_level: self.stop_level,
            deal_reference: self.deal_reference,
        }
    }
}

impl<'c, E, Di, Si, O, C, L, T, G>
    CreateWorkingOrderBuilder<'c, E, Di, Si, O, C, Missing, L, T, G>
{
    /// Set the expiry label (e.g. `"DFB"`, `"MAR-25"`).
    pub fn expiry(
        self,
        expiry: impl Into<String>,
    ) -> CreateWorkingOrderBuilder<'c, E, Di, Si, O, C, Set<String>, L, T, G> {
        CreateWorkingOrderBuilder {
            client: self.client,
            epic: self.epic,
            direction: self.direction,
            size: self.size,
            order_type: self.order_type,
            currency: self.currency,
            expiry: Set(expiry.into()),
            level: self.level,
            time_in_force: self.time_in_force,
            guaranteed_stop: self.guaranteed_stop,
            force_open: self.force_open,
            good_till_date: self.good_till_date,
            limit_distance: self.limit_distance,
            limit_level: self.limit_level,
            stop_distance: self.stop_distance,
            stop_level: self.stop_level,
            deal_reference: self.deal_reference,
        }
    }
}

impl<'c, E, Di, Si, O, C, X, T, G>
    CreateWorkingOrderBuilder<'c, E, Di, Si, O, C, X, Missing, T, G>
{
    /// Set the order trigger level.
    pub fn level(
        self,
        level: f64,
    ) -> CreateWorkingOrderBuilder<'c, E, Di, Si, O, C, X, Set<f64>, T, G> {
        CreateWorkingOrderBuilder {
            client: self.client,
            epic: self.epic,
            direction: self.direction,
            size: self.size,
            order_type: self.order_type,
            currency: self.currency,
            expiry: self.expiry,
            level: Set(level),
            time_in_force: self.time_in_force,
            guaranteed_stop: self.guaranteed_stop,
            force_open: self.force_open,
            good_till_date: self.good_till_date,
            limit_distance: self.limit_distance,
            limit_level: self.limit_level,
            stop_distance: self.stop_distance,
            stop_level: self.stop_level,
            deal_reference: self.deal_reference,
        }
    }
}

impl<'c, E, Di, Si, O, C, X, L, G>
    CreateWorkingOrderBuilder<'c, E, Di, Si, O, C, X, L, Missing, G>
{
    /// Set the time-in-force for the order.
    pub fn time_in_force(
        self,
        tif: TimeInForce,
    ) -> CreateWorkingOrderBuilder<'c, E, Di, Si, O, C, X, L, Set<TimeInForce>, G> {
        CreateWorkingOrderBuilder {
            client: self.client,
            epic: self.epic,
            direction: self.direction,
            size: self.size,
            order_type: self.order_type,
            currency: self.currency,
            expiry: self.expiry,
            level: self.level,
            time_in_force: Set(tif),
            guaranteed_stop: self.guaranteed_stop,
            force_open: self.force_open,
            good_till_date: self.good_till_date,
            limit_distance: self.limit_distance,
            limit_level: self.limit_level,
            stop_distance: self.stop_distance,
            stop_level: self.stop_level,
            deal_reference: self.deal_reference,
        }
    }
}

impl<'c, E, Di, Si, O, C, X, L, T>
    CreateWorkingOrderBuilder<'c, E, Di, Si, O, C, X, L, T, Missing>
{
    /// Set whether a guaranteed stop is required.
    pub fn guaranteed_stop(
        self,
        guaranteed_stop: bool,
    ) -> CreateWorkingOrderBuilder<'c, E, Di, Si, O, C, X, L, T, Set<bool>> {
        CreateWorkingOrderBuilder {
            client: self.client,
            epic: self.epic,
            direction: self.direction,
            size: self.size,
            order_type: self.order_type,
            currency: self.currency,
            expiry: self.expiry,
            level: self.level,
            time_in_force: self.time_in_force,
            guaranteed_stop: Set(guaranteed_stop),
            force_open: self.force_open,
            good_till_date: self.good_till_date,
            limit_distance: self.limit_distance,
            limit_level: self.limit_level,
            stop_distance: self.stop_distance,
            stop_level: self.stop_level,
            deal_reference: self.deal_reference,
        }
    }
}

// ── Optional setters (available on any state) ─────────────────────────────────

impl<E, Di, Si, O, C, X, L, T, G>
    CreateWorkingOrderBuilder<'_, E, Di, Si, O, C, X, L, T, G>
{
    /// Force open a new position even if a conflicting position exists.
    pub fn with_force_open(mut self, force_open: bool) -> Self {
        self.force_open = Some(force_open);
        self
    }

    /// Set a good-till date for the order (`GOOD_TILL_DATE` time-in-force).
    pub fn with_good_till_date(mut self, dt: NaiveDateTime) -> Self {
        self.good_till_date = Some(dt);
        self
    }

    /// Set a limit distance (in pips) for the order.
    pub fn with_limit_distance(mut self, dist: f64) -> Self {
        self.limit_distance = Some(dist);
        self
    }

    /// Set an absolute limit level for the order.
    pub fn with_limit_level(mut self, level: f64) -> Self {
        self.limit_level = Some(level);
        self
    }

    /// Set a stop distance (in pips) for the order.
    pub fn with_stop_distance(mut self, dist: f64) -> Self {
        self.stop_distance = Some(dist);
        self
    }

    /// Set an absolute stop level for the order.
    pub fn with_stop_level(mut self, level: f64) -> Self {
        self.stop_level = Some(level);
        self
    }

    /// Override the deal reference (client-side tracking token).
    pub fn with_deal_reference(mut self, r: impl Into<DealReference>) -> Self {
        self.deal_reference = Some(r.into());
        self
    }
}

// ── Wire body (private) ───────────────────────────────────────────────────────

/// The JSON body sent to `POST /workingorders/otc`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateWorkingOrderBody {
    currency_code: Currency,
    direction: Direction,
    epic: Epic,
    expiry: String,
    guaranteed_stop: bool,
    level: f64,
    size: f64,
    time_in_force: TimeInForce,
    /// Maps to wire key `"type"` per IG API spec.
    #[serde(rename = "type")]
    order_type: OrderType,
    #[serde(skip_serializing_if = "Option::is_none")]
    force_open: Option<bool>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_good_till_date_opt"
    )]
    good_till_date: Option<NaiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit_distance: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit_level: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_distance: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_level: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deal_reference: Option<DealReference>,
}

#[allow(clippy::ref_option)]
fn serialize_good_till_date_opt<S>(
    dt: &Option<NaiveDateTime>,
    s: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match dt {
        Some(dt) => s.serialize_str(&crate::time::format(*dt, crate::time::ApiVersion::V2)),
        None => s.serialize_none(),
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateResponse {
    deal_reference: DealReference,
}

// ── The `.send()` method — only on the fully-specified builder ────────────────

impl ReadyBuilder<'_> {
    /// Submit the working order and return the deal confirmation.
    ///
    /// Internally posts to `POST /workingorders/otc`, then polls
    /// `GET /confirms/{dealReference}` up to 5 times (1 s apart).
    ///
    /// # Errors
    ///
    /// Returns `Error::Api` if IG rejects the request, or `Error::Auth` if
    /// no session is active.
    #[tracing::instrument(
        skip_all,
        fields(
            epic = %self.epic.0,
            direction = ?self.direction.0,
            size = self.size.0
        )
    )]
    pub async fn send(self) -> Result<DealConfirmation> {
        let body = CreateWorkingOrderBody {
            currency_code: self.currency.0,
            direction: self.direction.0,
            epic: self.epic.0,
            expiry: self.expiry.0,
            guaranteed_stop: self.guaranteed_stop.0,
            level: self.level.0,
            size: self.size.0,
            time_in_force: self.time_in_force.0,
            order_type: self.order_type.0,
            force_open: self.force_open,
            good_till_date: self.good_till_date,
            limit_distance: self.limit_distance,
            limit_level: self.limit_level,
            stop_distance: self.stop_distance,
            stop_level: self.stop_level,
            deal_reference: self.deal_reference,
        };

        let resp: CreateResponse = self
            .client
            .transport
            .request(
                Method::POST,
                "workingorders/otc",
                Some(2),
                Some(&body),
                &self.client.session,
            )
            .await?;

        fetch_confirmation(self.client, &resp.deal_reference).await
    }
}

/// Poll `GET /confirms/{deal_reference}` up to 5 × 1 s.
pub(super) async fn fetch_confirmation(
    client: &crate::IgClient,
    deal_reference: &DealReference,
) -> Result<DealConfirmation> {
    let path = format!("confirms/{}", deal_reference.as_str());
    let mut last_err: Option<crate::error::Error> = None;

    for attempt in 1u8..=5 {
        match client
            .transport
            .request::<(), DealConfirmation>(Method::GET, &path, Some(1), None::<&()>, &client.session)
            .await
        {
            Ok(conf) => return Ok(conf),
            Err(e) => {
                warn!(attempt, error = %e, "confirm fetch failed; will retry");
                last_err = Some(e);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }

    Err(last_err.unwrap())
}
