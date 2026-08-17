//! Internal subscription registry.
//!
//! Each call to `subscribe_*` on [`crate::streaming::StreamingClient`] produces a
//! numbered subscription that is registered here.  When an incoming frame
//! arrives with a matching item index the registry deserialises the raw field
//! state and forwards the typed event to the appropriate channel.
//!
//! When a subscriber drops its [`tokio::sync::mpsc::Receiver`] the channel
//! becomes closed.  The next time the reader task tries to send on a closed
//! sender, `send` returns `Err` and we remove the entry from the registry.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;
use tracing::debug;

use crate::streaming::connection::SubscribeSpec;
use crate::streaming::events::{
    AccountUpdate, CandleScale, ChartCandleUpdate, ChartTickUpdate, MarketUpdate, PriceUpdate,
    TradeUpdate,
};
use crate::streaming::protocol::{FieldValue, merge_fields};

/// Lightstreamer data adapter serving the `PRICE` item family.
pub(crate) const PRICING_ADAPTER: &str = "Pricing";

// ---------------------------------------------------------------------------
// Subscription kind
// ---------------------------------------------------------------------------

/// Internal enum describing what kind of data a subscription carries.
pub(crate) enum SubscriptionKind {
    Price {
        account_id: String,
        epic: String,
        tx: mpsc::Sender<PriceUpdate>,
    },
    Market {
        epic: String,
        tx: mpsc::Sender<MarketUpdate>,
    },
    ChartTick {
        epic: String,
        tx: mpsc::Sender<ChartTickUpdate>,
    },
    ChartCandle {
        epic: String,
        scale: CandleScale,
        tx: mpsc::Sender<ChartCandleUpdate>,
    },
    Account {
        account_id: String,
        tx: mpsc::Sender<AccountUpdate>,
    },
    Trade {
        account_id: String,
        tx: mpsc::Sender<TradeUpdate>,
    },
}

impl std::fmt::Debug for SubscriptionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Price {
                account_id, epic, ..
            } => write!(f, "Price({account_id}/{epic})"),
            Self::Market { epic, .. } => write!(f, "Market({epic})"),
            Self::ChartTick { epic, .. } => write!(f, "ChartTick({epic})"),
            Self::ChartCandle { epic, scale, .. } => write!(f, "ChartCandle({epic}/{scale})"),
            Self::Account { account_id, .. } => write!(f, "Account({account_id})"),
            Self::Trade { account_id, .. } => write!(f, "Trade({account_id})"),
        }
    }
}

// ---------------------------------------------------------------------------
// Registry entry
// ---------------------------------------------------------------------------

pub(crate) struct Entry {
    pub(crate) kind: SubscriptionKind,
    /// Running field state — one slot per field declared at subscription time.
    pub(crate) field_state: Vec<Option<String>>,
    /// Lightstreamer 1-based item number assigned by the server.
    pub(crate) item_index: usize,
}

impl std::fmt::Debug for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Entry {{ item_index: {}, kind: {:?} }}",
            self.item_index, self.kind
        )
    }
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Shared subscription registry.
///
/// Cheap to clone — backed by `Arc<Mutex<..>>`.
#[derive(Debug, Clone)]
pub(crate) struct Registry {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Debug)]
struct Inner {
    /// Map from Lightstreamer item index → subscription entry.
    by_index: HashMap<usize, Entry>,
    /// Monotonically increasing counter used to assign item indices.
    next_index: usize,
}

