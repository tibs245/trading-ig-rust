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

| feature        | default | description                                              |
| -------------- | ------- | -------------------------------------------------------- |
| `rustls-tls`   | yes     | TLS via `rustls`                                         |
| `native-tls`   | no      | TLS via system OpenSSL                                   |
| `stream`       | no      | Lightstreamer streaming client                           |
| `encryption`   | no      | Encrypted-password login (RSA)                           |
| `live`         | no      | Compile the live integration test suite (read-only path) |
| `live-trading` | no      | Also compile write/mutation live tests (implies `live`)  |

## Live integration tests

A separate test file (`tests/live_integration.rs`) exercises the real IG Demo
API end-to-end.  All tests are `#[ignore]`d and require explicit credentials,
so they never run in CI.

**Read-only tests** (session, accounts, markets, prices, positions, watchlists,
client sentiment, history):

```bash
IG_API_KEY=your-key IG_USERNAME=you IG_PASSWORD=secret \
  cargo test --features live --ignored --test live_integration
```

**Write / mutation tests** (watchlist create/delete, preferences round-trip)
additionally require the `live-trading` feature **and** `IG_LIVE_TRADING_OK=1`:

```bash
IG_API_KEY=your-key IG_USERNAME=you IG_PASSWORD=secret IG_LIVE_TRADING_OK=1 \
  cargo test --features live-trading --ignored --test live_integration
```

If `IG_API_KEY` is absent the whole suite skips gracefully, so
`cargo test --all-features` in CI passes without hitting the network.

## Project knowledge

Internal conventions, architecture decisions, and the IG API spec live
under [`_knowledge/`](_knowledge/). Start with
[`_knowledge/index.md`](_knowledge/index.md) — it lists every file with
a one-line summary so you can load only what you need.

## License

BSD-3-Clause, mirroring the original [`trading-ig`](https://github.com/ig-python/trading-ig)
Python project.
