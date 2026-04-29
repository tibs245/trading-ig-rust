//! Integration tests for the historical prices domain.
//!
//! All tests use a local wiremock instance — no live IG API calls.
//!
//! Tests that need simple path-only matching reuse [`IgMockServer`]; tests
//! that need query-parameter matching build the wiremock [`MockServer`]
//! directly so they can mount custom matchers without modifying
//! `tests/support/mock_server.rs`.

mod support;

use support::fixtures;
use support::matchers::{HasApiKey, HasVersion};
use support::mock_server::IgMockServer;

use trading_ig::models::common::Epic;
use trading_ig::prices::models::{HistoricalPricesRequest, Resolution};
use trading_ig::{Credentials, Environment, Error, IgClient};

use chrono::NaiveDate;
use wiremock::matchers::{header_exists, method, path, path_regex, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── shared helpers ──────────────────────────────────────────────────────────

fn epic() -> Epic {
    Epic::new("CS.D.GBPUSD.TODAY.IP")
}

fn json_200(fixture: &str) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .insert_header("Content-Type", "application/json; charset=UTF-8")
        .set_body_string(fixtures::load(fixture))
}

/// Build an [`IgClient`] pointing at `server` and pre-logged-in via v3 OAuth.
async fn authenticated_client(server: &MockServer) -> IgClient {
    // Mount the login endpoint.
    Mock::given(method("POST"))
        .and(path("/session"))
        .and(HasApiKey)
        .and(HasVersion(3))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(fixtures::load("session/login_v3.json")),
        )
        .mount(server)
        .await;

    let base = url::Url::parse(&server.uri()).expect("valid URL");
    let client = IgClient::builder()
        .environment(Environment::Custom(base))
        .api_key("test-api-key")
        .credentials(Credentials::password("test-user", "test-pass"))
        .build()
        .expect("client builds");
    client.session().login().await.expect("login");
    client
}

fn dt(h: u32) -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(2014, 12, 15)
        .unwrap()
        .and_hms_opt(h, 0, 0)
        .unwrap()
}

// ── v3 basic ─────────────────────────────────────────────────────────────────

/// Golden-path: single page, basic request, typed result is correct.
#[tokio::test]
async fn history_v3_golden_path() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_get(
        &format!("prices/{}", epic()),
        3,
        "prices/history_v3_basic.json",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let result = client
        .prices()
        .history_v3(&epic(), HistoricalPricesRequest::default())
        .await
        .expect("history_v3 succeeds");

    assert_eq!(result.prices.len(), 2, "two price points");
    assert_eq!(result.instrument_type, "CURRENCIES");
    assert_eq!(result.metadata.page_data.total_pages, 1);
    assert_eq!(result.metadata.allowance.remaining_allowance, 9998);
    assert_eq!(result.prices[0].snapshot_time, "2014/12/15 09:00:00");
    // snapshotTimeUTC parses into a NaiveDateTime.
    assert_eq!(result.prices[0].snapshot_time_utc, dt(9));
    // Optional fields.
    assert!(result.prices[0].open_price.last_traded.is_none());
    assert_eq!(result.prices[0].last_traded_volume, Some(1234));
}

/// Schema edge case: resolution query param is sent when specified.
#[tokio::test]
async fn history_v3_with_resolution_sends_query_param() {
    let server = MockServer::start().await;
    let client = authenticated_client(&server).await;

    Mock::given(method("GET"))
        .and(path(format!("/prices/{}", epic())))
        .and(HasApiKey)
        .and(HasVersion(3))
        .and(query_param("resolution", "HOUR"))
        .respond_with(json_200("prices/history_v3_basic.json"))
        .mount(&server)
        .await;

    let req = HistoricalPricesRequest {
        resolution: Some(Resolution::Hour),
        ..Default::default()
    };
    let result = client.prices().history_v3(&epic(), req).await.expect("ok");
    assert_eq!(result.prices.len(), 2);
}

// ── v3 auto-pagination ───────────────────────────────────────────────────────

/// Pagination: `history_v3_all` fetches both pages and returns combined Vec.
#[tokio::test]
async fn history_v3_all_combines_pages() {
    let server = MockServer::start().await;
    let client = authenticated_client(&server).await;

    let epic_path = format!("/prices/{}", epic());

    // Page 1 — pageNumber=1.
    Mock::given(method("GET"))
        .and(path(epic_path.clone()))
        .and(HasApiKey)
        .and(HasVersion(3))
        .and(query_param("pageNumber", "1"))
        .respond_with(json_200("prices/history_v3_paged_p1.json"))
        .mount(&server)
        .await;

    // Page 2 — pageNumber=2.
    Mock::given(method("GET"))
        .and(path(epic_path.clone()))
        .and(HasApiKey)
        .and(HasVersion(3))
        .and(query_param("pageNumber", "2"))
        .respond_with(json_200("prices/history_v3_paged_p2.json"))
        .mount(&server)
        .await;

    let result = client
        .prices()
        .history_v3_all(&epic(), HistoricalPricesRequest::default())
        .await
        .expect("history_v3_all succeeds");

    // 1 price from p1 + 1 from p2 = 2 combined.
    assert_eq!(result.prices.len(), 2, "_all must combine both pages");
    assert_eq!(result.prices[0].snapshot_time, "2014/12/15 09:00:00");
    assert_eq!(result.prices[1].snapshot_time, "2014/12/15 09:01:00");
    // metadata reflects the last page.
    assert_eq!(result.metadata.page_data.page_number, 2);
    assert_eq!(result.metadata.allowance.remaining_allowance, 9998);
    // instrument_type taken from page 1.
    assert_eq!(result.instrument_type, "CURRENCIES");
}

