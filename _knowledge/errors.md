# Errors

## Single crate-level enum

There is **one** error type for the entire crate: `crate::Error`. Domain
modules **must not** define their own error enum. If you need a new
failure mode, add a variant to the central `Error` and document it.

Current variants (`src/error.rs`):

- `Http(reqwest::Error)` — transport / IO error.
- `Api { status, source: ApiError }` — IG returned an HTTP error with a
  parseable `errorCode` payload.
- `Auth(String)` — bad credentials, missing session, etc.
- `RateLimited(String)` — explicit rate-limit signal.
- `Deserialization(serde_json::Error)` — unexpected response shape.
- `Config(String)` — invalid configuration at construction time.
- `InvalidInput(String)` — the caller passed a value the wire format
  cannot represent.
- `Url(url::ParseError)`, `HeaderValue(http::header::InvalidHeaderValue)`
  — internal plumbing errors.

## `ApiError` payload

```rust
pub struct ApiError {
    pub error_code: String,                                 // IG's machine-readable code
    pub extra: serde_json::Map<String, serde_json::Value>,  // any other fields IG sent
}
```

`error_code` is the canonical key callers match on. The `extra` map
preserves anything IG sent (e.g. a human-readable `errorMessage`).

## Helpers for common predicates

```rust
err.is_auth()          // token issues, expired session
err.is_rate_limited()  // hit the API quota
```

Add new helpers here, **not** in domain modules.

## Surfacing errors from domain code

Domain methods return `crate::Result<T>`. They do **not** wrap errors
themselves — `Transport::request` already maps non-2xx responses to
`Error::Api { ... }` with the parsed `errorCode`.

Only add explicit error mapping when you need to convert an
*input-validation* failure (which lives entirely in your code, before
the request) into `Error::InvalidInput("…")`.

## Things to NOT do

- Don't `unwrap()` or `expect()` on user-controlled data.
- Don't add a domain-specific error enum and convert to `crate::Error`
  at the boundary. Just use `crate::Error` directly.
- Don't lose the IG `errorCode` by stringifying it into a generic
  message — the structured form is what users will pattern-match on.
