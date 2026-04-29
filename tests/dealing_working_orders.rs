//! Integration tests for the `dealing/working_orders` domain.
//!
//! All tests run against a local wiremock instance — no live IG API is used.

#![allow(clippy::doc_markdown, clippy::float_cmp, clippy::redundant_closure_for_method_calls)]

mod support;

use support::fixtures;
use support::matchers::{HasApiKey, HasVersion};
use trading_ig::dealing::working_orders::models::UpdateWorkingOrderRequest;
use trading_ig::dealing::DealStatus;
use trading_ig::models::common::{Currency, DealId, Direction, Epic, OrderType, TimeInForce};
use trading_ig::{Credentials, Environment, IgClient};
use url::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── Test-local helpers ────────────────────────────────────────────────────────

/// Start a wiremock server + build an `IgClient` pointing at it.
async fn start_server() -> (MockServer, IgClient) {
    let server = MockServer::start().await;
    let base = Url::parse(&server.uri()).expect("valid url");
    let client = IgClient::builder()
        .environment(Environment::Custom(base))
        .api_key("test-api-key")
        .credentials(Credentials::password("test-user", "test-pass"))
        .build()
        .expect("client builds");
    (server, client)
}

/// Mount the v3 login fixture.
async fn mount_login_v3(server: &MockServer) {
    let body = fixtures::load("session/login_v3.json");
    Mock::given(method("POST"))
        .and(path("session"))
        .and(HasApiKey)
        .and(HasVersion(3))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(body),
        )
        .mount(server)
        .await;
}

/// Mount a GET that returns `fixture`.
async fn mount_get(server: &MockServer, path_str: &str, version: u8, fixture: &str) {
    let body = fixtures::load(fixture);
    Mock::given(method("GET"))
        .and(path(path_str))
        .and(HasApiKey)
        .and(HasVersion(version))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(body),
        )
        .mount(server)
        .await;
}

/// Mount a POST that returns `fixture`.
async fn mount_post(server: &MockServer, path_str: &str, version: u8, fixture: &str) {
    let body = fixtures::load(fixture);
    Mock::given(method("POST"))
        .and(path(path_str))
        .and(HasApiKey)
        .and(HasVersion(version))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(body),
        )
        .mount(server)
        .await;
}

/// Mount a PUT that returns `fixture`.
async fn mount_put(server: &MockServer, path_str: &str, version: u8, fixture: &str) {
    let body = fixtures::load(fixture);
    Mock::given(method("PUT"))
        .and(path(path_str))
        .and(HasApiKey)
        .and(HasVersion(version))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(body),
        )
        .mount(server)
        .await;
}

/// Mount a DELETE that returns `fixture`.
async fn mount_delete(server: &MockServer, path_str: &str, version: u8, fixture: &str) {
    let body = fixtures::load(fixture);
    Mock::given(method("DELETE"))
        .and(path(path_str))
        .and(HasApiKey)
        .and(HasVersion(version))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(body),
        )
        .mount(server)
        .await;
}