impl Registry {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                by_index: HashMap::new(),
                next_index: 1,
            })),
        }
    }

    /// Register a new subscription and return the `control.txt` spec for it.
    ///
    /// Returning the spec (rather than just the index) keeps the initial
    /// `add` and the post-reconnect re-`add` derived from the same code path.
    pub(crate) fn register(&self, kind: SubscriptionKind) -> SubscribeSpec {
        let mut inner = self.inner.lock().expect("registry lock");
        let idx = inner.next_index;
        inner.next_index += 1;
        let field_len = kind_field_count(&kind);
        let spec = kind_wire_params(idx, &kind);
        inner.by_index.insert(
            idx,
            Entry {
                kind,
                field_state: vec![None; field_len],
                item_index: idx,
            },
        );
        spec
    }

    /// Apply an incoming update frame to the matching subscription.
    ///
    /// Returns `true` if the subscription is still live, `false` if the
    /// sender was closed (caller should remove the entry).
    pub(crate) fn apply_update(&self, item_index: usize, fields: &[FieldValue]) -> bool {
        let mut inner = self.inner.lock().expect("registry lock");
        let Some(entry) = inner.by_index.get_mut(&item_index) else {
            return true; // unknown index — ignore
        };
        merge_fields(&mut entry.field_state, fields);
        let state = entry.field_state.clone();
        let alive = dispatch(&entry.kind, &state);
        if !alive {
            debug!(item_index, "subscriber dropped — removing subscription");
        }
        alive
    }

    /// Remove a dead subscription from the registry.
    pub(crate) fn remove(&self, item_index: usize) {
        self.inner
            .lock()
            .expect("registry lock")
            .by_index
            .remove(&item_index);
    }

    /// Snapshot all entries for re-subscription after a reconnect.
    pub(crate) fn snapshot_for_resubscribe(&self) -> Vec<SubscribeSpec> {
        self.inner
            .lock()
            .expect("registry lock")
            .by_index
            .values()
            .map(|e| kind_wire_params(e.item_index, &e.kind))
            .collect()
    }
}

/// Send a typed event to the appropriate channel.  Returns `false` when the
/// channel is closed (receiver dropped).
fn dispatch(kind: &SubscriptionKind, state: &[Option<String>]) -> bool {
    match kind {
        SubscriptionKind::Price {
            account_id,
            epic,
            tx,
        } => {
            let update = PriceUpdate::from_raw(account_id, epic, state);
            tx.try_send(update).is_ok() || !tx.is_closed()
        }
        SubscriptionKind::Market { epic, tx } => {
            let update = MarketUpdate::from_raw(epic, state);
            tx.try_send(update).is_ok() || !tx.is_closed()
        }
        SubscriptionKind::ChartTick { epic, tx } => {
            let update = ChartTickUpdate::from_raw(epic, state);
            tx.try_send(update).is_ok() || !tx.is_closed()
        }
        SubscriptionKind::ChartCandle { epic, scale, tx } => {
            let update = ChartCandleUpdate::from_raw(epic, *scale, state);
            tx.try_send(update).is_ok() || !tx.is_closed()
        }
        SubscriptionKind::Account { account_id, tx } => {
            let update = AccountUpdate::from_raw(account_id, state);
            tx.try_send(update).is_ok() || !tx.is_closed()
        }
        SubscriptionKind::Trade { account_id, tx } => {
            let update = TradeUpdate::from_raw(account_id, state);
            tx.try_send(update).is_ok() || !tx.is_closed()
        }
    }
}

/// How many fields does this subscription kind declare?
fn kind_field_count(kind: &SubscriptionKind) -> usize {
    use crate::streaming::events::{
        ACCOUNT_FIELDS, CHART_CANDLE_FIELDS, CHART_TICK_FIELDS, MARKET_FIELDS, PRICE_FIELDS,
        TRADE_FIELDS,
    };
    match kind {
        SubscriptionKind::Price { .. } => PRICE_FIELDS.len(),
        SubscriptionKind::Market { .. } => MARKET_FIELDS.len(),
        SubscriptionKind::ChartTick { .. } => CHART_TICK_FIELDS.len(),
        SubscriptionKind::ChartCandle { .. } => CHART_CANDLE_FIELDS.len(),
        SubscriptionKind::Account { .. } => ACCOUNT_FIELDS.len(),
        SubscriptionKind::Trade { .. } => TRADE_FIELDS.len(),
    }
}

