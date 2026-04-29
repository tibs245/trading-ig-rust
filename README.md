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

## Cargo features

| feature       | default | description                              |
| ------------- | ------- | ---------------------------------------- |
| `rustls-tls`  | yes     | TLS via `rustls`                         |
| `native-tls`  | no      | TLS via system OpenSSL                   |
| `stream`      | no      | Lightstreamer streaming client           |
| `encryption`  | no      | Encrypted-password login (RSA)           |

## Project knowledge

Internal conventions, architecture decisions, and the IG API spec live
under [`_knowledge/`](_knowledge/). Start with
[`_knowledge/index.md`](_knowledge/index.md) — it lists every file with
a one-line summary so you can load only what you need.

## License

BSD-3-Clause, mirroring the original [`trading-ig`](https://github.com/ig-python/trading-ig)
Python project.
