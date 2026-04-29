//! Integration tests for the history domain.
//!
//! Covers activity v3 (golden + auto-pagination), activity v1 (period + date
//! range), transactions v2 (golden + API error), and transactions v1 period.

mod support;

use chrono::NaiveDate;
use support::fixtures;
use support::matchers::{HasApiKey, HasVersion};
use support::mock_server::IgMockServer;
use trading_ig::history::{
    ActivityRequest, ActivityStatus, ActivityType, TransactionType, TransactionsRequest,
};
use trading_ig::{Credentials, Environment, IgClient};
use wiremock::matchers::{method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ────────────────────────────────────────────────────────────────────────────
// Activity v3 — golden path (single page, no pagination)
// ────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn activity_v3_golden_single_page() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_get("history/activity", 3, "history/activity_v3_p2.json")
        .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let req = ActivityRequest::default();
    let activities = client
        .history()
        .activity_v3(req)
        .await
        .expect("activity_v3 succeeds");

    assert_eq!(activities.len(), 1);
    assert_eq!(activities[0].deal_id.as_str(), "DEAL002");
    assert_eq!(activities[0].epic.as_str(), "CS.D.EURUSD.TODAY.IP");
    assert!(matches!(
        activities[0].activity_type,
        ActivityType::WorkingOrder
    ));
    assert!(matches!(activities[0].status, ActivityStatus::Rejected));
}

// ────────────────────────────────────────────────────────────────────────────
// Activity v3 — auto-pagination across two pages
//
// Page 1 fixture contains metadata.paging.next pointing to
// "history/activity?pageSize=50&from=2024-01-01T00:00:00&_page=2".
// We mount page 2 so it matches the `_page=2` query parameter.
// The method must follow the next URL and combine both pages.
// ────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn activity_v3_auto_pagination_two_pages() {
    // Use a raw MockServer so we can mount mocks with query-param matchers.
    let server = MockServer::start().await;
    let base_url = url::Url::parse(&server.uri()).unwrap();

    let p1_body = fixtures::load("history/activity_v3_p1.json");
    let p2_body = fixtures::load("history/activity_v3_p2.json");

    // Login mock (v3 OAuth).
    let login_body = fixtures::load("session/login_v3.json");
    Mock::given(method("POST"))
        .and(path("session"))
        .and(HasApiKey)
        .and(HasVersion(3))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(login_body),
        )
        .mount(&server)
        .await;

    // Page 1: matched by pageSize=50 AND no _page param present.
    Mock::given(method("GET"))
        .and(path("history/activity"))
        .and(query_param("pageSize", "50"))
        .and(query_param_is_missing("_page"))
        .and(HasApiKey)
        .and(HasVersion(3))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(p1_body),
        )
        .mount(&server)
        .await;

    // Page 2: matched by _page=2 (the unique param in the next URL).
    Mock::given(method("GET"))
        .and(path("history/activity"))
        .and(query_param("_page", "2"))
        .and(HasApiKey)
        .and(HasVersion(3))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(p2_body),
        )
        .mount(&server)
        .await;

    let client = IgClient::builder()
        .environment(Environment::Custom(base_url))
        .api_key("test-api-key")
        .credentials(Credentials::password("test-user", "test-pass"))
        .build()
        .expect("client builds");

    client.session().login().await.expect("login");

    let req = ActivityRequest::default();
    let activities = client
        .history()
        .activity_v3(req)
        .await
        .expect("activity_v3 with pagination succeeds");

    // Both pages combined: 1 item from page 1 + 1 item from page 2 = 2.
    assert_eq!(
        activities.len(),
        2,
        "should have combined activities from both pages"
    );
    assert_eq!(activities[0].deal_id.as_str(), "DEAL001", "page 1 item first");
    assert_eq!(activities[1].deal_id.as_str(), "DEAL002", "page 2 item second");
}

// ────────────────────────────────────────────────────────────────────────────
// Activity v3 — API error
// ────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn activity_v3_api_error_is_surfaced() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_error(
        "GET",
        "history/activity",
        403,
        "error.public-api.failure.kyc.required",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let err = client
        .history()
        .activity_v3(ActivityRequest::default())
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

// ────────────────────────────────────────────────────────────────────────────
// Activity v3 — detailed sub-objects
// ────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn activity_v3_detailed_parses_sub_objects() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_get(
        "history/activity",
        3,
        "history/activity_v3_detailed.json",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let req = ActivityRequest {
        detailed: true,
        ..Default::default()
    };
    let activities = client
        .history()
        .activity_v3(req)
        .await
        .expect("detailed activity_v3 succeeds");

    assert_eq!(activities.len(), 1);
    let details = activities[0].details.as_ref().expect("details present");
    assert_eq!(details.actions.len(), 1);
    assert_eq!(details.actions[0].action_type, "POSITION_OPENED");
    assert_eq!(details.market_name.as_deref(), Some("GBP/USD"));
}

