# HTTP transport

All outbound HTTP calls go through `crate::client::http::Transport`.
**Domain modules MUST NOT call `reqwest` directly.** This invariant is
what makes auth, tracing, and error mapping work uniformly.

## How to call it

The transport exposes one method you'll use in domain code:

```rust
pub(crate) async fn request<B, R>(
    &self,
    method: Method,
    path: &str,                  // joined to the environment base URL
    version: Option<u8>,         // → "Version" header (None to skip)
    body: Option<&B>,            // serialised as JSON if Some
    session: &SharedSession,     // reads tokens, sets auth headers
) -> Result<R>
where B: Serialize + ?Sized, R: DeserializeOwned;
```

Typical use:

```rust
let body: Envelope = self.client.transport
    .request(Method::GET, "accounts", Some(1), None::<&()>, &self.client.session)
    .await?;
```

For a body:

```rust
self.client.transport
    .request::<_, ()>(Method::PUT, &format!("watchlists/{id}"), Some(1),
                      Some(&AddMarket { epic }), &self.client.session)
    .await?;
```

## What `Transport` does for you

- Builds the URL by joining `path` to the active `Environment`'s base.
- Injects mandatory headers: `X-IG-API-KEY`, `Accept`,
  `Content-Type`, `Version`.
- Reads the active session and injects auth headers:
  - **OAuth (v3)**: `Authorization: Bearer …` + `IG-ACCOUNT-ID`.
  - **CST/XST (v1/v2)**: `CST` and `X-SECURITY-TOKEN`.
- Emits a `debug_span!("ig.http", ...)` with method/path/version. Token
  values are **never** included.
- Maps non-2xx responses to `Error::Api { status, source: ApiError }`,
  preserving IG's `errorCode`.
- Handles empty response bodies (e.g. `DELETE /session`) — returns
  `serde_json::Value::Null` deserialised into your `R`.

## What `Transport` does NOT do

- It does **not** auto-refresh expired OAuth tokens. The session
  `needs_refresh()` helper exists; if a domain method needs to refresh
  pre-emptively, do it explicitly. (Auto-refresh-on-401 is on the
  roadmap; don't reimplement it locally.)
- It does **not** retry on errors. IG's `fetch_deal_by_deal_reference`
  is a special case in the Python lib (5 retries with 1s back-off);
  implement it inline in that one endpoint, not at transport level.
- It does **not** rate-limit. Rate limiting is a separate concern (see
  the Python lib for two buckets — trading vs non-trading); we'll add
  it when needed.

## When you need to add a new mechanism to the transport

If you find yourself wanting to do anything fundamentally HTTP-shaped
in a domain module — custom retry, custom headers, content negotiation,
etc. — extend `Transport` instead. Keep domain code about *what*, not
*how*.
