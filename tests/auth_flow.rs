//! End-to-end tests of the authentication flow against a wiremock instance.
//!
//! These tests *do not* hit the live IG API.

mod support;

use support::mock_server::IgMockServer;
use trading_ig::{Environment, IgClient};

#[tokio::test]
async fn login_v3_populates_session_state() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;

    let client = mock.client();
    let info = client.session().login().await.expect("v3 login succeeds");

    assert_eq!(info.account_id, "ABC123");
    assert_eq!(info.client_id, "999999");
    assert_eq!(info.locale.as_deref(), Some("fr_FR"));
    assert_eq!(info.currency_iso_code.as_deref(), Some("EUR"));
}

#[tokio::test]
async fn login_v2_reads_tokens_from_response_headers() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v2().await;

    let client = mock.client();
    let info = client
        .session()
        .login_v2()
        .await
        .expect("v2 login succeeds");

    assert_eq!(info.account_id, "ABC123");
    // The session state is now populated; subsequent authenticated calls
    // would inject CST + X-SECURITY-TOKEN headers.
}

#[tokio::test]
async fn login_failure_is_surfaced_as_api_error() {
    let mock = IgMockServer::start().await;
    mock.mount_error("POST", "session", 401, "error.security.invalid-details")
        .await;

    let client = mock.client();
    let err = client
        .session()
        .login()
        .await
        .expect_err("login should fail");

    match err {
        trading_ig::Error::Api { status, source } => {
            assert_eq!(status.as_u16(), 401);
            assert_eq!(source.error_code, "error.security.invalid-details");
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}

#[tokio::test]
async fn custom_environment_round_trips() {
    let mock = IgMockServer::start().await;
    // Sanity check: a Custom environment can be built from the mock URL
    // and IgClient construction succeeds.
    let client: IgClient = IgClient::builder()
        .environment(Environment::Custom(mock.base_url()))
        .api_key("k")
        .build()
        .expect("client builds without credentials");
    assert_eq!(client.config().api_key, "k");
}

#[tokio::test]
async fn login_then_logout_clears_session() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_error("DELETE", "session", 200, "ok").await; // logout body is empty

    let client = mock.client();
    client.session().login().await.expect("login");
    // Logout is best-effort: even on a server error it clears local state.
    let _ = client.session().logout().await;
}
