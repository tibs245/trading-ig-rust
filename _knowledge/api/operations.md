# Operations (Application Management)

Three endpoints, v1, non-trading bucket. Manage your API keys and
their allowances.

## `GET /operations/application` (v1) — list

```rust
client.operations().applications().await? -> Vec<Application>

pub struct Application {
    pub name: String,
    pub api_key: String,
    pub status: ApplicationStatus,             // ENABLED | DISABLED | REVOKED
    pub allowance_account_overall: u32,
    pub allowance_account_trading: u32,
    pub allowance_application_overall: u32,
    pub concurrent_subscriptions_limit: u32,
    pub allow_equities: bool,
    pub allow_quote_orders: bool,
    pub created_date: NaiveDateTime,
}
```

Returns a JSON array directly (no envelope).

## `PUT /operations/application` (v1) — update

```rust
pub struct UpdateApplicationRequest {
    pub api_key: String,
    pub status: ApplicationStatus,
    pub allowance_account_overall: u32,
    pub allowance_account_trading: u32,
}

client.operations().update_application(req).await? -> Application
```

## `PUT /operations/application/disable` (v1) — disable

Disables the **current** API key. Re-enabling requires the IG web UI.
No body.

```rust
client.operations().disable_current_key().await? -> Application
```

## Fixtures to add

- `operations/applications_v1.json` (1–2 keys).
- `operations/disabled_v1.json`.

## Tests required

- List, update, disable: golden each.
- A 4xx with `error.public-api.failure.not.an.administrator`
  (operations endpoints often 403 for non-admin keys).

## Note

These endpoints are rarely called interactively. They're mainly for
automated key rotation. Keep the API surface small and mirror
Python's behaviour — no bells and whistles.
