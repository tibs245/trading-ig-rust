# Tracing

We use the `tracing` crate for all logs and observability. No `println!`,
no `log` macros directly.

## Span per public method

Annotate every public async method on a typed domain API:

```rust
#[tracing::instrument(skip_all, fields(epic = %epic, version = 2))]
pub async fn open(&self, ...) -> Result<DealReference> { ... }
```

Conventions:

- **Span name** = the call-site path: `accounts.list`,
  `dealing.positions.open`, `markets.search`.
- `skip_all` by default; add useful `fields(...)` explicitly.
- Always include `epic`, `deal_id`, `account_id`, `watchlist_id` when
  the method takes one of them. Use `%` (Display) or `?` (Debug)
  formatters as appropriate.

## What NOT to log

- **Never** log credentials, OAuth access/refresh tokens, CST,
  X-SECURITY-TOKEN. The HTTP transport already redacts these — don't
  re-introduce them via your span fields.
- Don't log full response bodies at INFO level. Use DEBUG or TRACE.

## Levels

- `trace!` — verbose internal state (request bodies before send).
- `debug!` — request lifecycle ("sending request", "got 200").
- `info!` — meaningful events (login succeeded, session refreshed).
- `warn!` — recoverable anomalies (token expired, retrying).
- `error!` — only when the call is about to return `Err`.

The transport already emits a `debug!("sending request")` per call —
don't duplicate it in domain code.

## Tracing setup in tests / examples

In test or example code, set up a subscriber if you want to see output:

```rust
tracing_subscriber::fmt()
    .with_env_filter("trading_ig=debug")
    .try_init()
    .ok();
```