// ── v2 num-points ─────────────────────────────────────────────────────────────

/// Golden-path: fixed N points.
#[tokio::test]
async fn history_by_num_points_v2_golden_path() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_get(
        &format!("prices/{}/HOUR/100", epic()),
        2,
        "prices/history_num_points_v2.json",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let result = client
        .prices()
        .history_by_num_points_v2(&epic(), Resolution::Hour, 100)
        .await
        .expect("history_by_num_points_v2 succeeds");

    assert_eq!(result.prices.len(), 1);
    assert_eq!(result.metadata.allowance.remaining_allowance, 9999);
}

/// Schema edge case: zero-point response is a valid (empty) result.
#[tokio::test]
async fn history_by_num_points_v2_zero_points() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_get(
        &format!("prices/{}/MINUTE/0", epic()),
        2,
        "prices/history_num_points_v2_empty.json",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let result = client
        .prices()
        .history_by_num_points_v2(&epic(), Resolution::Minute, 0)
        .await
        .expect("zero-point response is valid");

    assert!(result.prices.is_empty(), "empty prices vec for zero points");
    assert_eq!(result.metadata.page_data.total_pages, 0);
}

// ── v2 date range ─────────────────────────────────────────────────────────────

/// Golden-path: date range v2 (dates embedded in path).
#[tokio::test]
async fn history_by_date_range_v2_golden_path() {
    let server = MockServer::start().await;
    let client = authenticated_client(&server).await;

    // v2 date format: YYYY/MM/DD HH:MM:SS.
    // Slashes in the date are extra path segments; the space between date and
    // time is percent-encoded to %20 in the URL path as reported by url.path().
    Mock::given(method("GET"))
        .and(path_regex(
            r"^/prices/CS\.D\.GBPUSD\.TODAY\.IP/HOUR/2014/12/15%2009:00:00/2014/12/15%2011:00:00$",
        ))
        .and(HasApiKey)
        .and(HasVersion(2))
        .respond_with(json_200("prices/history_date_range_v2.json"))
        .mount(&server)
        .await;

    let result = client
        .prices()
        .history_by_date_range_v2(&epic(), Resolution::Hour, dt(9), dt(11))
        .await
        .expect("history_by_date_range_v2 succeeds");

    assert_eq!(result.prices.len(), 1);
    assert_eq!(result.metadata.allowance.remaining_allowance, 9997);
}

// ── v1 date range ─────────────────────────────────────────────────────────────

/// Golden-path: date range v1 (dates in query params, lowercase names).
#[tokio::test]
async fn history_by_date_range_v1_golden_path() {
    let server = MockServer::start().await;
    let client = authenticated_client(&server).await;

    // v1: startdate / enddate as query params, format YYYY:MM:DD-HH:MM:SS.
    Mock::given(method("GET"))
        .and(path(format!("/prices/{}/HOUR", epic())))
        .and(HasApiKey)
        .and(HasVersion(1))
        .and(query_param("startdate", "2014:12:15-09:00:00"))
        .and(query_param("enddate", "2014:12:15-11:00:00"))
        .respond_with(json_200("prices/history_date_range_v1.json"))
        .mount(&server)
        .await;

    let result = client
        .prices()
        .history_by_date_range_v1(&epic(), Resolution::Hour, dt(9), dt(11))
        .await
        .expect("history_by_date_range_v1 succeeds");

    assert_eq!(result.prices.len(), 1);
    // Optional lastTradedVolume is null in the fixture.
    assert!(result.prices[0].last_traded_volume.is_none());
    assert_eq!(result.metadata.allowance.remaining_allowance, 9996);
}

// ── API error ─────────────────────────────────────────────────────────────────

/// 4xx with IG errorCode is surfaced as `Error::Api`.
#[tokio::test]
async fn history_returns_api_error_on_allowance_exceeded() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_error(
        "GET",
        &format!("prices/{}", epic()),
        403,
        "error.public-api.exceeded-account-historical-data-allowance",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let err = client
        .prices()
        .history_v3(&epic(), HistoricalPricesRequest::default())
        .await
        .expect_err("should fail with allowance error");

    match err {
        Error::Api { status, source } => {
            assert_eq!(status.as_u16(), 403);
            assert_eq!(
                source.error_code,
                "error.public-api.exceeded-account-historical-data-allowance"
            );
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}

/// `Error::Api` is also returned from v2 num-points when IG rate-limits.
#[tokio::test]
async fn history_num_points_v2_returns_api_error() {
    let server = MockServer::start().await;
    let client = authenticated_client(&server).await;

    Mock::given(method("GET"))
        .and(path_regex(r"^/prices/.+/MINUTE/\d+$"))
        .and(header_exists("X-IG-API-KEY"))
        .respond_with(
            ResponseTemplate::new(403)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(
                    r#"{"errorCode":"error.public-api.exceeded-account-historical-data-allowance"}"#,
                ),
        )
        .mount(&server)
        .await;

    let err = client
        .prices()
        .history_by_num_points_v2(&epic(), Resolution::Minute, 100)
        .await
        .expect_err("should fail");

    assert!(
        matches!(err, Error::Api { status, .. } if status.as_u16() == 403),
        "expected 403 Api error, got {err:?}"
    );
}
