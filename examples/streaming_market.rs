//! Live streaming smoke test — connects to the IG demo API and subscribes to
//! EUR/USD market updates, printing 5 updates then disconnecting.
//!
//! # Requirements
//!
//! - The `stream` Cargo feature must be enabled.
//! - Credentials must be supplied via environment variables (see below).
//!
//! # Usage
//!
//! ```bash
//! # Source your .env file, then run:
//! ( set -a; . /path/to/.env; set +a; \
//!   cargo run --features stream --quiet --example streaming_market )
//! ```
//!
//! # Required environment variables
//!
//! | Variable         | Description                       |
//! | ---------------- | --------------------------------- |
//! | `IG_API_KEY`     | Your IG API key                   |
//! | `IG_IDENTIFIER`  | Your IG account username          |
//! | `IG_PASSWORD`    | Your IG account password          |
//! | `IG_ACC_TYPE`    | `DEMO` or `LIVE` (default: DEMO)  |
//! | `IG_EPIC`        | Epic to subscribe to (optional)   |
//!
//! The example exits cleanly after receiving 5 market updates or after a 30 s timeout.
//!
//! # Note
//!
//! This example requires the `stream` feature:
//! `cargo run --features stream --example streaming_market`

// When the stream feature is enabled, compile the real streaming main.
#[cfg(feature = "stream")]
mod stream_impl {
    use std::time::Duration;

    use trading_ig::streaming::events::CandleScale;
    use trading_ig::{Credentials, Environment, IgClient, Result};

    pub async fn run() -> Result<()> {
        tracing_subscriber::fmt()
            .with_env_filter("trading_ig=debug,streaming_market=info")
            .try_init()
            .ok();

        // Read configuration from environment variables.
        // Supports both `IG_IDENTIFIER` and the alternative `IG_USERNAME` spelling.
        let api_key = std::env::var("IG_API_KEY")
            .unwrap_or_else(|_| panic!("IG_API_KEY not set — source your .env first"));
        let identifier = std::env::var("IG_IDENTIFIER")
            .or_else(|_| std::env::var("IG_USERNAME"))
            .unwrap_or_else(|_| panic!("IG_IDENTIFIER (or IG_USERNAME) not set"));
        let password =
            std::env::var("IG_PASSWORD").unwrap_or_else(|_| panic!("IG_PASSWORD not set"));

        // IG_ACC_TYPE="DEMO"/"LIVE" or IG_ACC_NUMBER (account-level selection, defaults Demo).
        let env_type = std::env::var("IG_ACC_TYPE").unwrap_or_else(|_| "DEMO".into());
        let env = match env_type.to_uppercase().as_str() {
            "LIVE" => Environment::Live,
            _ => Environment::Demo,
        };

        let epic = std::env::var("IG_EPIC").unwrap_or_else(|_| "CS.D.EURUSD.CFD.IP".to_string());

        println!("Connecting to IG ({env:?}) …");

        // Build client.
        let client = IgClient::builder()
            .environment(env)
            .api_key(&api_key)
            .credentials(Credentials::password(identifier, password))
            .build()?;

        // Try v2 login first (gives CST/XST directly, needed by Lightstreamer).
        // If v2 fails (e.g. encrypted-password accounts), fall back to v3 + read(true).
        let session_info = match client.session().login_v2().await {
            Ok(info) => info,
            Err(e) => {
                println!("v2 login failed ({e}), trying v3 + read(true)…");
                let v3 = client.session().login().await?;
                // Exchange the OAuth token for CST/XST so Lightstreamer can use them.
                client.session().read(true).await?;
                v3
            }
        };
        println!(
            "Logged in as account {} (endpoint: {})",
            session_info.account_id, session_info.lightstreamer_endpoint
        );

        // Connect to Lightstreamer (auto-reconnect enabled by default).
        let (stream, _events) = client.streaming().connect().await?;
        println!("Lightstreamer session: {}", stream.session_id());

        // Subscribe to market price updates.
        let mut market_rx = stream.subscribe_market(&epic).await?;
        println!("Subscribed to MARKET:{epic}");

        // Also subscribe to 1-minute candles as a second subscription.
        let mut candle_rx = stream
            .subscribe_chart_candle(&epic, CandleScale::OneMinute)
            .await?;
        println!("Subscribed to CHART:{epic}:1MINUTE");

        let mut count = 0usize;
        let timeout = tokio::time::sleep(Duration::from_secs(30));
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                Some(update) = market_rx.recv() => {
                    println!(
                        "[market] {}: bid={:?} offer={:?} state={:?} delay={:?} time={:?}",
                        update.epic,
                        update.bid,
                        update.offer,
                        update.market_state,
                        update.market_delay,
                        update.update_time,
                    );
                    count += 1;
                    if count >= 5 {
                        println!("Received {count} updates — disconnecting.");
                        break;
                    }
                }
                Some(candle) = candle_rx.recv() => {
                    println!(
                        "[candle] {} {:?}: ofr_open={:?} ofr_close={:?} cons_end={:?}",
                        candle.epic,
                        candle.scale,
                        candle.ofr_open,
                        candle.ofr_close,
                        candle.cons_end,
                    );
                }
                () = &mut timeout => {
                    println!("Timeout reached after 30 s with {count} updates received.");
                    break;
                }
            }
        }

        stream.disconnect().await?;
        println!("Disconnected cleanly.");
        Ok(())
    }
}

#[cfg(feature = "stream")]
#[tokio::main]
async fn main() {
    if let Err(e) = stream_impl::run().await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

/// Stub main when the `stream` feature is not enabled.
#[cfg(not(feature = "stream"))]
fn main() {
    eprintln!(
        "This example requires the `stream` Cargo feature.\n\
         Run with: cargo run --features stream --example streaming_market"
    );
    std::process::exit(1);
}
