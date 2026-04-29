#![cfg(feature = "live")]
//! Live integration tests against demo-api.ig.com.
//!
//! Run manually (read-only endpoints):
//!
//! ```bash
//! IG_API_KEY=... IG_USERNAME=... IG_PASSWORD=... \
//!   cargo test --features live --ignored --test live_integration
//! ```
//!
//! Write / mutation tests (watchlist CRUD, preferences round-trip) additionally
//! require the `live-trading` feature AND `IG_LIVE_TRADING_OK=1`:
//!
//! ```bash
//! IG_API_KEY=... IG_USERNAME=... IG_PASSWORD=... IG_LIVE_TRADING_OK=1 \
//!   cargo test --features live-trading --ignored --test live_integration
//! ```
//!
//! All tests are `#[ignore]` so that `cargo test` (plain or `--all-features`)
//! never runs them in CI.  Every test also checks `IG_API_KEY` at runtime and
//! returns early (graceful skip) when credentials are absent.

#[cfg(feature = "live-trading")]
use trading_ig::accounts::UpdatePreferences;
use trading_ig::history::{ActivityRequest, TransactionsRequest};
use trading_ig::markets::models::MarketDetailFilter;
use trading_ig::models::common::Epic;
use trading_ig::prices::models::{HistoricalPricesRequest, Resolution};
#[cfg(feature = "live-trading")]
use trading_ig::watchlists::CreateWatchlistRequest;
use trading_ig::{Credentials, Environment, IgClient};

// ── Shared helpers ────────────────────────────────────────────────────────────

/// Returns `Some(())` when all three required env-vars are present,
/// `None` otherwise.  Each test uses `let Some(()) = creds_present() else { return };`
/// to skip gracefully when credentials are absent.
fn creds_present() -> Option<()> {
    if std::env::var("IG_API_KEY").is_ok()
        && std::env::var("IG_USERNAME").is_ok()
        && std::env::var("IG_PASSWORD").is_ok()
    {
        Some(())
    } else {
        None
    }
}

/// Build an `IgClient` pointed at the Demo environment using env-var credentials.
///
/// # Panics
///
/// Panics if the required environment variables (`IG_API_KEY`, `IG_USERNAME`,
/// `IG_PASSWORD`) are missing or if the client cannot be constructed.
fn build_client() -> IgClient {
    let api_key = std::env::var("IG_API_KEY").expect("IG_API_KEY must be set");
    let username = std::env::var("IG_USERNAME").expect("IG_USERNAME must be set");
    let password = std::env::var("IG_PASSWORD").expect("IG_PASSWORD must be set");

    IgClient::builder()
        .environment(Environment::Demo)
        .api_key(api_key)
        .credentials(Credentials::password(username, password))
        .build()
        .expect("failed to build IgClient")
}

/// Log in with the v3 flow and return the `SessionInfo`. Panics on failure.
async fn login(client: &IgClient) -> trading_ig::SessionInfo {
    client
        .session()
        .login()
        .await
        .expect("session().login() failed")
}

// ── Session tests (read-only) ─────────────────────────────────────────────────

#[tokio::test]
#[ignore = "live network test — requires IG_API_KEY, IG_USERNAME, IG_PASSWORD in the environment"]
async fn live_session_login_v3() {
    let Some(()) = creds_present() else { return };
    let client = build_client();
    let info = login(&client).await;
    assert!(!info.account_id.is_empty(), "account_id must be non-empty");
    assert!(!info.client_id.is_empty(), "client_id must be non-empty");
    assert!(
        !info.lightstreamer_endpoint.is_empty(),
        "lightstreamer_endpoint must be non-empty"
    );
}

#[tokio::test]
#[ignore = "live network test — requires IG_API_KEY, IG_USERNAME, IG_PASSWORD in the environment"]
async fn live_session_login_v2() {
    let Some(()) = creds_present() else { return };
    let client = build_client();
    let info = client.session().login_v2().await.expect("login_v2 failed");
    assert!(!info.account_id.is_empty());
    assert!(!info.client_id.is_empty());
}

