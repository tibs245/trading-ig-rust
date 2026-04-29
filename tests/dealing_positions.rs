//! Integration tests for the `dealing/positions` domain.
//!
//! Each test spins up its own `IgMockServer` so tests are fully parallel-safe.

mod support;

use support::mock_server::IgMockServer;
use trading_ig::dealing::positions::{ClosePositionRequest, DealStatus, UpdatePositionRequest};
use trading_ig::models::common::{DealId, DealReference, Direction, Epic, OrderType};

// ---------------------------------------------------------------------------
// List v1
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_v1_golden_path() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_get("positions", 1, "dealing/positions/list_v1.json")
        .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let positions = client
        .dealing()
        .positions()
        .list_v1()
        .await
        .expect("list_v1 succeeds");

    assert_eq!(positions.len(), 1);
    let p = &positions[0];
    assert_eq!(p.deal_id.as_str(), "DIAAAABBBCCC01");
    assert_eq!(p.deal_reference.as_str(), "ref-v1-001");
    assert_eq!(p.direction, Direction::Buy);
    assert!((p.size - 1.0).abs() < f64::EPSILON);
    assert_eq!(
        p.market.epic.as_ref().map(trading_ig::models::Epic::as_str),
        Some("CS.D.GBPUSD.TODAY.IP")
    );
    assert_eq!(p.market.expiry.as_deref(), Some("DFB"));
}

#[tokio::test]
async fn list_v1_empty_returns_empty_vec() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_get("positions", 1, "dealing/positions/list_v1_empty.json")
        .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let positions = client
        .dealing()
        .positions()
        .list_v1()
        .await
        .expect("list_v1 empty succeeds");

    assert!(positions.is_empty());
}

