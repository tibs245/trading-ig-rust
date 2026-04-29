# Architecture

## Crate layout

```
src/
├── lib.rs              # public re-exports + crate-level docs
├── error.rs            # Error / Result (see errors.md)
├── config.rs           # Environment (Demo/Live/Custom), IgConfig
├── client/             # IgClient + builder + low-level HTTP transport
├── session/            # Auth (v1/v2/v3), token store, refresh logic
├── models/             # Cross-cutting types (Currency, Direction, …)
├── time.rs             # Date/time conversions per IG API version
├── accounts/           # Domain modules (Vague 1) — one per IG domain
├── markets/
├── dealing/positions/
├── dealing/working_orders/
├── prices/
├── watchlists/
├── client_sentiment/
├── history/
└── streaming/          # (Vague 3) Lightstreamer client
```

## Domain module shape

A domain module looks like this:

```
src/<domain>/
├── mod.rs              # `pub struct <Domain>Api<'a> { client: &'a IgClient }`
├── api.rs              # endpoint methods (list / get / create / …)
├── models.rs           # request + response structs (serde)
└── builder.rs          # type-state builder(s), only when needed
```

## The `IgClient` → typed domain-API pattern

Each domain exposes a typed accessor on `IgClient`:

```rust
impl IgClient {
    pub fn accounts(&self) -> AccountsApi<'_> { AccountsApi { client: self } }
    pub fn markets(&self)  -> MarketsApi<'_>  { MarketsApi  { client: self } }
    pub fn dealing(&self)  -> DealingApi<'_>  { DealingApi  { client: self } }
    // ...
}
```

For nested domains (`dealing/positions`):

```rust
impl<'a> DealingApi<'a> {
    pub fn positions(&self) -> PositionsApi<'a> { ... }
    pub fn working_orders(&self) -> WorkingOrdersApi<'a> { ... }
}
```

This gives the call sites:

```rust
client.accounts().list().await?;
client.dealing().positions().list_v2().await?;
client.markets().get(&epic).await?;
```

## Composability

`IgClient` is `Clone` (Arc inside) — pass it by value, store it in
structs, share it across tasks. The HTTP `Transport` and shared session
state are reference-counted; nothing locks unless a session refresh is
in flight.

The crate is meant to be usable standalone (no framework assumptions):
no global state, no required runtime configuration, only `tokio` as the
async executor.

## What goes where — quick decision tree

- A new HTTP call to IG → in the matching domain's `api.rs`.
- A new request/response type used by **one** endpoint → that domain's
  `models.rs`.
- A type used by **two or more** domains (e.g. `Direction`) →
  `src/models/common.rs`.
- A new session-level mechanism (token kind, auth header) → `session/`.
- Anything that touches `reqwest` directly → forbidden in domain code;
  extend `client/http.rs` instead.
