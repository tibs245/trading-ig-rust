//! Integration tests for the repeat dealing domain.

mod support;

use support::mock_server::IgMockServer;
use trading_ig::Error;
use trading_ig::models::Epic;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, ResponseTemplate};

use support::matchers::{HasApiKey, HasVersion};
use support::fixtures;

// ── golden path: all windows ──────────────────────────────────────────────────

#[tokio::test]
async fn window_returns_all_windows() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_get(
        "repeat-dealing-window",
        1,
        "repeat_dealing/window_v1.json",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let windows = client
        .repeat_dealing()
        .window()
        .await
        .expect("window");

    assert_eq!(windows.len(), 2);
    assert_eq!(windows[0].epic.as_str(), "CS.D.GBPUSD.TODAY.IP");
    assert_eq!(windows[1].epic.as_str(), "IX.D.DAX.IFD.IP");
}

// ── golden path: window for specific epic ────────────────────────────────────

#[tokio::test]
async fn window_for_epic_filters_results() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;

    // Mount a response for the query-param variant.
    let body = fixtures::load("repeat_dealing/window_v1.json");
    Mock::given(method("GET"))
        .and(path("repeat-dealing-window"))
        .and(query_param("epic", "CS.D.GBPUSD.TODAY.IP"))
        .and(HasApiKey)
        .and(HasVersion(1))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(body),
        )
        .mount(&mock.server)
        .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let epic = Epic::new("CS.D.GBPUSD.TODAY.IP");
    let windows = client
        .repeat_dealing()
        .window_for(&epic)
        .await
        .expect("window_for");

    assert_eq!(windows.len(), 2);
    assert_eq!(windows[0].epic.as_str(), "CS.D.GBPUSD.TODAY.IP");
}

// ── retry: 500 → 500 → 200 succeeds ──────────────────────────────────────────

#[tokio::test]
async fn window_retries_on_transient_error() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;

    // First two calls return 500.
    Mock::given(method("GET"))
        .and(path("repeat-dealing-window"))
        .and(HasApiKey)
        .and(HasVersion(1))
        .respond_with(
            ResponseTemplate::new(500)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(r#"{"errorCode":"error.internal-server-error"}"#),
        )
        .up_to_n_times(2)
        .with_priority(1)
        .mount(&mock.server)
        .await;

    // Third call (and beyond) returns 200.
    let body = fixtures::load("repeat_dealing/window_v1.json");
    Mock::given(method("GET"))
        .and(path("repeat-dealing-window"))
        .and(HasApiKey)
        .and(HasVersion(1))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(body),
        )
        .with_priority(2)
        .mount(&mock.server)
        .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let windows = client
        .repeat_dealing()
        .window()
        .await
        .expect("window should succeed after retries");

    assert_eq!(windows.len(), 2);
}

// ── API error propagated after exhausting retries ────────────────────────────

#[tokio::test]
async fn window_fails_after_max_retries() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;

    // All calls return 500.
    Mock::given(method("GET"))
        .and(path("repeat-dealing-window"))
        .and(HasApiKey)
        .and(HasVersion(1))
        .respond_with(
            ResponseTemplate::new(500)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(r#"{"errorCode":"error.internal-server-error"}"#),
        )
        .mount(&mock.server)
        .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let err = client
        .repeat_dealing()
        .window()
        .await
        .expect_err("should fail after 5 retries");

    match err {
        Error::Api { status, source } => {
            assert_eq!(status.as_u16(), 500);
            assert_eq!(source.error_code, "error.internal-server-error");
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}