#[tokio::test]
async fn list_v1_api_error_surfaces_error_code() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_error(
        "GET",
        "positions",
        403,
        "error.public-api.failure.kyc.required",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let err = client
        .dealing()
        .positions()
        .list_v1()
        .await
        .expect_err("should fail with API error");

    match err {
        trading_ig::Error::Api { status, source } => {
            assert_eq!(status.as_u16(), 403);
            assert_eq!(source.error_code, "error.public-api.failure.kyc.required");
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// List v2
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_v2_golden_path() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_get("positions", 2, "dealing/positions/list_v2.json")
        .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let positions = client
        .dealing()
        .positions()
        .list_v2()
        .await
        .expect("list_v2 succeeds");

    assert_eq!(positions.len(), 2);

    let p0 = &positions[0];
    assert_eq!(p0.deal_id.as_str(), "DIAAAABBBCCC02");
    assert_eq!(p0.direction, Direction::Buy);
    assert!(p0.created_date_utc.is_some(), "UTC date present");
    assert!(p0.stop_level.is_none());
    assert!(p0.limit_level.is_none());

    let p1 = &positions[1];
    assert_eq!(p1.deal_id.as_str(), "DIAAAABBBCCC03");
    assert_eq!(p1.direction, Direction::Sell);
    assert!(p1.controlled_risk);
    assert!(p1.stop_level.is_some());
    assert!(p1.created_date_utc.is_none(), "UTC date absent");
}

#[tokio::test]
async fn list_v2_empty_returns_empty_vec() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_get("positions", 2, "dealing/positions/list_v2_empty.json")
        .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let positions = client
        .dealing()
        .positions()
        .list_v2()
        .await
        .expect("list_v2 empty succeeds");

    assert!(positions.is_empty());
}

#[tokio::test]
async fn list_v2_api_error_surfaces_error_code() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_error(
        "GET",
        "positions",
        401,
        "error.public-api.failure.client-token-invalid",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let err = client
        .dealing()
        .positions()
        .list_v2()
        .await
        .expect_err("should fail");

    match err {
        trading_ig::Error::Api { status, source } => {
            assert_eq!(status.as_u16(), 401);
            assert_eq!(
                source.error_code,
                "error.public-api.failure.client-token-invalid"
            );
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}

#[tokio::test]
async fn list_alias_calls_list_v2() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    // `list()` uses Version: 2
    mock.mount_get("positions", 2, "dealing/positions/list_v2.json")
        .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let positions = client
        .dealing()
        .positions()
        .list()
        .await
        .expect("list succeeds");

    assert_eq!(positions.len(), 2);
}

// ---------------------------------------------------------------------------
// Get single position
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_position_golden_path() {
    let deal_id = "DIAAAABBBCCC02";
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_get(
        &format!("positions/{deal_id}"),
        2,
        "dealing/positions/get_v2.json",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let position = client
        .dealing()
        .positions()
        .get(&DealId::new(deal_id))
        .await
        .expect("get succeeds");

    assert_eq!(position.deal_id.as_str(), deal_id);
    assert_eq!(position.direction, Direction::Buy);
    assert!(position.limit_level.is_some());
    assert!(position.stop_level.is_some());
}

// ---------------------------------------------------------------------------
// Confirm (no retry needed in golden path)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn confirm_golden_path() {
    let deal_ref = DealReference::new("ref-open-001");
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_get(
        &format!("confirms/{}", deal_ref.as_str()),
        1,
        "dealing/confirms/accepted_v1.json",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let confirmation = client
        .dealing()
        .positions()
        .confirm(&deal_ref)
        .await
        .expect("confirm succeeds");

    assert_eq!(confirmation.deal_reference.as_str(), "ref-open-001");
    assert_eq!(confirmation.deal_status, DealStatus::Accepted);
    assert!(confirmation.reason.is_none());
}

#[tokio::test]
async fn confirm_rejected_deal_returns_rejected_status() {
    let deal_ref = DealReference::new("ref-open-002");
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_get(
        &format!("confirms/{}", deal_ref.as_str()),
        1,
        "dealing/confirms/rejected_v1.json",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let confirmation = client
        .dealing()
        .positions()
        .confirm(&deal_ref)
        .await
        .expect("confirm returns (even for rejected)");

    assert_eq!(confirmation.deal_status, DealStatus::Rejected);
    assert_eq!(confirmation.reason.as_deref(), Some("INSUFFICIENT_FUNDS"));
}

#[tokio::test]
async fn confirm_retries_on_transient_error_then_succeeds() {
    use wiremock::matchers::{header_exists, method, path};
    use wiremock::{Mock, ResponseTemplate};

    let deal_ref = DealReference::new("ref-open-001");
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;

    let confirm_path = format!("confirms/{}", deal_ref.as_str());
    let fixture_body = support::fixtures::load("dealing/confirms/accepted_v1.json");

    // First request → 503 (transient error)
    Mock::given(method("GET"))
        .and(path(&confirm_path))
        .and(header_exists("X-IG-API-KEY"))
        .respond_with(
            ResponseTemplate::new(503)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(r#"{"errorCode":"error.service.unavailable"}"#),
        )
        .up_to_n_times(1)
        .with_priority(1)
        .mount(mock.server())
        .await;

    // Subsequent requests → 200 success
    Mock::given(method("GET"))
        .and(path(&confirm_path))
        .and(header_exists("X-IG-API-KEY"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(fixture_body),
        )
        .with_priority(2)
        .mount(mock.server())
        .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let confirmation = client
        .dealing()
        .positions()
        .confirm(&deal_ref)
        .await
        .expect("confirm succeeds after retry");

    assert_eq!(confirmation.deal_status, DealStatus::Accepted);
}

// ---------------------------------------------------------------------------
// Open (type-state builder — golden path + rejected)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn open_position_golden_path_accepted() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_post("positions/otc", 2, "dealing/positions/open_v2.json")
        .await;
    mock.mount_get(
        "confirms/ref-open-001",
        1,
        "dealing/confirms/accepted_v1.json",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let confirmation = client
        .dealing()
        .positions()
        .open()
        .epic(Epic::new("CS.D.GBPUSD.TODAY.IP"))
        .direction(Direction::Buy)
        .size(1.0)
        .order_type(OrderType::Market)
        .currency("GBP")
        .expiry("DFB")
        .guaranteed_stop(false)
        .send()
        .await
        .expect("open succeeds");

    assert_eq!(confirmation.deal_reference.as_str(), "ref-open-001");
    assert_eq!(confirmation.deal_status, DealStatus::Accepted);
}

#[tokio::test]
async fn open_position_with_stop_and_limit() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_post("positions/otc", 2, "dealing/positions/open_v2.json")
        .await;
    mock.mount_get(
        "confirms/ref-open-001",
        1,
        "dealing/confirms/accepted_v1.json",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let confirmation = client
        .dealing()
        .positions()
        .open()
        .epic(Epic::new("CS.D.GBPUSD.TODAY.IP"))
        .direction(Direction::Buy)
        .size(1.0)
        .order_type(OrderType::Market)
        .currency("GBP")
        .expiry("DFB")
        .guaranteed_stop(false)
        .with_stop_distance(20.0)
        .with_limit_distance(10.0)
        .send()
        .await
        .expect("open with stop/limit succeeds");

    assert_eq!(confirmation.deal_status, DealStatus::Accepted);
}

#[tokio::test]
async fn open_position_confirmation_rejected() {
    // Mount the open_v2 POST with a different deal ref so we can mount
    // a rejected confirmation.
    use wiremock::matchers::{header_exists, method, path};
    use wiremock::{Mock, ResponseTemplate};

    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;

    // POST /positions/otc → dealReference: ref-open-002
    let open_body = r#"{"dealReference":"ref-open-002"}"#;
    Mock::given(method("POST"))
        .and(path("positions/otc"))
        .and(header_exists("X-IG-API-KEY"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(open_body),
        )
        .mount(mock.server())
        .await;

    mock.mount_get(
        "confirms/ref-open-002",
        1,
        "dealing/confirms/rejected_v1.json",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let confirmation = client
        .dealing()
        .positions()
        .open()
        .epic(Epic::new("CS.D.GBPUSD.TODAY.IP"))
        .direction(Direction::Buy)
        .size(1.0)
        .order_type(OrderType::Market)
        .currency("GBP")
        .expiry("DFB")
        .guaranteed_stop(false)
        .send()
        .await
        .expect("open returns even for rejected confirmation");

    assert_eq!(confirmation.deal_status, DealStatus::Rejected);
    assert_eq!(confirmation.reason.as_deref(), Some("INSUFFICIENT_FUNDS"));
}

// ---------------------------------------------------------------------------
// Update
// ---------------------------------------------------------------------------

#[tokio::test]
async fn update_position_golden_path() {
    let deal_id = "DIAAAABBBCCC02";
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_put(
        &format!("positions/otc/{deal_id}"),
        2,
        "dealing/positions/update_v2.json",
    )
    .await;
    mock.mount_get(
        "confirms/ref-update-001",
        1,
        "dealing/confirms/update_accepted_v1.json",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let req = UpdatePositionRequest {
        guaranteed_stop: false,
        stop_level: Some(1.18),
        limit_level: Some(1.30),
        trailing_stop: None,
        trailing_stop_distance: None,
        trailing_stop_increment: None,
    };

    let confirmation = client
        .dealing()
        .positions()
        .update(&DealId::new(deal_id), req)
        .await
        .expect("update succeeds");

    assert_eq!(confirmation.deal_status, DealStatus::Accepted);
    assert_eq!(confirmation.deal_reference.as_str(), "ref-update-001");
}

#[tokio::test]
async fn update_position_api_error() {
    let deal_id = "DIAAAABBBCCC99";
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_error(
        "PUT",
        &format!("positions/otc/{deal_id}"),
        400,
        "error.public-api.failure.deal.unknown.deal",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let req = UpdatePositionRequest::new(false);

    let err = client
        .dealing()
        .positions()
        .update(&DealId::new(deal_id), req)
        .await
        .expect_err("should fail");

    match err {
        trading_ig::Error::Api { status, source } => {
            assert_eq!(status.as_u16(), 400);
            assert_eq!(
                source.error_code,
                "error.public-api.failure.deal.unknown.deal"
            );
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Close
// ---------------------------------------------------------------------------

#[tokio::test]
async fn close_position_golden_path() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_delete_json("positions/otc", 1, "dealing/positions/close_v1.json")
        .await;
    mock.mount_get(
        "confirms/ref-close-001",
        1,
        "dealing/confirms/close_accepted_v1.json",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let req = ClosePositionRequest {
        deal_id: Some(DealId::new("DIAAAABBBCCC02")),
        direction: Direction::Sell,
        epic: Some(Epic::new("CS.D.GBPUSD.TODAY.IP")),
        expiry: Some("DFB".to_owned()),
        level: None,
        order_type: OrderType::Market,
        quote_id: None,
        size: 1.0,
        time_in_force: None,
    };

    let confirmation = client
        .dealing()
        .positions()
        .close(req)
        .await
        .expect("close succeeds");

    assert_eq!(confirmation.deal_status, DealStatus::Accepted);
    assert_eq!(confirmation.deal_reference.as_str(), "ref-close-001");
    assert!(confirmation.profit.is_some());
}

#[tokio::test]
async fn close_position_api_error() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_error(
        "DELETE",
        "positions/otc",
        400,
        "error.public-api.failure.position.unknown",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let req = ClosePositionRequest {
        deal_id: Some(DealId::new("DIAAAABBBCCC99")),
        direction: Direction::Sell,
        epic: None,
        expiry: None,
        level: None,
        order_type: OrderType::Market,
        quote_id: None,
        size: 1.0,
        time_in_force: None,
    };

    let err = client
        .dealing()
        .positions()
        .close(req)
        .await
        .expect_err("should fail");

    match err {
        trading_ig::Error::Api { status, source } => {
            assert_eq!(status.as_u16(), 400);
            assert_eq!(
                source.error_code,
                "error.public-api.failure.position.unknown"
            );
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}
