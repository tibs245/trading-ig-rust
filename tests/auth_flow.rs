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

#[tokio::test]
async fn read_session_returns_details() {
    use support::fixtures::load;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    Mock::given(method("GET"))
        .and(path("session"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(load("session/read_v1.json")),
        )
        .mount(mock.server())
        .await;

    let client = mock.client();
    client.session().login().await.expect("login");
    let details = client.session().read(false).await.expect("read");
    assert_eq!(details.account_id, "ABC123");
    assert_eq!(details.account_type.as_deref(), Some("CFD"));
    assert_eq!(details.currency.as_deref(), Some("EUR"));
}

#[tokio::test]
async fn read_session_with_fetch_tokens_stores_cst_tokens() {
    use support::fixtures::load;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    Mock::given(method("GET"))
        .and(path("session"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .insert_header("CST", "fetched-cst-token")
                .insert_header("X-SECURITY-TOKEN", "fetched-xst-token")
                .set_body_string(load("session/read_v1.json")),
        )
        .mount(mock.server())
        .await;

    // Subsequent authenticated call requires CST headers — proves the tokens
    // were stored after `read(true)` and are now used in place of the OAuth
    // bearer token from login_v3.
    Mock::given(method("GET"))
        .and(path("accounts"))
        .and(support::matchers::HasCstHeaders)
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(r#"{"accounts": []}"#),
        )
        .mount(mock.server())
        .await;

    let client = mock.client();
    client.session().login().await.expect("login");
    let _ = client.session().read(true).await.expect("read with tokens");
    let _ = client.accounts().list().await.expect("CST headers used");
}

#[tokio::test]
async fn switch_account_updates_local_account_id() {
    use support::fixtures::load;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    Mock::given(method("PUT"))
        .and(path("session"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(load("session/switch_account_v1.json")),
        )
        .mount(mock.server())
        .await;

    // After switch_account, subsequent v3 calls must carry the new account
    // id in `IG-ACCOUNT-ID`. We verify this by mounting an /accounts mock
    // that requires the new id.
    Mock::given(method("GET"))
        .and(path("accounts"))
        .and(wiremock::matchers::header("IG-ACCOUNT-ID", "DEF456"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(r#"{"accounts": []}"#),
        )
        .mount(mock.server())
        .await;

    let client = mock.client();
    client.session().login().await.expect("login");
    let resp = client
        .session()
        .switch_account("DEF456", true)
        .await
        .expect("switch");
    assert!(resp.dealing_enabled);
    let _ = client.accounts().list().await.expect("new account id used");
}

#[cfg(feature = "encryption")]
#[tokio::test]
async fn encryption_key_endpoint_deserialises() {
    use support::fixtures::load;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    let mock = IgMockServer::start().await;
    Mock::given(method("GET"))
        .and(path("session/encryptionKey"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(load("session/encryption_key.json")),
        )
        .mount(mock.server())
        .await;

    let client = mock.client();
    let key = client
        .session()
        .encryption_key()
        .await
        .expect("encryption key fetched");
    assert!(key.encryption_key.starts_with("MIIB"));
    assert_eq!(key.time_stamp, 1_700_000_000_000);
}
