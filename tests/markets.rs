//! Integration tests for the markets domain.
//!
//! Each test spins up its own `wiremock::MockServer` (directly or via
//! `IgMockServer`) so that `cargo test` can run them in parallel without
//! interference.

mod support;

use trading_ig::markets::models::{
    InstrumentType, MarketDetailFilter, MarketStatus, NavigationChild,
};
use trading_ig::models::common::Epic;
use trading_ig::{Credentials, Environment, Error, IgClient};
use wiremock::matchers::{header_exists, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use support::fixtures;
use support::matchers::{HasApiKey, HasVersion};
use support::mock_server::IgMockServer;

// ---------------------------------------------------------------------------
// Helper: spin up a raw MockServer with a login + custom mock,
//         then return the client.
// ---------------------------------------------------------------------------

/// Build an `IgClient` pointed at `server` and mount a v3 login on `server`.
async fn client_with_login(server: &MockServer) -> IgClient {
    // Mount v3 login
    let login_body = fixtures::load("session/login_v3.json");
    Mock::given(method("POST"))
        .and(path("/session"))
        .and(header_exists("X-IG-API-KEY"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(login_body),
        )
        .mount(server)
        .await;

    let base_url = url::Url::parse(&server.uri()).expect("valid mock url");
    let client = IgClient::builder()
        .environment(Environment::Custom(base_url))
        .api_key("test-api-key")
        .credentials(Credentials::password("test-user", "test-pass"))
        .build()
        .expect("client builds");

    client.session().login().await.expect("login");
    client
}

// ============================================================================
// Search (v1)
// ============================================================================

#[tokio::test]
async fn search_returns_market_summaries() {
    let server = MockServer::start().await;

    // Mount a fixture for the search request
    let body = fixtures::load("markets/search_eurusd_v1.json");
    Mock::given(method("GET"))
        .and(path("/markets"))
        .and(query_param("searchTerm", "EUR/USD"))
        .and(HasApiKey)
        .and(HasVersion(1))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(body),
        )
        .mount(&server)
        .await;

    let client = client_with_login(&server).await;
    let results = client.markets().search("EUR/USD").await.expect("search ok");

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].epic, Epic::new("CS.D.EURUSD.TODAY.IP"));
    assert_eq!(results[0].instrument_name, "EUR/USD");
    assert_eq!(results[0].instrument_type, InstrumentType::Currencies);
    assert_eq!(results[0].market_status, MarketStatus::Tradeable);
    assert!(results[0].streaming_prices_available);
    assert!((results[0].bid.unwrap() - 1.1220).abs() < 1e-6);
    assert!((results[0].offer.unwrap() - 1.1222).abs() < 1e-6);
}

#[tokio::test]
async fn search_returns_empty_vec_when_no_matches() {
    let server = MockServer::start().await;

    let body = fixtures::load("markets/search_empty_v1.json");
    Mock::given(method("GET"))
        .and(path("/markets"))
        .and(query_param("searchTerm", "ZZZNOMATCH"))
        .and(HasApiKey)
        .and(HasVersion(1))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(body),
        )
        .mount(&server)
        .await;

    let client = client_with_login(&server).await;
    let results = client
        .markets()
        .search("ZZZNOMATCH")
        .await
        .expect("search ok");
    assert!(results.is_empty(), "expected empty results");
}

// ============================================================================
// Bulk fetch (v2)
// ============================================================================

#[tokio::test]
async fn get_many_returns_multiple_market_details() {
    let server = MockServer::start().await;

    let body = fixtures::load("markets/get_many_v2.json");
    Mock::given(method("GET"))
        .and(path("/markets"))
        .and(query_param(
            "epics",
            "CS.D.EURUSD.TODAY.IP,IX.D.FTSE.DAILY.IP",
        ))
        .and(query_param("filter", "ALL"))
        .and(HasApiKey)
        .and(HasVersion(2))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(body),
        )
        .mount(&server)
        .await;

    let client = client_with_login(&server).await;
    let epics = vec![
        Epic::new("CS.D.EURUSD.TODAY.IP"),
        Epic::new("IX.D.FTSE.DAILY.IP"),
    ];
    let details = client
        .markets()
        .get_many(&epics, MarketDetailFilter::All)
        .await
        .expect("get_many ok");

    assert_eq!(details.len(), 2);
    assert_eq!(details[0].instrument.epic, Epic::new("CS.D.EURUSD.TODAY.IP"));
    assert_eq!(
        details[0].instrument.instrument_type,
        InstrumentType::Currencies
    );
    assert_eq!(details[0].snapshot.market_status, MarketStatus::Tradeable);
    assert!((details[0].snapshot.bid.unwrap() - 1.1220).abs() < 1e-6);
    assert_eq!(details[1].instrument.epic, Epic::new("IX.D.FTSE.DAILY.IP"));
    assert_eq!(
        details[1].instrument.instrument_type,
        InstrumentType::Indices
    );
}

