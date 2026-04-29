# Internal conventions for `trading-ig` (Rust port)

This file captures conventions agreed upon at project bootstrapping. Read
it before adding code or spawning subagents.

## Crate layout

```
src/
├── lib.rs              # public re-exports + crate-level docs
├── error.rs            # Error / Result
├── config.rs           # Environment (Demo/Live), URLs, defaults
├── client/             # IgClient + builder + low-level HTTP transport
├── session/            # Auth (v1/v2/v3), token store, refresh logic
├── models/             # Cross-cutting types (Currency, Direction, …)
├── time.rs             # Date/time conversions per IG API version
├── rate_limit.rs       # Token-bucket rate limiter
├── accounts/           # Domain modules (Vague 1) — one per IG domain
├── markets/
├── dealing/
├── prices/
├── watchlists/
├── client_sentiment/
├── history/
└── streaming/          # (Vague 3) Lightstreamer client
```

Each domain module exposes a typed entry point on the client:

```rust
client.accounts().list().await?;
client.markets().search("EUR/USD").await?;
client.dealing().positions().open(...).await?;
```

The accessor (`fn accounts(&self) -> AccountsApi<'_>`) borrows the client
so users can drop it without ceremony.

## Naming

- Public types use PascalCase: `OpenPositionRequest`, `MarketDetails`.
- Field names: snake_case in Rust, mapped to IG's camelCase via
  `#[serde(rename_all = "camelCase")]` at the struct level when possible,
  or per-field `#[serde(rename = "…")]` for irregularities.
- Endpoint methods on the typed API are short verbs: `list`, `get`,
  `search`, `open`, `close`, `update`.
- Module names singular when the module wraps a single concept
  (`session`), plural when it groups items (`accounts`, `markets`).

## Errors

- Single crate-level `Error` enum in `src/error.rs` using `thiserror`.
- Variants: `Http`, `Api { code, message, status }`, `Auth`, `RateLimited`,
  `Deserialization`, `Config`, `InvalidInput`.
- `Result<T> = std::result::Result<T, Error>`.
- Domain modules **never** define their own error enum. Add a variant to
  the central `Error` if you need a new failure mode.

## Tracing

- Use `tracing::instrument` on every public async method.
- Span name = endpoint method, e.g. `accounts.list`, `dealing.open_position`.
- Always include `epic`, `deal_id`, `account_id` as fields when relevant.
- Never log credentials, OAuth tokens, CST, or X-SECURITY-TOKEN values.
  Helpers in `client/http.rs` redact these automatically.

## HTTP transport

- All requests go through `client::http::Transport::request(...)`. Domain
  modules **must not** call `reqwest` directly.
- Auth headers (Version, X-IG-API-KEY, CST/XST or Bearer) are injected by
  the transport based on the active session state.
- Date formatting is centralised in `src/time.rs`; do not reimplement it
  in domain modules.

## Tests

- Unit tests live next to the code (`#[cfg(test)] mod tests { ... }`).
- Endpoint behaviour is verified by integration tests under `tests/`,
  using the helpers in `tests/support/`.
- Fixture JSONs go under `tests/fixtures/json/<domain>/<scenario>.json`
  and are loaded by `tests/support/fixtures.rs::load("...")`.
- Each domain integration test should cover: golden path, server error
  (4xx with API errorCode), and at least one schema edge case (nullable
  field, missing optional, version variant).
- Never hit the real IG API in `cargo test`. Live integration tests are
  gated behind a separate binary or `#[ignore]`d test suite.

## Adding a new endpoint

1. Add request/response structs to `<domain>/models.rs` (serde).
2. Add the method to `<domain>/api.rs`, calling `Transport::request`.
3. Drop one or more JSON fixtures under `tests/fixtures/json/<domain>/`.
4. Write an integration test in `tests/<domain>.rs` using `IgMockServer`.
5. If a new shared concept appears (e.g. a new enum), put it in
   `src/models/common.rs` so other domains can re-use it.

## Versioning policy (recap from `docs/API_CATALOG.md`)

- IG endpoints are versioned via the `Version` HTTP header (1, 2, 3).
- **We expose every version that the Python `trading-ig` library
  exposes**, no more, no less. This keeps the Rust port a true 1:1
  alternative.
- When an endpoint has multiple versions with **different response
  schemas**, expose them as separate methods with explicit names rather
  than a `version: u8` argument:

  ```rust
  // Good — schemas differ, methods carry the version
  client.dealing().positions().list_v1().await?;   // -> PositionsV1
  client.dealing().positions().list_v2().await?;   // -> PositionsV2

  // Acceptable — historical prices: three independent endpoints
  client.prices().history_by_epic_v3(epic, opts).await?;
  client.prices().history_by_num_points_v2(epic, res, n).await?;
  client.prices().history_by_date_range_v1(...).await?;
  client.prices().history_by_date_range_v2(...).await?;
  ```

- The version method without a `_vN` suffix (e.g. `list()`) aliases the
  highest version (matches Python's default `version="2"` arg behaviour).
  Document the alias in the doc comment.
- Versions to cover, per the Python source:
  - **Session**: v2 (CST/XST in headers) + v3 (OAuth) — both, plus
    refresh-token endpoint (v1).
  - **Positions list** (`/positions`): v1 + v2 (`list_v1`, `list_v2`,
    `list` = v2).
  - **Position by deal id** (`/positions/{dealId}`): v2.
  - **Open / update / close position**: v2 (open + update), v1 (close).
  - **Working orders list** (`/workingorders`): v1 + v2 (alias = v2).
  - **Working orders create / update / delete**: v2.
  - **Markets**: search v1, fetch by epics v2, fetch by epic v3.
  - **Historical prices**: three distinct endpoints in v1, v2, v3 — port
    all three (Python wraps them as separate methods).
  - **Activity history**: v1 (period and date), v3 (paginated). Both.
  - **Transaction history**: v1 (period) + v2 (paginated). Both.
  - **All other domains** (watchlists, sentiment, accounts, operations,
    repeat dealing): single version each per Python.
- Date formats differ by version — see `src/time.rs`.

## Out of scope

- No pandas / DataFrame conversions in core. (Optional `polars` feature
  may be added later — do not preempt.)
- No sync API wrapper. Async only.
