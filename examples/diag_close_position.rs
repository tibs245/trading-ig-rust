//! List open positions then close the first. Prints the outbound JSON
//! and the full error chain, handy for transport-level diagnostics.
//!
//! ```bash
//! IG_API_KEY=… IG_USERNAME=… IG_PASSWORD=… \
//!     cargo run --example diag_close_position
//! ```

use trading_ig::dealing::positions::ClosePositionRequest;
use trading_ig::models::common::{Direction, OrderType};
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
    println!("=== login OK ===");

    let positions = client.dealing().positions().list().await?;
    println!("=== open positions: {} ===", positions.len());

    if positions.is_empty() {
        println!("No positions to close. Exit.");
        return Ok(());
    }

    for p in &positions {
        println!(
            "  deal_id={} epic={:?} direction={:?} size={}",
            p.deal_id, p.market.epic, p.direction, p.size
        );
    }

    let first = &positions[0];
    let close_dir = match first.direction {
        Direction::Buy => Direction::Sell,
        Direction::Sell => Direction::Buy,
    };

    let close_req = ClosePositionRequest {
        deal_id: Some(first.deal_id.clone()),
        direction: close_dir,
        epic: None,
        expiry: None,
        level: None,
        order_type: OrderType::Market,
        quote_id: None,
        size: first.size,
        time_in_force: None,
    };

    let json = serde_json::to_string_pretty(&close_req).expect("serialises");
    println!("=== outbound JSON ===\n{json}\n=====================");

    match client.dealing().positions().close(close_req).await {
        Ok(conf) => println!("=== close OK ===\n{conf:#?}"),
        Err(e) => {
            println!("=== close FAILED ===");
            println!("error chain (Debug): {e:#?}");
            println!("error chain (Display): {e}");
            let mut src: Option<&dyn std::error::Error> = std::error::Error::source(&e);
            let mut depth = 0;
            while let Some(s) = src {
                println!("  caused by [{depth}]: {s}");
                src = s.source();
                depth += 1;
            }
        }
    }

    Ok(())
}