/// Build the `control.txt` parameters for the wire protocol.
fn kind_wire_params(item_index: usize, kind: &SubscriptionKind) -> SubscribeSpec {
    use crate::streaming::events::{
        ACCOUNT_FIELDS, CHART_CANDLE_FIELDS, CHART_TICK_FIELDS, MARKET_FIELDS, PRICE_FIELDS,
        TRADE_FIELDS,
    };
    let (item_name, fields, mode, data_adapter) = match kind {
        SubscriptionKind::Price {
            account_id, epic, ..
        } => (
            format!("PRICE:{account_id}:{epic}"),
            PRICE_FIELDS.join(" "),
            "MERGE",
            Some(PRICING_ADAPTER),
        ),
        SubscriptionKind::Market { epic, .. } => (
            format!("MARKET:{epic}"),
            MARKET_FIELDS.join(" "),
            "MERGE",
            None,
        ),
        SubscriptionKind::ChartTick { epic, .. } => (
            format!("CHART:{epic}:TICK"),
            CHART_TICK_FIELDS.join(" "),
            "DISTINCT",
            None,
        ),
        SubscriptionKind::ChartCandle { epic, scale, .. } => (
            format!("CHART:{epic}:{}", scale.as_str()),
            CHART_CANDLE_FIELDS.join(" "),
            "MERGE",
            None,
        ),
        SubscriptionKind::Account { account_id, .. } => (
            format!("ACCOUNT:{account_id}"),
            ACCOUNT_FIELDS.join(" "),
            "MERGE",
            None,
        ),
        SubscriptionKind::Trade { account_id, .. } => (
            format!("TRADE:{account_id}"),
            TRADE_FIELDS.join(" "),
            "DISTINCT",
            None,
        ),
    };
    SubscribeSpec {
        item_index,
        item_name,
        fields,
        mode,
        data_adapter,
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::protocol::FieldValue;
    use tokio::sync::mpsc;

    fn make_market_entry(
        epic: &str,
    ) -> (
        Registry,
        mpsc::Receiver<crate::streaming::events::MarketUpdate>,
    ) {
        let registry = Registry::new();
        let (tx, rx) = mpsc::channel(32);
        registry.register(SubscriptionKind::Market {
            epic: epic.to_owned(),
            tx,
        });
        (registry, rx)
    }

    #[tokio::test]
    async fn registry_assigns_sequential_indices() {
        let registry = Registry::new();
        let (tx1, _rx1) = mpsc::channel::<crate::streaming::events::MarketUpdate>(1);
        let (tx2, _rx2) = mpsc::channel::<crate::streaming::events::MarketUpdate>(1);
        let spec1 = registry.register(SubscriptionKind::Market {
            epic: "A".into(),
            tx: tx1,
        });
        let spec2 = registry.register(SubscriptionKind::Market {
            epic: "B".into(),
            tx: tx2,
        });
        assert_eq!(spec1.item_index, 1);
        assert_eq!(spec2.item_index, 2);
    }

    #[tokio::test]
    async fn apply_update_dispatches_to_correct_channel() {
        let (registry, mut rx) = make_market_entry("EPIC1");
        let fields = vec![FieldValue::Value("1.0".into())];
        let alive = registry.apply_update(1, &fields);
        assert!(alive);
        assert!(rx.try_recv().is_ok());
    }

    #[tokio::test]
    async fn apply_update_for_unknown_index_is_ignored() {
        let (registry, mut rx) = make_market_entry("EPIC1");
        let fields = vec![FieldValue::Value("1.0".into())];
        // Item index 99 does not exist.
        let alive = registry.apply_update(99, &fields);
        assert!(alive, "unknown index should not report dead");
        assert!(rx.try_recv().is_err(), "no message should be dispatched");
    }

    #[tokio::test]
    async fn apply_update_detects_closed_channel() {
        let registry = Registry::new();
        let (tx, rx) = mpsc::channel::<crate::streaming::events::MarketUpdate>(1);
        registry.register(SubscriptionKind::Market {
            epic: "EPIC".into(),
            tx,
        });
        drop(rx); // close the receiver

        let fields = vec![FieldValue::Value("1.0".into())];
        let alive = registry.apply_update(1, &fields);
        assert!(!alive, "should detect closed receiver");
    }

    #[tokio::test]
    async fn merge_is_applied_across_updates() {
        let registry = Registry::new();
        let (tx, mut rx) = mpsc::channel(32);
        let idx = registry
            .register(SubscriptionKind::Market {
                epic: "E".into(),
                tx,
            })
            .item_index;

        // First update: set bid.
        registry.apply_update(idx, &[FieldValue::Value("1.0".into())]);
        let u1 = rx.try_recv().unwrap();
        assert_eq!(u1.bid, Some(1.0));

        // Second update: bid unchanged, offer set.
        registry.apply_update(
            idx,
            &[FieldValue::Unchanged, FieldValue::Value("1.1".into())],
        );
        let u2 = rx.try_recv().unwrap();
        // Bid should be preserved from state.
        assert_eq!(u2.bid, Some(1.0));
        assert_eq!(u2.offer, Some(1.1));
    }

    #[tokio::test]
    async fn remove_clears_registry_entry() {
        let registry = Registry::new();
        let (tx, _rx) = mpsc::channel::<crate::streaming::events::MarketUpdate>(1);
        let idx = registry
            .register(SubscriptionKind::Market {
                epic: "E".into(),
                tx,
            })
            .item_index;

        registry.remove(idx);

        // After removal, apply_update should be a no-op (returns true — unknown index).
        let fields = vec![FieldValue::Value("1.0".into())];
        let alive = registry.apply_update(idx, &fields);
        assert!(alive, "removed entry treated as unknown (ignored)");
    }

    #[tokio::test]
    async fn snapshot_for_resubscribe_returns_all_entries() {
        let registry = Registry::new();
        let (tx1, _rx1) = mpsc::channel::<crate::streaming::events::MarketUpdate>(1);
        let (tx2, _rx2) = mpsc::channel::<crate::streaming::events::AccountUpdate>(1);
        registry.register(SubscriptionKind::Market {
            epic: "IX.D.FTSE".into(),
            tx: tx1,
        });
        registry.register(SubscriptionKind::Account {
            account_id: "ABC123".into(),
            tx: tx2,
        });

        let subs = registry.snapshot_for_resubscribe();
        assert_eq!(subs.len(), 2);

        let names: Vec<&str> = subs.iter().map(|s| s.item_name.as_str()).collect();
        assert!(names.contains(&"MARKET:IX.D.FTSE"));
        assert!(names.contains(&"ACCOUNT:ABC123"));
    }

    #[test]
    fn kind_wire_params_correct() {
        let spec = kind_wire_params(
            1,
            &SubscriptionKind::Market {
                epic: "A".into(),
                tx: tokio::sync::mpsc::channel(1).0,
            },
        );
        assert_eq!(spec.item_name, "MARKET:A");
        assert!(spec.fields.contains("BID"));
        assert_eq!(spec.mode, "MERGE");

        let spec = kind_wire_params(
            2,
            &SubscriptionKind::ChartTick {
                epic: "B".into(),
                tx: tokio::sync::mpsc::channel(1).0,
            },
        );
        assert_eq!(spec.item_name, "CHART:B:TICK");
        assert_eq!(spec.mode, "DISTINCT");

        let spec = kind_wire_params(
            3,
            &SubscriptionKind::ChartCandle {
                epic: "C".into(),
                scale: crate::streaming::events::CandleScale::Hour,
                tx: tokio::sync::mpsc::channel(1).0,
            },
        );
        assert_eq!(spec.item_name, "CHART:C:HOUR");
        assert_eq!(spec.mode, "MERGE");

        let spec = kind_wire_params(
            4,
            &SubscriptionKind::Trade {
                account_id: "D".into(),
                tx: tokio::sync::mpsc::channel(1).0,
            },
        );
        assert_eq!(spec.item_name, "TRADE:D");
        assert_eq!(spec.mode, "DISTINCT");
    }

    /// PRICE embeds the account id AND must select the `Pricing` data adapter —
    /// without `LS_data_adapter` the server serves the wrong item family.
    #[test]
    fn price_wire_params_carry_account_and_pricing_adapter() {
        let spec = kind_wire_params(
            1,
            &SubscriptionKind::Price {
                account_id: "ABC123".into(),
                epic: "CS.D.EURUSD.CFD.IP".into(),
                tx: tokio::sync::mpsc::channel(1).0,
            },
        );
        assert_eq!(spec.item_name, "PRICE:ABC123:CS.D.EURUSD.CFD.IP");
        assert_eq!(spec.mode, "MERGE");
        assert_eq!(spec.data_adapter, Some(PRICING_ADAPTER));
        assert!(spec.fields.contains("BIDPRICE1"));
        assert!(spec.fields.contains("ASKPRICE1"));
    }

    /// Every non-PRICE family must stay on the default adapter.
    #[test]
    fn non_price_wire_params_have_no_data_adapter() {
        let spec = kind_wire_params(
            1,
            &SubscriptionKind::Account {
                account_id: "D".into(),
                tx: tokio::sync::mpsc::channel(1).0,
            },
        );
        assert_eq!(spec.data_adapter, None);
    }
}