#[tokio::test]
#[ignore = "live network test — requires IG_API_KEY, IG_USERNAME, IG_PASSWORD in the environment"]
async fn live_session_refresh() {
    let Some(()) = creds_present() else { return };
    let client = build_client();
    login(&client).await;
    // Refresh the OAuth access token — should not error.
    client.session().refresh().await.expect("refresh failed");
    // Confirm we can still make an authenticated call after the refresh.
    client
        .accounts()
        .list()
        .await
        .expect("accounts.list after refresh");
}

#[tokio::test]
#[ignore = "live network test — requires IG_API_KEY, IG_USERNAME, IG_PASSWORD in the environment"]
async fn live_session_read_no_tokens() {
    let Some(()) = creds_present() else { return };
    let client = build_client();
    login(&client).await;
    let details = client
        .session()
        .read(false)
        .await
        .expect("session.read(false)");
    assert!(!details.account_id.is_empty());
    assert!(!details.client_id.is_empty());
}

#[tokio::test]
#[ignore = "live network test — requires IG_API_KEY, IG_USERNAME, IG_PASSWORD in the environment"]
async fn live_session_read_with_tokens() {
    let Some(()) = creds_present() else { return };
    let client = build_client();
    login(&client).await;
    // Fetching with fetch_tokens=true swaps in CST/XST and writes them into
    // local state.  Verify by making a follow-up authenticated call.
    let details = client
        .session()
        .read(true)
        .await
        .expect("session.read(true)");
    assert!(!details.account_id.is_empty());
    // Follow-up call to confirm the session is still usable.
    client
        .accounts()
        .list()
        .await
        .expect("accounts.list after read(true)");
}

#[tokio::test]
#[ignore = "live network test — requires IG_API_KEY, IG_USERNAME, IG_PASSWORD in the environment"]
async fn live_session_logout() {
    let Some(()) = creds_present() else { return };
    let client = build_client();
    login(&client).await;
    client.session().logout().await.expect("logout failed");
}

// ── Accounts tests (read-only) ────────────────────────────────────────────────

#[tokio::test]
#[ignore = "live network test — requires IG_API_KEY, IG_USERNAME, IG_PASSWORD in the environment"]
async fn live_accounts_list() {
    let Some(()) = creds_present() else { return };
    let client = build_client();
    login(&client).await;
    let accounts = client.accounts().list().await.expect("accounts.list");
    assert!(!accounts.is_empty(), "expected at least one account");
    assert!(
        accounts.iter().all(|a| !a.account_id.is_empty()),
        "every account must have a non-empty account_id"
    );
}

#[tokio::test]
#[ignore = "live network test — requires IG_API_KEY, IG_USERNAME, IG_PASSWORD in the environment"]
async fn live_accounts_preferences() {
    let Some(()) = creds_present() else { return };
    let client = build_client();
    login(&client).await;
    // Just assert the call succeeds; the actual bool value depends on account config.
    let _prefs = client
        .accounts()
        .preferences()
        .await
        .expect("accounts.preferences");
}

// ── Markets tests (read-only) ─────────────────────────────────────────────────

#[tokio::test]
#[ignore = "live network test — requires IG_API_KEY, IG_USERNAME, IG_PASSWORD in the environment"]
async fn live_markets_search() {
    let Some(()) = creds_present() else { return };
    let client = build_client();
    login(&client).await;
    let results = client
        .markets()
        .search("EUR")
        .await
        .expect("markets.search");
    assert!(
        !results.is_empty(),
        "expected at least one result for 'EUR'"
    );
}

