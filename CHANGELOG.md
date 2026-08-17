# Changelog

All notable changes to `trading-ig` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] — 0.1.6

### Deprecated

- **`StreamingClient::subscribe_market`.** IG deprecated the Lightstreamer
  `MARKET` subscription (and its `L1` alias): end of life 1 May 2026,
  decommissioned 8 May 2026. Use
  [`subscribe_price`](src/streaming/client.rs) instead. The method carries
  `#[deprecated]` and also emits a `tracing::warn!` once per process, so
  already-deployed bots see the notice in their logs and not just at
  compile time.

### Added

- **`PRICE` subscription** — `subscribe_price(epic)` and
  `subscribe_price_for_account(account_id, epic)`, returning the new
  `PriceUpdate`. The item name is `PRICE:<accountId>:<epic>` and the
  subscription selects the `Pricing` data adapter.
- **`LS_data_adapter` on `control.txt`.** The Lightstreamer control request
  can now select a data adapter; `PRICE` is the only family that needs one.
- `PriceUpdate` exposes the 5-tier dealing ladder (`bid_prices`,
  `ask_prices`, `bid_sizes`, `ask_sizes`), the dealing quote IDs
  (`bid_quote_id`, `ask_quote_id`) and the ladder currency — none of which
  `MARKET` provided.
- `CandleScale::Second`, matching the `SECOND` scale IG documents.
- Seven `CHART:<epic>:<scale>` fields that were missing from the candle
  subscription: `LTV`, `TTV`, `DAY_OPEN_MID`, `DAY_NET_CHG_MID`,
  `DAY_PERC_CHG_MID`, `DAY_HIGH`, `DAY_LOW`.
- `ACCOUNT` fields `PNL_LR` / `PNL_NLR` (`AccountUpdate::pnl_lr` /
  `pnl_nlr`).

### Changed

- **Breaking (pre-1.0):** `ChartCandleUpdate` and `AccountUpdate` gained
  public fields, so exhaustive struct literals and patterns need updating.
  Both derive `Default`, so `..Default::default()` is the cheap fix.
- `examples/streaming_market.rs` renamed to `examples/streaming_price.rs`
  and switched to `subscribe_price`.

### Migration notes

`MARKET` → `PRICE` is not a rename. Beyond the item name gaining the
account id, two fields change meaning: `UPDATE_TIME` (UK-local
`HH:MM:SS` string) becomes `timestamp` (UTC milliseconds), and
`MARKET_STATE` becomes `dlg_flag`, which uses a different vocabulary
(`DEAL` where `MARKET` said `TRADEABLE`). Full mapping table in
`_knowledge/api/streaming.md`.

`LS_data_adapter=Pricing` is taken from IG's published reference and has
not yet been exercised against a live demo session.

## [0.1.5] — 2026-07-21

### Security

- Bumped vulnerable transitive dependencies flagged by `cargo audit` /
  `cargo deny`. `Cargo.lock` only — no public API change.
  - `quinn-proto` 0.11.14 → 0.11.16 — RUSTSEC-2026-0185 (remote memory
    exhaustion via unbounded out-of-order stream reassembly).
  - `crossbeam-epoch` 0.9.18 → 0.9.20 — RUSTSEC-2026-0204 (invalid
    pointer dereference in `fmt::Display`).
  - `anyhow` 1.0.102 → 1.0.104 — RUSTSEC-2026-0190 (unsoundness in
    `Error::downcast_mut()`).
  - `memmap2` 0.9.10 → 0.9.11 — RUSTSEC-2026-0186 (unchecked pointer
    offset).
  - `spin` 0.9.8 → 0.9.9 — replaces the yanked 0.9.8.

## [0.1.3] — 2026-06-05

### Fixed

- **`close_position` rejected with `validation.null-not-allowed.request`.**
  IG silently drops the body of real `DELETE` requests, so every
  required field on `ClosePositionRequest` was reported as null. The
  transport now mirrors the official Python `trading-ig` client and
  rewrites body-carrying `DELETE`s as `POST` with an `_method: DELETE`
  header. Bodyless DELETEs (`session` logout, `watchlists` delete,
  `workingorders` delete) are untouched. Validated live on a demo
  account: position fully closed, expected P/L confirmed.
- Defence in depth: `ClosePositionRequest` now carries
  `#[serde(skip_serializing_if = "Option::is_none")]` on every optional
  field, so explicit `null`s never reach IG even if a future code path
  bypasses the transport rewrite.

### Added

- `examples/diag_close_position.rs` — list open positions and close the
  first, logging the outbound JSON. Handy when reproducing the
  null-not-allowed bug or validating the rewrite against a fresh demo
  account.
- `tests/dealing_positions.rs::close_position_rewrites_delete_to_post_with_method_override`
  pins the rewrite + asserts the JSON body survives intact.
- `tests/support/matchers::HasMethodOverride` and
  `IgMockServer::mount_delete_error` helpers for tests that target
  body-carrying DELETE endpoints.

## [0.1.1] — 2026-04-29

### Added

- **`session().login_with_encryption()`** (behind the `encryption`
  cargo feature) — log in v3 with an RSA-encrypted password instead
  of plaintext. Recommended for accounts holding real funds (live or
  funded demo). Internally fetches the encryption key, encrypts the
  password with PKCS#1 v1.5, and posts to `/session` with
  `encryptedPassword=true`.
- **`SECURITY.md`** — vulnerability reporting policy (FR + EN),
  maintainer contact email <thibault.barske@kolombo.xyz>, encryption
  recommendation for funded accounts, defensive practices the crate
  enforces.
- README "Recommended for funded accounts" section pointing at the
  new helper.

