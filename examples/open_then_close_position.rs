//! Demonstrate the open/close position API surface without sending real orders.
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
//! cargo run --example open_then_close_position
//! ```
//!
//! The code guarded by `cfg(any())` is **never compiled** — it exists purely to
//! show the API surface.  Remove that gate and supply real values to send an
//! actual order.

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

    // List current open positions as a safety check.
    let positions = client.dealing().positions().list().await?;
    println!("Open positions: {}", positions.len());

    if !positions.is_empty() {
        println!("Positions already open — not opening a new one (safety guard).");
        for p in &positions {
            println!(
                "  deal_id={} direction={:?} size={}",
                p.deal_id, p.direction, p.size
            );
        }
        return Ok(());
    }

    // -------------------------------------------------------------------------
    // The block below is gated with `cfg(any())` so it is NEVER compiled.
    // It demonstrates the type-state builder syntax for opening and closing a
    // position.  To use it for real:
    //   1. Remove the `#[cfg(any())]` attribute.
    //   2. Replace the example values with valid values for your account.
    //   3. Ensure you are on a demo account before running.
    // -------------------------------------------------------------------------
    #[cfg(any())]
    {
        use trading_ig::dealing::positions::models::{ClosePositionRequest, Direction, OrderType};
        use trading_ig::models::common::{Direction, Epic, OrderType};

        // Open a position using the compile-time type-state builder.
        // All seven mandatory fields must be set; the compiler rejects .send()
        // until they are all present.
        let confirmation = client
            .dealing()
            .positions()
            .open()
            .epic(Epic::new("CS.D.GBPUSD.TODAY.IP"))
            .direction(Direction::Buy)
            .size(1.0)
            .order_type(OrderType::Market)
            .currency("GBP")
            .expiry("DFB")
            .guaranteed_stop(false)
            .with_stop_distance(20.0) // optional: 20-point stop
            .send()
            .await?;

        println!("Opened position: deal_id={}", confirmation.deal_id);

        // Close the position we just opened.
        let close_req = ClosePositionRequest {
            deal_id: Some(confirmation.deal_id),
            direction: Direction::Sell, // must be opposite to the open direction
            epic: None,
            expiry: None,
            level: None,
            order_type: OrderType::Market,
            quote_id: None,
            size: 1.0,
            time_in_force: None,
        };

        let close_confirmation = client.dealing().positions().close(close_req).await?;
        println!(
            "Closed position: deal_id={} status={:?}",
            close_confirmation.deal_id, close_confirmation.status
        );
    }

    println!("No positions open. Showing API syntax only (see cfg(any()) block).");
    Ok(())
}
