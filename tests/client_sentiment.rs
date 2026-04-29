//! Integration tests for the client sentiment domain.

mod support;

use support::mock_server::IgMockServer;
use trading_ig::Error;

// ── golden path: single market ───────────────────────────────────────────────

#[tokio::test]
async fn get_single_returns_sentiment() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_get(
        "clientsentiment/CC.D.LCO.UNC.IP",
        1,
        "client_sentiment/get_v1.json",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let s = client
        .client_sentiment()
        .get("CC.D.LCO.UNC.IP")
        .await
        .expect("get sentiment");

    assert_eq!(s.market_id, "CC.D.LCO.UNC.IP");
    assert!((s.long_position_percentage - 68.0).abs() < f64::EPSILON);
    assert!((s.short_position_percentage - 32.0).abs() < f64::EPSILON);
    // sanity: long + short ≈ 100
    assert!((s.long_position_percentage + s.short_position_percentage - 100.0).abs() < 0.01);
}

// ── golden path: bulk markets ────────────────────────────────────────────────

#[tokio::test]
async fn get_many_returns_all_sentiments() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_get("clientsentiment", 1, "client_sentiment/get_many_v1.json")
        .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let many = client
        .client_sentiment()
        .get_many(&["CC.D.LCO.UNC.IP", "IX.D.DAX.IFD.IP", "CS.D.GBPUSD.TODAY.IP"])
        .await
        .expect("get many");

    assert_eq!(many.len(), 3);
    assert_eq!(many[0].market_id, "CC.D.LCO.UNC.IP");
    assert_eq!(many[1].market_id, "IX.D.DAX.IFD.IP");
    assert_eq!(many[2].market_id, "CS.D.GBPUSD.TODAY.IP");

    // sanity: all long + short ≈ 100
    for s in &many {
        assert!(
            (s.long_position_percentage + s.short_position_percentage - 100.0).abs() < 0.01,
            "market {} percentages don't sum to 100",
            s.market_id
        );
    }
}

// ── golden path: related markets ─────────────────────────────────────────────

#[tokio::test]
async fn related_returns_related_sentiments() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_get(
        "clientsentiment/related/CC.D.LCO.UNC.IP",
        1,
        "client_sentiment/related_v1.json",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let related = client
        .client_sentiment()
        .related("CC.D.LCO.UNC.IP")
        .await
        .expect("related");

    assert_eq!(related.len(), 2);
    assert_eq!(related[0].market_id, "IX.D.DAX.IFD.IP");
    assert_eq!(related[1].market_id, "IX.D.FTSE.IFD.IP");
}

// ── API error: unknown market id ─────────────────────────────────────────────

#[tokio::test]
async fn get_unknown_market_returns_api_error() {
    let mock = IgMockServer::start().await;
    mock.mount_login_v3().await;
    mock.mount_error(
        "GET",
        "clientsentiment/UNKNOWN.MARKET",
        404,
        "error.public-api.failure.market-not-found",
    )
    .await;

    let client = mock.client();
    client.session().login().await.expect("login");

    let err = client
        .client_sentiment()
        .get("UNKNOWN.MARKET")
        .await
        .expect_err("should fail");

    match err {
        Error::Api { status, source } => {
            assert_eq!(status.as_u16(), 404);
            assert_eq!(
                source.error_code,
                "error.public-api.failure.market-not-found"
            );
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}