#[tokio::test]
#[ignore = "live network test — requires IG_API_KEY, IG_USERNAME, IG_PASSWORD in the environment"]
async fn live_markets_get_single() {
    let Some(()) = creds_present() else { return };
    let client = build_client();
    login(&client).await;

    let results = client
        .markets()
        .search("EUR")
        .await
        .expect("markets.search");
    let epic = results.first().expect("no results").epic.clone();

    let details = client.markets().get(&epic).await.expect("markets.get");
    assert_eq!(details.instrument.epic, epic);
}

#[tokio::test]
#[ignore = "live network test — requires IG_API_KEY, IG_USERNAME, IG_PASSWORD in the environment"]
async fn live_markets_get_many() {
    let Some(()) = creds_present() else { return };
    let client = build_client();
    login(&client).await;

    let results = client
        .markets()
        .search("EUR")
        .await
        .expect("markets.search");
    // Take up to two epics from the search results.
    let epics: Vec<Epic> = results.iter().take(2).map(|m| m.epic.clone()).collect();
    assert!(epics.len() >= 2, "need at least 2 results to test get_many");

    let many = client
        .markets()
        .get_many(&epics, MarketDetailFilter::All)
        .await
        .expect("markets.get_many");
    assert_eq!(many.len(), epics.len());
}

#[tokio::test]
#[ignore = "live network test — requires IG_API_KEY, IG_USERNAME, IG_PASSWORD in the environment"]
async fn live_markets_navigation() {
    let Some(()) = creds_present() else { return };
    let client = build_client();
    login(&client).await;
    let nav = client
        .markets()
        .navigation()
        .await
        .expect("markets.navigation");
    // Top-level navigation contains child nodes (no markets at root level).
    assert!(
        !nav.nodes.is_empty(),
        "expected at least some navigation nodes"
    );
}

// ── Prices tests (read-only) ──────────────────────────────────────────────────

/// Known EUR/USD rolling cash epic — works on both demo and live.
const EURUSD_EPIC: &str = "CS.D.EURUSD.TODAY.IP";

#[tokio::test]
#[ignore = "live network test — requires IG_API_KEY, IG_USERNAME, IG_PASSWORD in the environment"]
async fn live_prices_history_v3() {
    let Some(()) = creds_present() else { return };
    let client = build_client();
    login(&client).await;

    let epic = Epic::new(EURUSD_EPIC);
    let now = chrono::Utc::now().naive_utc();
    let one_hour_ago = now - chrono::Duration::hours(1);

    let req = HistoricalPricesRequest {
        resolution: Some(Resolution::Minute),
        from: Some(one_hour_ago),
        to: Some(now),
        ..HistoricalPricesRequest::default()
    };

    let prices = client
        .prices()
        .history_v3(&epic, req)
        .await
        .expect("prices.history_v3");
    assert!(!prices.prices.is_empty(), "expected at least one bar");
}

#[tokio::test]
#[ignore = "live network test — requires IG_API_KEY, IG_USERNAME, IG_PASSWORD in the environment"]
async fn live_prices_history_by_num_points_v2() {
    let Some(()) = creds_present() else { return };
    let client = build_client();
    login(&client).await;

    let epic = Epic::new(EURUSD_EPIC);
    let prices = client
        .prices()
        .history_by_num_points_v2(&epic, Resolution::Hour, 5)
        .await
        .expect("prices.history_by_num_points_v2");
    assert!(prices.prices.len() <= 5, "should return at most 5 bars");
}

// ── Dealing tests (read-only) ─────────────────────────────────────────────────

#[tokio::test]
#[ignore = "live network test — requires IG_API_KEY, IG_USERNAME, IG_PASSWORD in the environment"]
async fn live_dealing_positions_list() {
    let Some(()) = creds_present() else { return };
    let client = build_client();
    login(&client).await;
    // Just assert the call succeeds; position count varies per account.
    let _positions = client
        .dealing()
        .positions()
        .list()
        .await
        .expect("dealing.positions.list");
}

