//! Log in with v3 session and list all accounts.
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
//! cargo run --example login_and_list_accounts
//! ```

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

    let accounts = client.accounts().list().await?;
    println!("Found {} account(s):", accounts.len());
    for account in &accounts {
        println!(
            "  {} — {} ({:?})",
            account.account_id, account.account_name, account.account_type
        );
    }

    Ok(())
}
