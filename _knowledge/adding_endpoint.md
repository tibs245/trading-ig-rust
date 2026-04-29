# Adding a new endpoint — checklist

**Read this before writing code.** Same flow whether you're porting one
endpoint or all endpoints in a domain.

## 1. Locate the spec

Open the matching `_knowledge/api/<domain>.md`. Find the endpoint:
HTTP verb, path, version header, request fields, response shape,
quirks.

If the endpoint isn't in the catalog, stop and ask — we may have missed
it on extraction.

## 2. Pick the request shape

Apply [`dto_conventions.md`](dto_conventions.md):

- 0–2 mandatory, 0 optional → method args.
- Many optional → `Request` struct + `Default`.
- ≥3 mandatory + ≥3 optional → type-state builder.

## 3. Add types

- Request type → `<domain>/models.rs` (or method args).
- Response type → `<domain>/models.rs`. Derive
  `Debug, Clone, Deserialize, Serialize`. Flatten the IG envelope if
  it improves ergonomics; provide `into_raw()` if you do.
- Newtypes used by 2+ domains → `src/models/common.rs`.

## 4. Add the endpoint method

In `<domain>/api.rs`, on the typed domain accessor:

```rust
impl<'a> AccountsApi<'a> {
    #[tracing::instrument(skip_all)]
    pub async fn list(&self) -> Result<Vec<Account>> {
        #[derive(Deserialize)]
        struct Envelope { accounts: Vec<Account> }
        let body: Envelope = self.client.transport
            .request(Method::GET, "accounts", Some(1), None::<&()>, &self.client.session)
            .await?;
        Ok(body.accounts)
    }
}
```

Conventions:

- Always go through `Transport::request`. Never use `reqwest` directly.
- `version` argument matches the IG `Version` header value.
- For endpoints that return an envelope around the data
  (`{ "accounts": [...] }`), define a private `struct Envelope` inside
  the method and unwrap it.
- Use `#[tracing::instrument]` with a span name matching the call site
  (`accounts.list`, `dealing.positions.open`). See
  [`tracing.md`](tracing.md) for fields to include.

## 5. Wire the accessor on `IgClient`

Add (once per domain):

```rust
impl IgClient {
    pub fn accounts(&self) -> AccountsApi<'_> { AccountsApi { client: self } }
}
```

For nested domains (`dealing/positions/`), add the accessor on the
parent domain.

## 6. Write fixtures

- Drop one or more JSON files under
  `tests/fixtures/json/<domain>/<scenario>.json`.
- Use **anonymised** values resembling real IG payloads (account ids
  like `ABC123`, epics like `CS.D.GBPUSD.TODAY.IP`).
- Cover at least: golden path, optional fields missing, nullable
  fields present.

## 7. Write integration tests

Create `tests/<domain>.rs` (one file per domain). Use the helpers in
`tests/support/`. See [`testing.md`](testing.md). Each domain test
file should cover, at minimum:

- The golden path (200 + valid body).
- A 4xx error with an IG `errorCode` mapped to `Error::Api`.
- One schema edge case (nullable, optional, version variant).

## 8. Verify

```bash
cargo test --all-targets
cargo clippy --all-targets --no-deps
```

Both must come back clean before you mark the task complete.

## 9. Doc comment

Every public type and method needs a doc comment. Include:

- One-line summary.
- A `# Errors` section if the method can fail in a non-obvious way.
- Link to the related entry in the IG Labs REST reference if relevant.
- For multi-version methods, document the chosen version and what the
  unsuffixed alias resolves to.
