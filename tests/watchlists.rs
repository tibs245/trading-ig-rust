//! Integration tests for the watchlists domain.
//!
//! All tests run against a local wiremock instance — no live IG API calls.

mod support;

use support::mock_server::IgMockServer;
use trading_ig::watchlists::{CreateWatchlistRequest, CreateWatchlistStatus};
use trading_ig::models::common::Epic;

// ---------------------------------------------------------------------------
// list
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_watchlists_golden() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_get("watchlists", 1, "watchlists/list_v1.json").await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let watchlists = client.watchlists().list().await.expect("list watchlists");

    assert_eq!(watchlists.len(), 2);

    let editable = &watchlists[0];
    assert_eq!(editable.id, "12345678");
    assert_eq!(editable.name, "My Watchlist");
    assert!(editable.editable);
    assert!(editable.deleteable);
    assert!(!editable.default_system_watchlist);

    let system = &watchlists[1];
    assert_eq!(system.id, "system-01");
    assert_eq!(system.name, "Popular Markets");
    assert!(!system.editable);
    assert!(!system.deleteable);
    assert!(system.default_system_watchlist);
}

// ---------------------------------------------------------------------------
// markets
// ---------------------------------------------------------------------------

#[tokio::test]
async fn markets_in_watchlist_golden() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_get("watchlists/12345678", 1, "watchlists/markets_v1.json").await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let markets = client
        .watchlists()
        .markets("12345678")
        .await
        .expect("markets");

    assert_eq!(markets.len(), 2);

    let gbp = &markets[0];
    assert_eq!(gbp.epic.as_str(), "CS.D.GBPUSD.TODAY.IP");
    assert_eq!(gbp.instrument_name, "GBP/USD");
    assert_eq!(gbp.market_status, "TRADEABLE");
    assert!(gbp.streaming_prices_available);
    assert!((gbp.bid.unwrap() - 1.265_f64).abs() < 1e-9);

    let eur = &markets[1];
    assert_eq!(eur.epic.as_str(), "CS.D.EURUSD.TODAY.IP");
}

// ---------------------------------------------------------------------------
// create (SUCCESS)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn create_watchlist_success() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_post("watchlists", 1, "watchlists/create_v1.json").await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let req = CreateWatchlistRequest {
        name: "Test Watchlist".to_string(),
        epics: vec![Epic::new("CS.D.GBPUSD.TODAY.IP")],
    };

    let resp = client.watchlists().create(req).await.expect("create");
    assert_eq!(resp.watchlist_id, "87654321");
    assert_eq!(resp.status, CreateWatchlistStatus::Success);
}

// ---------------------------------------------------------------------------
// create (SUCCESS_NOT_ALL_INSTRUMENTS_ADDED)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn create_watchlist_partial_success() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_post("watchlists", 1, "watchlists/create_partial_v1.json").await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let req = CreateWatchlistRequest {
        name: "Partial Watchlist".to_string(),
        epics: vec![
            Epic::new("CS.D.GBPUSD.TODAY.IP"),
            Epic::new("INVALID.EPIC.XXX"),
        ],
    };

    let resp = client.watchlists().create(req).await.expect("create partial");
    assert_eq!(resp.watchlist_id, "87654322");
    assert_eq!(resp.status, CreateWatchlistStatus::SuccessNotAllInstrumentsAdded);
}

// ---------------------------------------------------------------------------
// add_market
// ---------------------------------------------------------------------------

#[tokio::test]
async fn add_market_golden() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_put("watchlists/12345678", 1, "watchlists/add_market_v1.json").await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let epic = Epic::new("CS.D.GBPUSD.TODAY.IP");
    let resp = client
        .watchlists()
        .add_market("12345678", &epic)
        .await
        .expect("add market");

    assert_eq!(resp.status, "SUCCESS");
}

// ---------------------------------------------------------------------------
// remove_market
// ---------------------------------------------------------------------------

#[tokio::test]
async fn remove_market_golden() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    // Path: watchlists/{id}/{epic} — epic dots are part of the raw path
    mock.mount_delete_ok("watchlists/12345678/CS.D.GBPUSD.TODAY.IP", 1).await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let epic = Epic::new("CS.D.GBPUSD.TODAY.IP");
    client
        .watchlists()
        .remove_market("12345678", &epic)
        .await
        .expect("remove market");
}

// ---------------------------------------------------------------------------
// delete
// ---------------------------------------------------------------------------

#[tokio::test]
async fn delete_watchlist_golden() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_delete_ok("watchlists/12345678", 1).await;

    let client = mock.client();
    client.session().login().await.expect("login");

    client
        .watchlists()
        .delete("12345678")
        .await
        .expect("delete watchlist");
}

// ---------------------------------------------------------------------------
// API error
// ---------------------------------------------------------------------------

#[tokio::test]
async fn create_watchlist_name_too_long_returns_api_error() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_error(
        "POST",
        "watchlists",
        400,
        "error.invalid.watchlist.name.too.long",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let req = CreateWatchlistRequest {
        name: "A".repeat(200),
        epics: vec![],
    };

    let err = client
        .watchlists()
        .create(req)
        .await
        .expect_err("should fail");

    match err {
        trading_ig::Error::Api { status, source } => {
            assert_eq!(status.as_u16(), 400);
            assert_eq!(source.error_code, "error.invalid.watchlist.name.too.long");
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Schema edge case: optional price fields missing (nullable market)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn markets_with_null_prices() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;

    // Build a fixture body inline with no bid/offer
    let body = r#"{
        "markets": [{
            "instrumentName": "Closed Market",
            "expiry": "DFB",
            "epic": "IX.D.DAX.DAILY.IP",
            "instrumentType": "INDICES",
            "bid": null,
            "offer": null,
            "high": null,
            "low": null,
            "percentageChange": null,
            "netChange": null,
            "updateTime": null,
            "updateTimeUTC": null,
            "streamingPricesAvailable": false,
            "marketStatus": "CLOSED",
            "scalingFactor": null
        }]
    }"#;

    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("watchlists/closed-list"))
        .respond_with(
            wiremock::ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(body),
        )
        .mount(mock.server_ref())
        .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let markets = client
        .watchlists()
        .markets("closed-list")
        .await
        .expect("markets with nulls");

    assert_eq!(markets.len(), 1);
    let m = &markets[0];
    assert_eq!(m.epic.as_str(), "IX.D.DAX.DAILY.IP");
    assert_eq!(m.market_status, "CLOSED");
    assert!(m.bid.is_none());
    assert!(m.offer.is_none());
    assert!(!m.streaming_prices_available);
}
