//! Integration tests for the accounts domain.
//!
//! Each test spins up its own `IgMockServer` so they are safe to run in
//! parallel.  No real IG API is contacted.

mod support;

use support::mock_server::IgMockServer;
use trading_ig::accounts::{AccountType, UpdatePreferences};

// ---------------------------------------------------------------------------
// list
// ---------------------------------------------------------------------------

/// Happy path: `GET /accounts` returns two accounts; typed fields are correct.
#[tokio::test]
async fn list_accounts_golden_path() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_get("accounts", 1, "accounts/list_v1.json").await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let accounts = client.accounts().list().await.expect("list accounts");

    assert_eq!(accounts.len(), 2);

    let first = &accounts[0];
    assert_eq!(first.account_id, "ABC123");
    assert_eq!(first.account_alias.as_deref(), Some("My CFD Account"));
    assert_eq!(first.account_type, AccountType::Cfd);
    assert_eq!(first.account_name, "Demo CFD");
    assert_eq!(first.can_transfer_to_ma, Some(true));
    assert_eq!(first.can_transfer_from_ma, Some(false));
    assert!(first.default_account);
    assert!(first.preferred);
    // Exact equality is fine here — these are round-trips of JSON number
    // literals with no floating-point arithmetic in between.
    #[allow(clippy::float_cmp)]
    {
        assert_eq!(first.balance.balance, Some(10_000.0));
        assert_eq!(first.balance.deposit, Some(500.0));
        assert_eq!(first.balance.profit_loss, Some(125.5));
        assert_eq!(first.balance.available_cash, Some(9_500.0));
    }

    let second = &accounts[1];
    assert_eq!(second.account_id, "DEF456");
    assert!(second.account_alias.is_none());
    assert_eq!(second.account_type, AccountType::Spreadbet);
    assert!(!second.default_account);
    assert!(!second.preferred);
}

/// Schema edge case: the second account in the fixture has a null alias.
/// Verifies that `Option<String>` is deserialised correctly.
#[tokio::test]
async fn list_accounts_null_alias_is_none() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_get("accounts", 1, "accounts/list_v1.json").await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let accounts = client.accounts().list().await.expect("list accounts");
    assert!(accounts[1].account_alias.is_none());
}

/// IG returns 401 with a known error code; the crate surfaces `Error::Api`.
#[tokio::test]
async fn list_accounts_api_error_is_surfaced() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_error("GET", "accounts", 401, "error.security.invalid-details")
        .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let err = client
        .accounts()
        .list()
        .await
        .expect_err("should fail with 401");

    match err {
        trading_ig::Error::Api { status, source } => {
            assert_eq!(status.as_u16(), 401);
            assert_eq!(source.error_code, "error.security.invalid-details");
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// preferences (read)
// ---------------------------------------------------------------------------

/// Happy path: `GET /accounts/preferences` returns `trailingStopsEnabled: true`.
#[tokio::test]
async fn get_preferences_golden_path() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_get("accounts/preferences", 1, "accounts/preferences_v1.json")
        .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let prefs = client.accounts().preferences().await.expect("get prefs");
    assert!(prefs.trailing_stops_enabled);
}

/// Schema edge case: `trailingStopsEnabled: false` deserialises correctly.
#[tokio::test]
async fn get_preferences_disabled() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_get(
        "accounts/preferences",
        1,
        "accounts/preferences_disabled_v1.json",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let prefs = client.accounts().preferences().await.expect("get prefs");
    assert!(!prefs.trailing_stops_enabled);
}

// ---------------------------------------------------------------------------
// update_preferences (write)
// ---------------------------------------------------------------------------

/// Happy path: `PUT /accounts/preferences` echoes back the updated prefs.
#[tokio::test]
async fn update_preferences_golden_path() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    // IG echoes back the new preferences after a successful update.
    mock.mount_put("accounts/preferences", 1, "accounts/preferences_v1.json")
        .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let prefs = client
        .accounts()
        .update_preferences(UpdatePreferences {
            trailing_stops_enabled: true,
        })
        .await
        .expect("update prefs");

    assert!(prefs.trailing_stops_enabled);
}

/// Disabling trailing stops: the PUT request serialises `"false"` as a string.
/// The mock returns the disabled fixture; the caller sees `false`.
#[tokio::test]
async fn update_preferences_disable_trailing_stops() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_put(
        "accounts/preferences",
        1,
        "accounts/preferences_disabled_v1.json",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let prefs = client
        .accounts()
        .update_preferences(UpdatePreferences {
            trailing_stops_enabled: false,
        })
        .await
        .expect("update prefs");

    assert!(!prefs.trailing_stops_enabled);
}
