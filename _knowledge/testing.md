# Testing

## Layout

- **Unit tests** — next to the code (`#[cfg(test)] mod tests { ... }`)
  for pure logic (parsers, formatters, builders).
- **Integration tests** — under `tests/<domain>.rs`, one file per
  domain. They use the helpers in `tests/support/` to spin up a
  wiremock instance per test.

## Reusable helpers (`tests/support/`)

- `IgMockServer::start().await` — fresh wiremock + IG-shaped client.
- `mock.client()` — returns a pre-built `IgClient` already pointing at
  the mock.
- `mock.mount_login_v3()` / `mount_login_v2()` — mount a session-create
  response that puts the client in an authenticated state.
- `mock.mount_get(path, version, fixture)` — mount a fixture-backed
  GET response, validating `X-IG-API-KEY` and `Version` headers are
  present.
- `mock.mount_error(method, path, status, error_code)` — mount an IG
  error response (`{"errorCode":"…"}`).
- `support::matchers::HasApiKey`, `HasVersion(u8)`, `HasBearer`,
  `HasCstHeaders` — wiremock matchers for IG headers.
- `support::fixtures::load("path/to.json")` — read a fixture file.
- `support::fixtures::load_json("path")` — same, parsed.

If you need a new shared helper, **add it to `tests/support/`** rather
than copy-pasting into each test file.

## Adding a fixture

- Path: `tests/fixtures/json/<domain>/<scenario>.json`.
- Anonymised values: `ABC123` for account ids, `CS.D.GBPUSD.TODAY.IP`
  for epics, made-up timestamps.
- One scenario per file. Don't fold "happy" and "error" into one
  multi-purpose blob.

## Required coverage per domain

Every domain test file (`tests/<domain>.rs`) must cover, at minimum:

1. **Golden path** — happy 200 + valid body, asserts the typed result.
2. **API error** — IG returns 4xx with `{"errorCode":"…"}`. Assert
   `Error::Api { status, source: ApiError { error_code, .. } }`.
3. **Schema edge case** — nullable field present, optional missing,
   or a version-specific field shape.

For endpoints with multiple versions (positions list v1+v2, etc.),
add one test per version.

## What NOT to do

- **Never** hit the real IG API in `cargo test`. Live tests are gated
  behind `#[ignore]` or live in a separate binary.
- Don't share a single `MockServer` across tests — each test owns its
  own (wiremock is cheap, and parallel `cargo test` works correctly).
- Don't assert on full JSON bodies via string equality. Deserialise to
  the typed model and assert on fields. (`pretty_assertions` is
  available for nicer diffs.)
- Don't write fixtures by hand if you have access to a captured real
  response — anonymise it and drop it as-is.

## Running

```bash
cargo test --all-targets        # unit + integration
cargo clippy --all-targets --no-deps
```

Both must be clean before marking a task done.
