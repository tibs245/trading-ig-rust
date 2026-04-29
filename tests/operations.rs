//! Integration tests for the operations (application management) domain.

mod support;

use support::fixtures;
use support::matchers::{HasApiKey, HasVersion};
use support::mock_server::IgMockServer;
use trading_ig::Error;
use trading_ig::operations::{ApplicationStatus, UpdateApplicationRequest};
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

// ── golden path: list applications ───────────────────────────────────────────

#[tokio::test]
async fn applications_returns_list() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_get(
        "operations/application",
        1,
        "operations/applications_v1.json",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let apps = client
        .operations()
        .applications()
        .await
        .expect("applications");

    assert_eq!(apps.len(), 2);
    assert_eq!(apps[0].name, "My Trading App");
    assert_eq!(apps[0].status, ApplicationStatus::Enabled);
    assert!(apps[0].allow_equities);
    assert_eq!(apps[0].allowance_account_overall, 60);

    assert_eq!(apps[1].name, "Read-Only Monitor");
    assert_eq!(apps[1].status, ApplicationStatus::Disabled);
    assert!(!apps[1].allow_equities);
}

// ── golden path: update application ──────────────────────────────────────────

#[tokio::test]
async fn update_application_returns_updated_record() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;

    let body = fixtures::load("operations/applications_v1.json");
    // The update endpoint returns a single Application, not an array.
    // Re-use the first entry from our fixture by parsing and re-serialising.
    let apps: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap();
    let single = serde_json::to_string(&apps[0]).unwrap();

    Mock::given(method("PUT"))
        .and(path("operations/application"))
        .and(HasApiKey)
        .and(HasVersion(1))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json; charset=UTF-8")
                .set_body_string(single),
        )
        .mount(&mock.server)
        .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let req = UpdateApplicationRequest {
        api_key: "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4".into(),
        status: ApplicationStatus::Enabled,
        allowance_account_overall: 60,
        allowance_account_trading: 10,
    };

    let updated = client
        .operations()
        .update_application(req)
        .await
        .expect("update_application");

    assert_eq!(updated.name, "My Trading App");
    assert_eq!(updated.status, ApplicationStatus::Enabled);
}

// ── golden path: disable current key ─────────────────────────────────────────

#[tokio::test]
async fn disable_current_key_returns_disabled_record() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;

    let body = fixtures::load("operations/disabled_v1.json");
    Mock::given(method("PUT"))
        .and(path("operations/application/disable"))
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

    let disabled = client
        .operations()
        .disable_current_key()
        .await
        .expect("disable_current_key");

    assert_eq!(disabled.name, "My Trading App");
    assert_eq!(disabled.status, ApplicationStatus::Disabled);
}

// ── API error: non-administrator account ─────────────────────────────────────

#[tokio::test]
async fn applications_fails_for_non_admin() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_error(
        "GET",
        "operations/application",
        403,
        "error.public-api.failure.not.an.administrator",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let err = client
        .operations()
        .applications()
        .await
        .expect_err("should fail for non-admin");

    match err {
        Error::Api { status, source } => {
            assert_eq!(status.as_u16(), 403);
            assert_eq!(
                source.error_code,
                "error.public-api.failure.not.an.administrator"
            );
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}