#[tokio::test]
async fn get_many_snapshot_only_filter() {
    let server = MockServer::start().await;

    let body = fixtures::load("markets/get_many_snapshot_only_v2.json");
    Mock::given(method("GET"))
        .and(path("/markets"))
        .and(query_param("epics", "CS.D.EURUSD.TODAY.IP"))
        .and(query_param("filter", "SNAPSHOT_ONLY"))
        .and(HasApiKey)
        .and(HasVersion(2))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(body),
        )
        .mount(&server)
        .await;

    let client = client_with_login(&server).await;
    let epics = vec![Epic::new("CS.D.EURUSD.TODAY.IP")];
    let details = client
        .markets()
        .get_many(&epics, MarketDetailFilter::SnapshotOnly)
        .await
        .expect("get_many snapshot_only ok");

    assert_eq!(details.len(), 1);
    assert_eq!(details[0].snapshot.scaling_factor, Some(1));
}

#[tokio::test]
async fn get_many_empty_epics_returns_invalid_input() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let err = client
        .markets()
        .get_many(&[], MarketDetailFilter::All)
        .await
        .expect_err("should fail on empty epics");

    assert!(
        matches!(err, Error::InvalidInput(_)),
        "expected InvalidInput, got {err:?}"
    );
}

// ============================================================================
// Single market (v3)
// ============================================================================

#[tokio::test]
async fn get_single_market_returns_details() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_get(
        "markets/CS.D.GBPUSD.TODAY.IP",
        3,
        "markets/get_v3.json",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let epic = Epic::new("CS.D.GBPUSD.TODAY.IP");
    let details = client.markets().get(&epic).await.expect("get ok");

    assert_eq!(details.instrument.epic, epic);
    assert_eq!(
        details.instrument.instrument_type,
        InstrumentType::Currencies
    );
    assert_eq!(details.snapshot.market_status, MarketStatus::Tradeable);
    assert_eq!(details.instrument.currencies.len(), 2);
    assert!(details.instrument.currencies[0].is_default);
    assert!((details.snapshot.bid.unwrap() - 1.2700).abs() < 1e-6);
    assert!((details.snapshot.controlled_risk_extra_spread.unwrap_or(0.0) - 0.8).abs() < 1e-6);
}

#[tokio::test]
async fn get_single_market_not_found_returns_api_error() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_error(
        "GET",
        "markets/NONEXISTENT.EPIC",
        404,
        "error.public-api.failure.market.not-found",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let epic = Epic::new("NONEXISTENT.EPIC");
    let err = client
        .markets()
        .get(&epic)
        .await
        .expect_err("should be not found");

    match err {
        Error::Api { status, source } => {
            assert_eq!(status.as_u16(), 404);
            assert_eq!(
                source.error_code,
                "error.public-api.failure.market.not-found"
            );
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}

// ============================================================================
// Navigation (v1)
// ============================================================================

#[tokio::test]
async fn navigation_root_returns_nodes_only() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_get("marketnavigation", 1, "markets/navigation_root_v1.json")
        .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let root = client.markets().navigation().await.expect("navigation ok");

    assert!(root.markets.is_empty(), "root node should have no markets");
    assert_eq!(root.nodes.len(), 4);

    let names: Vec<&str> = root.nodes.iter().map(|n| n.name.as_str()).collect();
    assert!(names.contains(&"Forex"));
    assert!(names.contains(&"Indices"));

    let forex_node: &NavigationChild = root
        .nodes
        .iter()
        .find(|n| n.name == "Forex")
        .expect("forex node exists");
    assert_eq!(forex_node.id, "668394");
}

#[tokio::test]
async fn navigation_leaf_returns_markets_only() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_get(
        "marketnavigation/668394",
        1,
        "markets/navigation_leaf_v1.json",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let leaf = client
        .markets()
        .navigation_node("668394")
        .await
        .expect("navigation_node ok");

    assert!(leaf.nodes.is_empty(), "leaf node should have no children");
    assert_eq!(leaf.markets.len(), 1);
    assert_eq!(leaf.markets[0].epic, Epic::new("CS.D.EURUSD.TODAY.IP"));
    assert_eq!(leaf.markets[0].instrument_type, InstrumentType::Currencies);
    assert_eq!(leaf.markets[0].market_status, MarketStatus::Tradeable);
}