// ────────────────────────────────────────────────────────────────────────────
// Activity v1 — period
// ────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn activity_by_period_v1_golden() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_get(
        "history/activity/86400000",
        1,
        "history/activity_v1_period.json",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let acts = client
        .history()
        .activity_by_period_v1(86_400_000)
        .await
        .expect("activity_by_period_v1 succeeds");

    assert_eq!(acts.len(), 1);
    assert_eq!(acts[0].deal_id.as_str(), "DEAL001");
    assert_eq!(acts[0].epic.as_str(), "CS.D.GBPUSD.TODAY.IP");

    // Verify the v1 date was deserialised correctly from YYYY:MM:DD-HH:MM:SS.
    let expected = NaiveDate::from_ymd_opt(2024, 1, 15)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap();
    assert_eq!(acts[0].date, expected);
}

// ────────────────────────────────────────────────────────────────────────────
// Activity v1 — date range
// ────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn activity_by_date_range_v1_golden() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;

    // The path uses v1-formatted dates: YYYY:MM:DD-HH:MM:SS.
    let from_str = "2024:01:14-00:00:00";
    let to_str = "2024:01:15-23:59:59";
    let path_str = format!("history/activity/{from_str}/{to_str}");

    mock.mount_get(&path_str, 1, "history/activity_v1_date_range.json")
        .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let from = NaiveDate::from_ymd_opt(2024, 1, 14)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let to = NaiveDate::from_ymd_opt(2024, 1, 15)
        .unwrap()
        .and_hms_opt(23, 59, 59)
        .unwrap();

    let acts = client
        .history()
        .activity_by_date_range_v1(from, to)
        .await
        .expect("activity_by_date_range_v1 succeeds");

    assert_eq!(acts.len(), 2);
    assert_eq!(acts[0].deal_id.as_str(), "DEAL001");
    assert_eq!(acts[1].deal_id.as_str(), "DEAL002");
}

// ────────────────────────────────────────────────────────────────────────────
// Transactions v2 — golden path + numeric helpers
// ────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn transactions_v2_golden() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_get(
        "history/transactions",
        2,
        "history/transactions_v2.json",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let resp = client
        .history()
        .transactions_v2(TransactionsRequest::default())
        .await
        .expect("transactions_v2 succeeds");

    assert_eq!(resp.transactions.len(), 2);

    let t0 = &resp.transactions[0];
    assert_eq!(t0.reference, "DEAL001");
    assert_eq!(t0.instrument_name, "GBP/USD");
    assert_eq!(t0.profit_and_loss, "EUR123.45");
    assert!(!t0.cash_transaction);

    // Helper strips "EUR" prefix and parses the number.
    let pnl = t0.profit_and_loss_value().expect("parseable profit");
    assert!((pnl - 123.45_f64).abs() < 0.001, "pnl = {pnl}");

    // Negative profit/loss.
    let t1 = &resp.transactions[1];
    let pnl1 = t1
        .profit_and_loss_value()
        .expect("parseable negative profit");
    assert!((pnl1 - (-56.78_f64)).abs() < 0.001, "pnl1 = {pnl1}");

    // Paging metadata.
    assert_eq!(resp.metadata.page_data.total_pages, 1);
    assert_eq!(resp.metadata.page_data.page_number, 1);
}

// ────────────────────────────────────────────────────────────────────────────
// Transactions v2 — API error
// ────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn transactions_v2_api_error_is_surfaced() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_error(
        "GET",
        "history/transactions",
        400,
        "error.public-api.failure.validation.required",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let err = client
        .history()
        .transactions_v2(TransactionsRequest::default())
        .await
        .expect_err("should fail");

    match err {
        trading_ig::Error::Api { status, source } => {
            assert_eq!(status.as_u16(), 400);
            assert_eq!(
                source.error_code,
                "error.public-api.failure.validation.required"
            );
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Transactions v2 — type filter is serialised correctly
// ────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn transactions_v2_with_type_filter() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;

    // The request is sent as GET /history/transactions?type=DEPOSIT
    mock.mount_get(
        "history/transactions",
        2,
        "history/transactions_v2.json",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let req = TransactionsRequest {
        trans_type: Some(TransactionType::Deposit),
        ..Default::default()
    };
    // Succeeds if the mock matches (verifies our param serialisation doesn't crash).
    let _resp = client
        .history()
        .transactions_v2(req)
        .await
        .expect("filtered transactions_v2 succeeds");
}

// ────────────────────────────────────────────────────────────────────────────
// Transactions v1 — period
// ────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn transactions_by_period_v1_golden() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_get(
        "history/transactions/ALL_DEAL/86400000",
        1,
        "history/transactions_v1_period.json",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let txs = client
        .history()
        .transactions_by_period_v1(TransactionType::AllDeal, 86_400_000)
        .await
        .expect("transactions_by_period_v1 succeeds");

    assert_eq!(txs.len(), 1);
    assert_eq!(txs[0].reference, "DEAL003");
    assert_eq!(txs[0].profit_and_loss, "EUR99.00");

    let pnl = txs[0].profit_and_loss_value().expect("parseable");
    assert!((pnl - 99.0_f64).abs() < 0.001, "pnl = {pnl}");
}
