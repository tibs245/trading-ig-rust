# Accounts

Three endpoints, all v1, all non-trading rate bucket.

## `GET /accounts` (v1) — list

Returns `{ accounts: [Account, ...] }`.

`Account` shape:

```json
{
  "accountId": "ABC123",
  "accountAlias": "...",
  "accountType": "CFD",            // or "PHYSICAL", "SPREADBET"
  "accountName": "Demo CFD",
  "canTransferToMA": true,
  "canTransferFromMA": true,
  "defaultAccount": false,
  "preferred": true,
  "balance": {
    "balance": 10000.0,
    "deposit": 0.0,
    "profitLoss": 0.0,
    "availableCash": 10000.0
  }
}
```

Map to:

```rust
pub struct Account {
    pub account_id: String,
    pub account_alias: Option<String>,
    pub account_type: AccountType,        // CFD | PHYSICAL | SPREADBET
    pub account_name: String,
    pub can_transfer_to_ma: bool,
    pub can_transfer_from_ma: bool,
    pub default_account: bool,
    pub preferred: bool,
    pub balance: AccountBalance,
}

pub struct AccountBalance {
    pub balance: f64,
    pub deposit: f64,
    pub profit_loss: f64,
    pub available_cash: f64,
}
```

`AccountType` is a small enum with `#[serde(rename_all = "UPPERCASE")]`.

Method:

```rust
client.accounts().list().await? -> Vec<Account>
```

## `GET /accounts/preferences` (v1) — read

Returns at minimum `{ trailingStopsEnabled: bool }`. May contain more
fields in the future — model with `#[serde(default)]` for forward
compat.

```rust
pub struct AccountPreferences {
    pub trailing_stops_enabled: bool,
}

client.accounts().preferences().await? -> AccountPreferences
```

## `PUT /accounts/preferences` (v1) — update

Body: `{ trailingStopsEnabled: "true" | "false" }`. **String, not bool**
— IG quirk. Wrap this so the caller passes a `bool`:

```rust
client.accounts().update_preferences(UpdatePreferences {
    trailing_stops_enabled: true,
}).await? -> AccountPreferences
```

Internally serialise the bool as the IG-expected string.

## Errors to test

- 401 with `error.security.invalid-details` (no session) →
  `Error::Api`.
- (Preferences endpoints have no special error semantics worth
  modelling.)

## Fixtures to add

- `accounts/list_v1.json` — 1–2 accounts with full balance.
- `accounts/preferences_v1.json` — `trailing_stops_enabled: true`.
