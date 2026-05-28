//! Integration tests for proactive + reactive refresh and the
//! parallel-surfaces token model (REST + streaming + refresh).

use std::sync::Arc;
use std::time::Duration;

use trading_ig::{Credentials, Environment, IgClient};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

mod support;
use support::fixtures::load;
use support::matchers::HasBearer;
use support::mock_server::IgMockServer;

fn client_with_skew(mock: &IgMockServer, skew: Duration) -> IgClient {
    IgClient::builder()
        .environment(Environment::Custom(mock.base_url()))
        .api_key("test-api-key")
        .credentials(Credentials::password("test-user", "test-pass"))
        .token_refresh_skew(skew)
        .build()
        .expect("client builds")
}

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

#[tokio::test]
async fn reactive_refresh_recovers_from_401() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mount_refresh_endpoint(mock.server()).await;

    // First call → 401, then fallback mount serves 200.
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
    let accounts = client.accounts().list().await.expect("retry succeeds");
    assert!(!accounts.is_empty());

    assert_eq!(refresh_call_count(mock.server()).await, 1);

    let state = client.session_state().await;
    let refresh = state.tokens.refresh.expect("refresh state intact");
    assert_eq!(refresh.refresh_token, "rotated-refresh-token");
}

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

    let client = mock.client();
    client.session().login_v2().await.expect("v2 login");
    let err = client.accounts().list().await.expect_err("v2 401");
    match err {
        trading_ig::Error::Api { status, .. } => assert_eq!(status.as_u16(), 401),
        other => panic!("expected Error::Api 401, got {other:?}"),
    }
    assert_eq!(refresh_call_count(mock.server()).await, 0);
}

#[tokio::test]
async fn concurrent_refresh_is_serialised_to_one_post() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mount_refresh_endpoint(mock.server()).await;
    mock.mount_get("accounts", 1, "accounts/list_v1.json").await;

    let client = client_with_skew(&mock, Duration::from_mins(2));
    client.session().login().await.expect("login");

    let mut handles = Vec::new();
    for _ in 0..8 {
        let c = client.clone();
        handles.push(tokio::spawn(async move { c.accounts().list().await }));
    }
    for h in handles {
        h.await.expect("task").expect("call");
    }

    assert_eq!(refresh_call_count(mock.server()).await, 1);
}

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
    client.session().read(true).await.expect("read(true)");

    let state = client.session_state().await;
    let streaming = state.tokens.streaming.expect("streaming");
    assert_eq!(streaming.cst, "streaming-cst");
    assert_eq!(streaming.x_security_token, "streaming-xst");
    assert!(matches!(
        state.tokens.rest,
        Some(trading_ig::session::RestAuth::OAuth { .. })
    ));
    assert!(state.tokens.refresh.is_some());

    let _ = client
        .accounts()
        .list()
        .await
        .expect("REST still on Bearer");
}

#[allow(dead_code)]
const _: Option<Arc<()>> = None;