/// Mount an IG-style error response.
async fn mount_error(server: &MockServer, http_method: &str, path_str: &str, status: u16, error_code: &str) {
    Mock::given(method(http_method))
        .and(path(path_str))
        .and(HasApiKey)
        .respond_with(
            ResponseTemplate::new(status)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(format!(r#"{{"errorCode":"{error_code}"}}"#)),
        )
        .mount(server)
        .await;
}

// ── list_v1 ───────────────────────────────────────────────────────────────────

/// list_v1 returns the correct typed entries from the v1 envelope.
#[tokio::test]
async fn list_v1_golden_path() {
    let (server, client) = start_server().await;
    mount_login_v3(&server).await;
    mount_get(
        &server,
        "workingorders",
        1,
        "dealing/working_orders/list_v1.json",
    )
    .await;

    client.session().login().await.expect("login");

    let orders = client
        .dealing()
        .working_orders()
        .list_v1()
        .await
        .expect("list_v1 succeeds");

    assert_eq!(orders.len(), 1);
    let o = &orders[0];
    assert_eq!(o.order_data.deal_id.as_str(), "DIAAAAA111111111");
    assert_eq!(o.order_data.direction, Direction::Buy);
    assert_eq!(o.order_data.order_type, OrderType::Limit);
    assert_eq!(o.order_data.order_size, 1.0);
    assert_eq!(o.order_data.order_level, 1.2345);
    assert_eq!(o.market.epic.as_ref().map(|e| e.as_str()), Some("CS.D.GBPUSD.TODAY.IP"));
    assert_eq!(o.market.instrument_name.as_deref(), Some("GBP/USD"));
}

/// list_v1 API error surfaces the IG errorCode correctly.
#[tokio::test]
async fn list_v1_api_error_surfaces_error_code() {
    let (server, client) = start_server().await;
    mount_login_v3(&server).await;
    mount_error(
        &server,
        "GET",
        "workingorders",
        400,
        "error.public-api.failure.kyc.required",
    )
    .await;

    client.session().login().await.expect("login");

    let err = client
        .dealing()
        .working_orders()
        .list_v1()
        .await
        .expect_err("should fail");

    match err {
        trading_ig::Error::Api { status, source } => {
            assert_eq!(status.as_u16(), 400);
            assert_eq!(source.error_code, "error.public-api.failure.kyc.required");
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}

// ── list_v2 ───────────────────────────────────────────────────────────────────

/// list_v2 returns v2-schema entries with camelCase fields deserialized.
#[tokio::test]
async fn list_v2_golden_path() {
    let (server, client) = start_server().await;
    mount_login_v3(&server).await;
    mount_get(
        &server,
        "workingorders",
        2,
        "dealing/working_orders/list_v2.json",
    )
    .await;

    client.session().login().await.expect("login");

    let orders = client
        .dealing()
        .working_orders()
        .list_v2()
        .await
        .expect("list_v2 succeeds");

    assert_eq!(orders.len(), 1);
    let o = &orders[0];
    assert_eq!(o.order_data.deal_id.as_str(), "DIAAAAA222222222");
    assert_eq!(o.order_data.direction, Direction::Sell);
    assert_eq!(o.order_data.order_type, OrderType::Limit);
    assert_eq!(o.order_data.order_size, 2.5);
    assert!(!o.order_data.guaranteed_stop);
    assert_eq!(o.market.epic.as_ref().map(|e| e.as_str()), Some("CS.D.EURUSD.TODAY.IP"));
}

/// list_v2 returns an empty vec when the envelope contains no orders.
#[tokio::test]
async fn list_v2_empty_returns_empty_vec() {
    let (server, client) = start_server().await;
    mount_login_v3(&server).await;
    mount_get(
        &server,
        "workingorders",
        2,
        "dealing/working_orders/list_v2_empty.json",
    )
    .await;

    client.session().login().await.expect("login");

    let orders = client
        .dealing()
        .working_orders()
        .list_v2()
        .await
        .expect("list_v2 empty succeeds");

    assert!(orders.is_empty());
}

/// `list()` is a direct alias for `list_v2()` and uses Version: 2.
#[tokio::test]
async fn list_alias_calls_list_v2() {
    let (server, client) = start_server().await;
    mount_login_v3(&server).await;
    mount_get(
        &server,
        "workingorders",
        2,
        "dealing/working_orders/list_v2.json",
    )
    .await;

    client.session().login().await.expect("login");

    let orders = client
        .dealing()
        .working_orders()
        .list()
        .await
        .expect("list (v2 alias) succeeds");

    assert_eq!(orders.len(), 1);
}

/// list_v2 API error is surfaced with the IG errorCode intact.
#[tokio::test]
async fn list_v2_api_error_surfaces_error_code() {
    let (server, client) = start_server().await;
    mount_login_v3(&server).await;
    mount_error(
        &server,
        "GET",
        "workingorders",
        401,
        "error.public-api.failure.oauth-token-invalid",
    )
    .await;

    client.session().login().await.expect("login");

    let err = client
        .dealing()
        .working_orders()
        .list_v2()
        .await
        .expect_err("should fail");

    match err {
        trading_ig::Error::Api { status, source } => {
            assert_eq!(status.as_u16(), 401);
            assert_eq!(
                source.error_code,
                "error.public-api.failure.oauth-token-invalid"
            );
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}

// ── create ────────────────────────────────────────────────────────────────────

/// create() builder posts to /workingorders/otc, fetches confirmation, returns it.
#[tokio::test]
async fn create_golden_path_returns_accepted_confirmation() {
    let (server, client) = start_server().await;
    mount_login_v3(&server).await;

    mount_post(
        &server,
        "workingorders/otc",
        2,
        "dealing/working_orders/create_response.json",
    )
    .await;

    mount_get(
        &server,
        "confirms/WO-REF-ABCDEF123456",
        1,
        "dealing/working_orders/confirm_accepted.json",
    )
    .await;

    client.session().login().await.expect("login");

    let conf = client
        .dealing()
        .working_orders()
        .create()
        .epic(Epic::new("CS.D.GBPUSD.TODAY.IP"))
        .direction(Direction::Buy)
        .size(1.0)
        .order_type(OrderType::Limit)
        .currency(Currency::new("GBP"))
        .expiry("DFB")
        .level(1.2345)
        .time_in_force(TimeInForce::GoodTillCancelled)
        .guaranteed_stop(false)
        .with_stop_distance(20.0)
        .with_limit_distance(10.0)
        .send()
        .await
        .expect("create succeeds");

    assert_eq!(conf.deal_status, DealStatus::Accepted);
    assert_eq!(conf.deal_reference.as_str(), "WO-REF-ABCDEF123456");
    assert_eq!(
        conf.deal_id.as_ref().map(|d| d.as_str()),
        Some("DIBBBBBB111111111")
    );
}

/// create() with a rejected confirmation still returns Ok — rejection is in the payload.
#[tokio::test]
async fn create_rejected_confirmation_is_returned() {
    let (server, client) = start_server().await;
    mount_login_v3(&server).await;

    Mock::given(method("POST"))
        .and(path("workingorders/otc"))
        .and(HasApiKey)
        .and(HasVersion(2))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(r#"{"dealReference":"WO-REF-ABCDEF789012"}"#),
        )
        .mount(&server)
        .await;

    mount_get(
        &server,
        "confirms/WO-REF-ABCDEF789012",
        1,
        "dealing/working_orders/confirm_rejected.json",
    )
    .await;

    client.session().login().await.expect("login");

    let conf = client
        .dealing()
        .working_orders()
        .create()
        .epic(Epic::new("CS.D.GBPUSD.TODAY.IP"))
        .direction(Direction::Buy)
        .size(1.0)
        .order_type(OrderType::Limit)
        .currency(Currency::new("GBP"))
        .expiry("DFB")
        .level(1.2345)
        .time_in_force(TimeInForce::GoodTillCancelled)
        .guaranteed_stop(false)
        .send()
        .await
        .expect("create call itself succeeds; rejection is within the confirmation payload");

    assert_eq!(conf.deal_status, DealStatus::Rejected);
    assert_eq!(conf.reason.as_deref(), Some("MARKET_NOT_BORROWABLE"));
    assert!(conf.deal_id.is_none());
}

// ── update ────────────────────────────────────────────────────────────────────

/// update() puts to /workingorders/otc/{dealId}, fetches confirmation.
#[tokio::test]
async fn update_golden_path() {
    let (server, client) = start_server().await;
    mount_login_v3(&server).await;

    mount_put(
        &server,
        "workingorders/otc/DIAAAAA222222222",
        2,
        "dealing/working_orders/update_response.json",
    )
    .await;

    mount_get(
        &server,
        "confirms/WO-REF-UPDATE-111",
        1,
        "dealing/working_orders/confirm_update.json",
    )
    .await;

    client.session().login().await.expect("login");

    let req = UpdateWorkingOrderRequest {
        good_till_date: None,
        level: 1.0900,
        limit_distance: Some(20.0),
        limit_level: None,
        stop_distance: Some(30.0),
        stop_level: None,
        guaranteed_stop: false,
        time_in_force: TimeInForce::GoodTillCancelled,
        order_type: OrderType::Limit,
    };

    let conf = client
        .dealing()
        .working_orders()
        .update(&DealId::new("DIAAAAA222222222"), req)
        .await
        .expect("update succeeds");

    assert_eq!(conf.deal_status, DealStatus::Accepted);
    assert_eq!(conf.deal_reference.as_str(), "WO-REF-UPDATE-111");
    assert_eq!(conf.level, Some(1.0900));
}

// ── delete ────────────────────────────────────────────────────────────────────

/// delete() sends DELETE to /workingorders/otc/{dealId}, fetches confirmation.
#[tokio::test]
async fn delete_golden_path() {
    let (server, client) = start_server().await;
    mount_login_v3(&server).await;

    mount_delete(
        &server,
        "workingorders/otc/DIAAAAA333333333",
        2,
        "dealing/working_orders/delete_response.json",
    )
    .await;

    mount_get(
        &server,
        "confirms/WO-REF-DELETE-999",
        1,
        "dealing/working_orders/confirm_delete.json",
    )
    .await;

    client.session().login().await.expect("login");

    let conf = client
        .dealing()
        .working_orders()
        .delete(&DealId::new("DIAAAAA333333333"))
        .await
        .expect("delete succeeds");

    assert_eq!(conf.deal_status, DealStatus::Accepted);
    assert_eq!(conf.deal_reference.as_str(), "WO-REF-DELETE-999");
    assert_eq!(
        conf.deal_id.as_ref().map(|d| d.as_str()),
        Some("DIAAAAA333333333")
    );
}
