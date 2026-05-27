//! Integration tests for the auto-refresh + 401 retry behaviour
//! introduced with the parallel-surfaces token refactor.
//!
//! These tests exercise the v3 (OAuth) refresh path end-to-end against
//! `wiremock`-mocked IG endpoints. They guard against regressions on:
//!
//! - **Proactive refresh** : when the access token's TTL drops below
//!   the configured skew, the next authenticated request triggers a
//!   `POST /session/refresh-token` *before* hitting the actual endpoint.
//! - **Reactive refresh** : on a 401 from any authenticated endpoint,
//!   the crate attempts a refresh and retries once.
//! - **Reactive refresh failure** : if the refresh itself fails, the
//!   *original* 401 surfaces — callers want to see the meaningful
//!   error, not the incidental refresh failure.
//! - **v2 sessions don't refresh** : a v2 (CST) session 401 propagates
//!   immediately ; the crate has no refresh token to spend.
//! - **read(true) preserves OAuth** : after populating the streaming
//!   surface via `read(true)`, REST calls still go out with the OAuth
//!   Bearer header and the refresh state is intact.

use std::sync::Arc;
use std::time::Duration;

use trading_ig::{Credentials, Environment, IgClient};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

mod support;
use support::fixtures::load;
use support::matchers::HasBearer;
use support::mock_server::IgMockServer;

/// Build an `IgClient` pointing at the mock with a custom refresh skew.
/// Used by the proactive-refresh test : passing a skew larger than the
/// fixture's `expires_in: 60` guarantees `needs_refresh` fires on the
/// very next request.
fn client_with_skew(mock: &IgMockServer, skew: Duration) -> IgClient {
    IgClient::builder()
        .environment(Environment::Custom(mock.base_url()))
        .api_key("test-api-key")
        .credentials(Credentials::password("test-user", "test-pass"))
        .token_refresh_skew(skew)
        .build()
        .expect("client builds")
}

/// Mount a `/session/refresh-token` endpoint that returns a fresh OAuth
/// payload (new `access_token` + new `refresh_token` + 60s TTL). Tests
/// can then call [`refresh_call_count`] to assert how many times the
/// refresh path was exercised.
async fn mount_refresh_endpoint(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("session/refresh-token"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(
                    r#"{
                        "access_token": "rotated-access-token",
                        "refresh_token": "rotated-refresh-token",
                        "token_type": "Bearer",
                        "expires_in": "60"
                    }"#,
                ),
        )
        .mount(server)
        .await;
}

/// Count POST hits on `/session/refresh-token` by scanning recorded
/// requests. wiremock keeps every Request in memory by default.
async fn refresh_call_count(server: &MockServer) -> usize {
    server
        .received_requests()
        .await
        .unwrap_or_default()
        .iter()
        .filter(|r: &&Request| {
            r.method.as_str() == "POST" && r.url.path() == "/session/refresh-token"
        })
        .count()
}

// ---------------------------------------------------------------------------
// 1. Proactive refresh
// ---------------------------------------------------------------------------

/// With `token_refresh_skew = 120s` and the fixture's `expires_in = 60s`,
/// the access token is *already* inside the refresh window at login time.
/// The very next authenticated request must therefore trigger a refresh
/// before the actual endpoint is hit.
#[tokio::test]
async fn proactive_refresh_fires_when_token_within_skew() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mount_refresh_endpoint(mock.server()).await;
    mock.mount_get("accounts", 1, "accounts/list_v1.json").await;

    let client = client_with_skew(&mock, Duration::from_mins(2));
    client.session().login().await.expect("login");

    assert_eq!(
        refresh_call_count(mock.server()).await,
        0,
        "no refresh before first authenticated call"
    );

    let _ = client.accounts().list().await.expect("list accounts");

    assert_eq!(
        refresh_call_count(mock.server()).await,
        1,
        "exactly one proactive refresh fired before the request"
    );

    // The session now carries the rotated tokens.
    let state = client.session_state().await;
    let refresh = state.tokens.refresh.expect("refresh state intact");
    assert_eq!(refresh.refresh_token, "rotated-refresh-token");
}

/// With a tiny skew, the fresh 60s token does *not* trigger refresh on
/// the next request — proactive path stays dormant.
#[tokio::test]
async fn proactive_refresh_does_not_fire_when_outside_skew() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mount_refresh_endpoint(mock.server()).await;
    mock.mount_get("accounts", 1, "accounts/list_v1.json").await;

    let client = client_with_skew(&mock, Duration::from_secs(5));
    client.session().login().await.expect("login");
    let _ = client.accounts().list().await.expect("list accounts");

    assert_eq!(
        refresh_call_count(mock.server()).await,
        0,
        "no proactive refresh when access token has plenty of TTL"
    );
}

// ---------------------------------------------------------------------------
// 2. Reactive refresh on 401
// ---------------------------------------------------------------------------