#[tokio::test]
#[ignore = "live network test — requires IG_API_KEY, IG_USERNAME, IG_PASSWORD in the environment"]
async fn live_dealing_working_orders_list() {
    let Some(()) = creds_present() else { return };
    let client = build_client();
    login(&client).await;
    // Just assert the call succeeds; working order count varies per account.
    let _orders = client
        .dealing()
        .working_orders()
        .list()
        .await
        .expect("dealing.working_orders.list");
}

// ── Watchlists tests (read-only) ──────────────────────────────────────────────

#[tokio::test]
#[ignore = "live network test — requires IG_API_KEY, IG_USERNAME, IG_PASSWORD in the environment"]
async fn live_watchlists_list() {
    let Some(()) = creds_present() else { return };
    let client = build_client();
    login(&client).await;
    // Just assert the call succeeds; watchlist count varies per account.
    let _watchlists = client.watchlists().list().await.expect("watchlists.list");
}

// ── Client sentiment tests (read-only) ────────────────────────────────────────

/// Known FTSE market-id that reliably has sentiment data.
const FTSE_MARKET_ID: &str = "IX.D.FTSE.DAILY.IP";

#[tokio::test]
#[ignore = "live network test — requires IG_API_KEY, IG_USERNAME, IG_PASSWORD in the environment"]
async fn live_client_sentiment_get() {
    let Some(()) = creds_present() else { return };
    let client = build_client();
    login(&client).await;
    let sentiment = client
        .client_sentiment()
        .get(FTSE_MARKET_ID)
        .await
        .expect("client_sentiment.get");
    assert_eq!(sentiment.market_id, FTSE_MARKET_ID);
    // Long + short should sum to ~100 %.
    let total = sentiment.long_position_percentage + sentiment.short_position_percentage;
    assert!(
        (total - 100.0).abs() < 1.0,
        "long + short = {total:.2}, expected ~100"
    );
}

// ── History tests (read-only) ─────────────────────────────────────────────────

#[tokio::test]
#[ignore = "live network test — requires IG_API_KEY, IG_USERNAME, IG_PASSWORD in the environment"]
async fn live_history_activity_v3() {
    let Some(()) = creds_present() else { return };
    let client = build_client();
    login(&client).await;
    // Default request — no filters, should return without error.
    let _activities = client
        .history()
        .activity_v3(ActivityRequest::default())
        .await
        .expect("history.activity_v3");
}

#[tokio::test]
#[ignore = "live network test — requires IG_API_KEY, IG_USERNAME, IG_PASSWORD in the environment"]
async fn live_history_transactions_v2() {
    let Some(()) = creds_present() else { return };
    let client = build_client();
    login(&client).await;
    // Default request — no filters, should return without error.
    let _txns = client
        .history()
        .transactions_v2(TransactionsRequest::default())
        .await
        .expect("history.transactions_v2");
}

// ── Write / mutation tests (live-trading feature + IG_LIVE_TRADING_OK=1) ──────
//
// These tests create and then clean up real resources on the demo account.
// They are gated behind:
//   1. `#[cfg(feature = "live-trading")]` — compile-time gate.
//   2. `IG_LIVE_TRADING_OK=1` env-var at runtime — belt-and-suspenders guard.

#[cfg(feature = "live-trading")]
fn trading_ok() -> bool {
    std::env::var("IG_LIVE_TRADING_OK").as_deref() == Ok("1")
}

