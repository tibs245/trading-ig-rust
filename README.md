# trading-ig

Async Rust client for the [IG Markets](https://labs.ig.com/) REST and
Lightstreamer streaming APIs.

> Status: **early development** — Vague 0 (foundations) in progress.
> Not yet published on crates.io.

## Goals

- **Async-first** (`tokio`), `reqwest` under the hood.
- **Strongly typed**: all requests/responses modelled with `serde`.
- **Composable**: usable as a library independently of any framework.
- **Minimal dependencies**.
- **Structured logs/tracing** via the `tracing` crate.
- **Well-tested**: HTTP responses are covered by mock-server tests built
  from reusable fixtures.

## Quick start

```rust
use trading_ig::{IgClient, Environment, Credentials};

#[tokio::main]
async fn main() -> trading_ig::Result<()> {
    let client = IgClient::builder()
        .environment(Environment::Demo)
        .api_key(std::env::var("IG_API_KEY")?)
        .credentials(Credentials::password(
            std::env::var("IG_USERNAME")?,
            std::env::var("IG_PASSWORD")?,
        ))
        .build()?;

    client.session().login().await?;

    let accounts = client.accounts().list().await?;
    for account in accounts {
        println!("{} ({})", account.account_name, account.account_id);
    }
    Ok(())
}
```

### Examples

Three self-contained examples live in [`examples/`](examples/):

| Example | What it shows |
| ------- | ------------- |
| [`login_and_list_accounts`](examples/login_and_list_accounts.rs) | Log in (v3) and print all account IDs |
| [`search_market_and_get_history`](examples/search_market_and_get_history.rs) | Search for EUR/USD, fetch the last hour of minute bars |
| [`open_then_close_position`](examples/open_then_close_position.rs) | Type-state builder syntax for opening and closing a position |

All examples read credentials from `IG_API_KEY`, `IG_USERNAME`, and `IG_PASSWORD`:

```bash
IG_API_KEY=xxx IG_USERNAME=you IG_PASSWORD=secret \
  cargo run --example login_and_list_accounts
```

## Cargo features

| feature       | default | description                                          |
| ------------- | ------- | ---------------------------------------------------- |
| `rustls-tls`  | yes     | TLS via `rustls`                                     |
| `native-tls`  | no      | TLS via system OpenSSL                               |
| `stream`      | no      | Lightstreamer streaming client                       |
| `encryption`  | no      | Encrypted-password login (RSA)                       |
| `polars`      | no      | Conversions from tabular API responses to `DataFrame`|

### `polars` feature

Enable the `polars` feature to convert tabular API responses directly into
[Polars](https://docs.rs/polars) `DataFrame`s for analysis:

```toml
[dependencies]
trading-ig = { version = "0.1", features = ["polars"] }
```

```rust
use trading_ig::dataframe::IntoDataFrame;

// Convert a list of open positions into a DataFrame.
let positions = client.dealing().positions().list_v2().await?;
let df = positions.to_dataframe()?;
println!("{df}");

// Convert historical prices into a DataFrame (one row per bar).
let prices = client.prices().history_v3(&epic, Default::default()).await?;
let df = prices.to_dataframe()?;
// df columns: snapshot_time, snapshot_time_utc, open_bid, open_ask,
//             high_bid, high_ask, low_bid, low_ask, close_bid, close_ask,
//             last_traded_volume
println!("{df}");
```

The `IntoDataFrame` trait is implemented on:

| Type                  | Source                     |
| --------------------- | -------------------------- |
| `Vec<Account>`        | `client.accounts().list()` |
| `Vec<PositionV2>`     | `client.dealing().positions().list_v2()` |
| `Vec<WorkingOrderV2>` | `client.dealing().working_orders().list_v2()` |
| `HistoricalPrices`    | `client.prices().history_v3(…)` |
| `Vec<Activity>`       | `client.history().activity_v3(…)` |
| `Vec<Transaction>`    | `client.history().transactions_v2(…).transactions` |
| `Vec<MarketSummary>`  | `client.markets().search(…)` / watchlists |
| `Vec<Sentiment>`      | `client.client_sentiment().get_many(…)` |

## Project knowledge

Internal conventions, architecture decisions, and the IG API spec live
under [`_knowledge/`](_knowledge/). Start with
[`_knowledge/index.md`](_knowledge/index.md) — it lists every file with
a one-line summary so you can load only what you need.

## License

BSD-3-Clause, mirroring the original [`trading-ig`](https://github.com/ig-python/trading-ig)
Python project.