### Fixed

- `Cargo.toml`: corrected the `repository` URL to point at
  `tibs245/trading-ig-rust`. Added `homepage` and `documentation`
  metadata fields, and configured `[package.metadata.docs.rs]` so
  docs.rs builds with all features.
- `streaming::reconnect` doc comment: fix broken intra-doc link to
  `StreamingApi::connect_with`.
- `streaming::events::MarketUpdate::from_raw` doc comment: drop
  intra-doc link to a private `MARKET_FIELDS` constant (was
  `cargo doc -D warnings` failure on Rust 1.95).
- Two `Duration::from_secs(60)` call sites updated to
  `Duration::from_mins(1)` to satisfy Rust 1.95's new
  `clippy::duration_suboptimal_units` lint.

### Security

- [RUSTSEC-2023-0071] (`rsa` crate Marvin timing side-channel on
  PKCS#1 v1.5 *decryption*): acknowledged but **not applicable** to
  this crate — we only ever encrypt with IG's public key, never
  decrypt. An `ignore` is documented in `deny.toml` and in the
  `cargo audit` invocations of both CI workflows. See `SECURITY.md`
  for the full rationale.
- `cargo deny` is now wired into the weekly security workflow
  (`.github/workflows/audit.yml`) — checks advisories, licenses,
  banned crates, and source provenance.
- `cargo audit` is also added to the `pre-push` git hook (skipped if
  the binary isn't installed locally; CI runs it unconditionally).

[RUSTSEC-2023-0071]: https://rustsec.org/advisories/RUSTSEC-2023-0071

## [0.1.0] — 2026-04-29

Initial release. Async Rust port of the [`trading-ig`](https://github.com/ig-python/trading-ig)
Python client. Covers the full IG Markets REST surface and a Lightstreamer
streaming client with auto-reconnect.

### REST endpoints (44 in total, all async)

- **Session**: `login` (v3 OAuth), `login_v2` (CST/XST headers), `refresh`,
  `read(fetch_tokens)`, `switch_account`, `logout`, `encryption_key`
  (behind the `encryption` feature).
- **Accounts**: `list`, `preferences`, `update_preferences`.
- **Markets**: `search`, `get`, `get_many` (v2 bulk), `navigation`,
  `navigation_node`.
- **Prices** (3 distinct endpoints): `history_v3` + `history_v3_all`
  (auto-paginated), `history_by_num_points_v2`, `history_by_date_range_v2`,
  `history_by_date_range_v1`.
- **Dealing — positions**: `list`, `list_v1`, `list_v2`, `get`, `open`
  (type-state builder), `update`, `close`, `confirm` (with 5× × 1 s retry).
- **Dealing — working orders**: `list`, `list_v1`, `list_v2`, `create`
  (type-state builder), `update`, `delete`.
- **Watchlists**: `list`, `create`, `markets`, `add_market`,
  `remove_market`, `delete`.
- **Client sentiment**: `get`, `get_many`, `related`.
- **History**: `activity_v3` (auto-paginated next-URL),
  `activity_by_period_v1`, `activity_by_date_range_v1`, `transactions_v2`,
  `transactions_by_period_v1`.
- **Operations**: `applications`, `update_application`,
  `disable_current_key`.
- **Repeat dealing**: `window`, `window_for(epic)` (with 5× × 1 s retry).

### Streaming (behind the `stream` feature)

- Lightstreamer TLCP client with `MARKET`, `CHART:TICK`,
  `CHART:<scale>`, `ACCOUNT`, `TRADE` subscriptions.
- Each subscription returns a `tokio::sync::mpsc::Receiver<T>` of typed
  updates.
- Auto-reconnect on server `END` / unrecoverable session loss, with
  configurable back-off (`AutoReconnect` policy). Subscribers' channels
  are reused transparently across reconnects.
- A `StreamingEvent` channel is returned alongside the `StreamingClient`
  so callers can observe `Reconnected`, `ReconnectFailed`, and
  `Disconnected` events.

### Cross-cutting

- Strongly typed: every request / response is a `serde` struct or enum.
  Newtypes for `Epic`, `DealId`, `DealReference`, `Currency`. Common
  enums (`Direction`, `OrderType`, `TimeInForce`, `InstrumentType`,
  `MarketStatus`) under `trading_ig::models`.
- Single crate-level `Error` enum surfacing IG's `errorCode` payload via
  `Error::Api { status, source: ApiError { error_code, .. } }`. Helper
  predicates `is_auth()` and `is_rate_limited()`.
- Structured logs / spans via the `tracing` crate. Tokens and
  credentials are never logged.
- Date / time helpers in `trading_ig::time` covering IG's three input
  formats (v1 `Y:m:d-H:M:S`, v2 `Y/m/d H:M:S`, v3 ISO-8601).

### Cargo features

| feature       | default | description                              |
| ------------- | ------- | ---------------------------------------- |
| `rustls-tls`  | yes     | TLS via `rustls`                         |
| `native-tls`  | no      | TLS via system OpenSSL                   |
| `stream`      | no      | Lightstreamer streaming client           |
| `encryption`  | no      | Encrypted-password login (RSA PKCS#1v15) |

### Known limitations

- No client-side rate limiter yet (IG enforces 30 req/min trading,
  60 req/min non-trading).
- No automatic OAuth-on-401 refresh in the transport — call
  `session().refresh()` explicitly when needed.
- No sync wrapper. Async-only on `tokio`.
- Lightstreamer reconnect logic is covered by the public-API tests but
  not yet by an end-to-end TCP-level mock; the network path is exercised
  in live smoke tests against demo-api.ig.com.