/// Create a watchlist with a timestamped unique name, assert it appears in the
/// list, then delete it.  Leaves no persistent state.
#[cfg(feature = "live-trading")]
#[tokio::test]
#[ignore = "live network test — requires IG_API_KEY, IG_USERNAME, IG_PASSWORD in the environment"]
async fn live_watchlists_create_and_delete() {
    let Some(()) = creds_present() else { return };
    if !trading_ok() {
        eprintln!(
            "skipping live_watchlists_create_and_delete — set IG_LIVE_TRADING_OK=1 to enable"
        );
        return;
    }

    let client = build_client();
    login(&client).await;

    let name = format!(
        "live-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    );

    // Create
    let created = client
        .watchlists()
        .create(CreateWatchlistRequest {
            name: name.clone(),
            epics: vec![],
        })
        .await
        .expect("watchlists.create");
    let watchlist_id = created.watchlist_id.clone();

    // Verify it appears in the list.
    let all = client
        .watchlists()
        .list()
        .await
        .expect("watchlists.list after create");
    assert!(
        all.iter().any(|w| w.id == watchlist_id),
        "newly created watchlist '{name}' not found in list"
    );

    // Delete
    client
        .watchlists()
        .delete(&watchlist_id)
        .await
        .expect("watchlists.delete");

    // Verify it no longer appears.
    let after_delete = client
        .watchlists()
        .list()
        .await
        .expect("watchlists.list after delete");
    assert!(
        !after_delete.iter().any(|w| w.id == watchlist_id),
        "watchlist '{name}' still present after delete"
    );
}

/// Read the current account preferences, write the same value back, then read
/// again to confirm the round-trip.  This is safe because it preserves the
/// existing setting.
#[cfg(feature = "live-trading")]
#[tokio::test]
#[ignore = "live network test — requires IG_API_KEY, IG_USERNAME, IG_PASSWORD in the environment"]
async fn live_accounts_preferences_round_trip() {
    let Some(()) = creds_present() else { return };
    if !trading_ok() {
        eprintln!(
            "skipping live_accounts_preferences_round_trip — set IG_LIVE_TRADING_OK=1 to enable"
        );
        return;
    }

    let client = build_client();
    login(&client).await;

    // Read current value.
    let prefs = client
        .accounts()
        .preferences()
        .await
        .expect("accounts.preferences (read)");
    let original = prefs.trailing_stops_enabled;

    // Write the same value back (no actual change).
    let updated = client
        .accounts()
        .update_preferences(UpdatePreferences {
            trailing_stops_enabled: original,
        })
        .await
        .expect("accounts.update_preferences");
    assert_eq!(
        updated.trailing_stops_enabled, original,
        "preferences round-trip mismatch"
    );

    // Read again to confirm persistence.
    let confirmed = client
        .accounts()
        .preferences()
        .await
        .expect("accounts.preferences (confirm)");
    assert_eq!(confirmed.trailing_stops_enabled, original);
}

// ── Order skeletons (disabled by default) ─────────────────────────────────────
//
// The tests below are intentionally left as compile-time stubs that will never
// run unless a human explicitly enables them and adds the dealing logic.
// They exist to document the intended coverage surface.

// FIXME: requires explicit human approval before enabling — would place a real
// order on the demo account.  Un-comment and fill in the fields when ready.
//
// #[cfg(feature = "live-trading")]
// #[tokio::test]
// #[ignore]
// async fn live_open_and_close_position() {
//     // 1. login
//     // 2. client.dealing().positions().open()
//     //        .epic(...).direction(Direction::Buy).size(1.0)
//     //        .order_type(OrderType::Market).currency("GBP").expiry("DFB")
//     //        .guaranteed_stop(false).force_open(false)
//     //        .send().await?;
//     // 3. assert deal_status == DealStatus::Accepted
//     // 4. client.dealing().positions().close(...).await?
// }
//
// #[cfg(feature = "live-trading")]
// #[tokio::test]
// #[ignore]
// async fn live_create_and_delete_working_order() {
//     // 1. login
//     // 2. client.dealing().working_orders().create()
//     //        .epic(...).direction(Direction::Buy).size(1.0)
//     //        .order_type(OrderType::Limit).currency("GBP").expiry("DFB")
//     //        .level(1.0).time_in_force(TimeInForce::GoodTillCancelled)
//     //        .guaranteed_stop(false)
//     //        .send().await?;
//     // 3. assert deal_status == DealStatus::Accepted
//     // 4. client.dealing().working_orders().delete(&deal_id).await?
// }