/// First /accounts call returns 401 → refresh succeeds → retry returns
/// 200. The caller sees a successful response and is unaware of the
/// hiccup.
#[tokio::test]
async fn reactive_refresh_recovers_from_401() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mount_refresh_endpoint(mock.server()).await;

    // First call → 401 (client-token-invalid). Only matches once.
    Mock::given(method("GET"))
        .and(path("accounts"))
        .respond_with(
            ResponseTemplate::new(401)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(r#"{"errorCode":"error.security.client-token-invalid"}"#),
        )
        .up_to_n_times(1)
        .mount(mock.server())
        .await;
    // Subsequent calls → 200 (fallback mount, lower priority by
    // insertion order). The retry after refresh lands here.
    Mock::given(method("GET"))
        .and(path("accounts"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(load("accounts/list_v1.json")),
        )
        .mount(mock.server())
        .await;

    let client = mock.client();
    client.session().login().await.expect("login");
    let accounts = client
        .accounts()
        .list()
        .await
        .expect("retry after refresh succeeds");
    assert!(!accounts.is_empty(), "fixture has accounts");

    assert_eq!(
        refresh_call_count(mock.server()).await,
        1,
        "exactly one reactive refresh on 401"
    );

    // After refresh + retry, the session carries the rotated tokens.
    let state = client.session_state().await;
    let refresh = state.tokens.refresh.expect("refresh state intact");
    assert_eq!(refresh.refresh_token, "rotated-refresh-token");
}

/// First /accounts call returns 401, refresh ALSO fails (no
/// `/session/refresh-token` endpoint mounted → 404). The crate must
/// surface the *original* 401, not the refresh 404 — the caller wants
/// to see the meaningful error.
#[tokio::test]
async fn reactive_refresh_failure_surfaces_original_401() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    // No refresh endpoint mounted ⇒ POST returns 404.
    mock.mount_error(
        "GET",
        "accounts",
        401,
        "error.security.client-token-invalid",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");
    let err = client
        .accounts()
        .list()
        .await
        .expect_err("should fail with original 401");

    match err {
        trading_ig::Error::Api { status, source } => {
            assert_eq!(status.as_u16(), 401);
            assert_eq!(source.error_code, "error.security.client-token-invalid");
        }
        other => panic!("expected Error::Api 401, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 3. v2 sessions don't try to refresh
// ---------------------------------------------------------------------------

/// A v2-authenticated session has no refresh token. On 401, the crate
/// must NOT attempt a refresh (it would call the unmounted
/// /session/refresh-token endpoint). The 401 propagates immediately.
#[tokio::test]
async fn v2_session_401_skips_refresh() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v2().await;
    mock.mount_error(
        "GET",
        "accounts",
        401,
        "error.security.client-token-invalid",
    )
    .await;

    // Mount a refresh endpoint that would PANIC if hit. wiremock has no
    // direct "fail on call" but we use an unmatched filter (must not be
    // called) plus the post-hoc refresh_call_count assertion.
    let client = mock.client();
    client.session().login_v2().await.expect("v2 login");
    let err = client
        .accounts()
        .list()
        .await
        .expect_err("v2 401 propagates");

    match err {
        trading_ig::Error::Api { status, .. } => assert_eq!(status.as_u16(), 401),
        other => panic!("expected Error::Api 401, got {other:?}"),
    }

    assert_eq!(
        refresh_call_count(mock.server()).await,
        0,
        "v2 session must not call /session/refresh-token"
    );
}

// ---------------------------------------------------------------------------
// 4. read(true) preserves OAuth + populates streaming
// ---------------------------------------------------------------------------

/// After `login_v3` + `read(true)` the session must carry three things
/// at once : OAuth bearer for REST, refresh state for proactive
/// renewal, and CST/XST for Lightstreamer. This is the core regression
/// fix versus the prior tagged-enum design that forced callers to
/// choose between OAuth-with-refresh and CST-for-streaming.
#[tokio::test]
async fn read_true_after_v3_login_populates_three_surfaces() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;

    Mock::given(method("GET"))
        .and(path("session"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .insert_header("CST", "streaming-cst")
                .insert_header("X-SECURITY-TOKEN", "streaming-xst")
                .set_body_string(load("session/read_v1.json")),
        )
        .mount(mock.server())
        .await;

    // Subsequent call must still use Bearer (REST surface unchanged).
    Mock::given(method("GET"))
        .and(path("accounts"))
        .and(HasBearer)
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(load("accounts/list_v1.json")),
        )
        .mount(mock.server())
        .await;

    let client = mock.client();
    client.session().login().await.expect("v3 login");
    client
        .session()
        .read(true)
        .await
        .expect("read(true) populates streaming");

    // Streaming CST/XST present.
    let state = client.session_state().await;
    let streaming = state.tokens.streaming.expect("streaming auth populated");
    assert_eq!(streaming.cst, "streaming-cst");
    assert_eq!(streaming.x_security_token, "streaming-xst");

    // REST surface still on OAuth (Bearer).
    assert!(
        matches!(
            state.tokens.rest,
            Some(trading_ig::session::RestAuth::OAuth { .. })
        ),
        "OAuth bearer must survive read(true)"
    );

    // Refresh state intact.
    assert!(state.tokens.refresh.is_some(), "refresh state preserved");

    // And a REST call goes out with Bearer (not CST/XST).
    let _ = client
        .accounts()
        .list()
        .await
        .expect("REST still on Bearer");
}

// ---------------------------------------------------------------------------
// Silence unused-import warning when these helpers are only used in some
// tests on certain feature combinations.
// ---------------------------------------------------------------------------
#[allow(dead_code)]
const _: Option<Arc<()>> = None;
