//! Search for a market and fetch the last hour of minute bars.
//!
//! Set the following environment variables before running:
//!
//! ```text
//! IG_API_KEY=your_api_key
//! IG_USERNAME=your_username
//! IG_PASSWORD=your_password
//! ```
//!
//! Run with:
//! ```text
//! cargo run --example search_market_and_get_history
//! ```

use chrono::Utc;
use trading_ig::prices::models::{HistoricalPricesRequest, Resolution};
use trading_ig::{Credentials, Environment, IgClient};

#[tokio::main]
async fn main() -> trading_ig::Result<()> {
    tracing_subscriber::fmt::init();

    let api_key = std::env::var("IG_API_KEY").expect("IG_API_KEY must be set");
    let username = std::env::var("IG_USERNAME").expect("IG_USERNAME must be set");
    let password = std::env::var("IG_PASSWORD").expect("IG_PASSWORD must be set");

    let client = IgClient::builder()
        .environment(Environment::Demo)
        .api_key(api_key)
        .credentials(Credentials::password(username, password))
        .build()?;

    client.session().login().await?;

    // Search for EUR/USD markets.
    let results = client.markets().search("EUR/USD").await?;
    println!(
        "Search returned {} result(s) for \"EUR/USD\"",
        results.len()
    );

    let Some(first) = results.into_iter().next() else {
        println!("No markets found — exiting.");
        return Ok(());
    };

    println!(
        "Using first result: {} ({})",
        first.instrument_name, first.epic
    );

    // Fetch the last hour of one-minute bars via the v3 endpoint.
    let now = Utc::now().naive_utc();
    let one_hour_ago = now - chrono::Duration::hours(1);

    let req = HistoricalPricesRequest {
        resolution: Some(Resolution::Minute),
        from: Some(one_hour_ago),
        to: Some(now),
        ..Default::default()
    };

    let prices = client.prices().history_v3(&first.epic, req).await?;
    println!(
        "Received {} bar(s) for the last hour (instrument type: {})",
        prices.prices.len(),
        prices.instrument_type
    );

    Ok(())
}
