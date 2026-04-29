# Dealing — Positions (OTC)

Six endpoints. `open` is the most complex — use a **type-state builder**
(see `dto_conventions.md`).

## `GET /positions` — list (v1 + v2)

Two versions, two response shapes. Implement **both**.

```rust
client.dealing().positions().list_v1().await? -> Vec<PositionV1>
client.dealing().positions().list_v2().await? -> Vec<PositionV2>
client.dealing().positions().list().await?    // alias for list_v2
```

Both return `{ positions: [...] }` envelopes. The Rust list method
unwraps the envelope and returns the `Vec` directly.

**v2 entry shape:**
```json
{
  "position": {
    "contractSize": 1, "controlledRisk": false, "createdDate": "...",
    "createdDateUTC": "2024-01-01T09:30:00", "dealId": "DIAAA...",
    "dealReference": "ref-123", "direction": "BUY", "level": 1.2345,
    "limitLevel": null, "size": 1.0, "stopLevel": null,
    "trailingStep": null, "trailingStopDistance": null
  },
  "market": {
    "bid": 1.234, "offer": 1.235, "epic": "CS.D.GBPUSD.TODAY.IP",
    "expiry": "DFB", "instrumentName": "...", "instrumentType": "CURRENCIES",
    "lotSize": 1, "marketStatus": "TRADEABLE", "scalingFactor": 10000,
    "updateTime": "..."
  }
}
```

Flatten to:
```rust
pub struct PositionV2 {
    pub deal_id: DealId,
    pub deal_reference: DealReference,
    pub direction: Direction,
    pub size: f64,
    pub level: f64,
    pub limit_level: Option<f64>,
    pub stop_level: Option<f64>,
    pub controlled_risk: bool,
    pub contract_size: u32,
    pub created_date: NaiveDateTime,
    pub created_date_utc: Option<NaiveDateTime>,
    pub trailing_step: Option<f64>,
    pub trailing_stop_distance: Option<f64>,
    pub market: MarketSnapshot,    // keep as nested struct
}
```

`PositionV1` lacks `createdDateUTC` and uses the v1 date format; model
the differences explicitly.

## `GET /positions/{dealId}` (v2) — single

```rust
client.dealing().positions().get(&deal_id).await? -> PositionV2
```

Same shape as one v2 entry above.

## `POST /positions/otc` (v2) — open

**Use a type-state builder.** Mandatory fields:

- `currency_code: Currency`
- `direction: Direction`
- `epic: Epic`
- `expiry: String` (e.g. `"DFB"`, `"-"`, expiry date)
- `guaranteed_stop: bool`
- `order_type: OrderType` (`LIMIT`, `MARKET`, `QUOTE`)
- `size: f64`
- `force_open: bool` (defaults to `false`)

Optional:

- `level: Option<f64>` (required if `order_type` is `LIMIT` or `QUOTE`)
- `limit_distance` / `limit_level` (mutually exclusive)
- `stop_distance` / `stop_level` (mutually exclusive)
- `trailing_stop: Option<bool>`, `trailing_stop_increment`
- `time_in_force: Option<TimeInForce>`
- `quote_id: Option<String>` (for `QUOTE` order type)

Returns `{ dealReference }`. The Python lib auto-fetches the deal
confirmation via `GET /confirms/{ref}`. **Do the same** —
`open().send().await?` should return a `DealConfirmation`, not just a
reference. Document the fetch.

```rust
client.dealing().positions().open()
    .epic(epic).direction(Direction::Buy).size(1.0)
    .order_type(OrderType::Market).currency("EUR").expiry("DFB")
    .with_stop_distance(20.0)
    .send().await? -> DealConfirmation
```

## `PUT /positions/otc/{dealId}` (v2) — update

All fields optional except `guaranteed_stop`:

```rust
pub struct UpdatePositionRequest {
    pub guaranteed_stop: bool,
    pub limit_level: Option<f64>,
    pub stop_level: Option<f64>,
    pub trailing_stop: Option<bool>,
    pub trailing_stop_distance: Option<f64>,
    pub trailing_stop_increment: Option<f64>,
}

client.dealing().positions().update(&deal_id, req).await? -> DealConfirmation
```

Use the `Request` struct pattern (not a builder) — the optional
combinations are simpler here.

## `DELETE /positions/otc` (v1) — close

Body fields: `dealId, direction (opposite of open), epic, expiry,
level, orderType, quoteId?, size, timeInForce?`.

DELETE-with-body — `reqwest` supports it natively, no `_method=POST`
hack needed (Python uses one because `requests` doesn't support
DELETE+body cleanly).

```rust
client.dealing().positions().close(req).await? -> DealConfirmation
```

`req` is a `ClosePositionRequest` struct (~9 fields, no builder
needed).

## `GET /confirms/{dealReference}` (v1) — confirmation

Returns:
```rust
pub struct DealConfirmation {
    pub deal_reference: DealReference,
    pub deal_status: DealStatus,        // ACCEPTED | REJECTED | UNKNOWN
    pub reason: Option<String>,         // populated on REJECTED
    pub direction: Direction,
    pub epic: Epic,
    pub expiry: String,
    pub level: Option<f64>,
    pub limit_level: Option<f64>,
    pub stop_level: Option<f64>,
    pub order_type: OrderType,
    pub size: f64,
    pub timestamp: NaiveDateTime,
}
```

Python retries up to 5× with 1s back-off if non-200. Mirror this — the
confirmation is sometimes available after a small delay. Implement the
retry **inline** in this one method (do not generalise).

```rust
client.dealing().positions().confirm(&deal_ref).await? -> DealConfirmation
```

## Fixtures to add

- `dealing/positions/list_v1.json`, `list_v2.json`,
  `list_v2_empty.json`.
- `dealing/positions/get_v2.json`.
- `dealing/positions/open_v2.json` (response: `{ dealReference }`).
- `dealing/confirms/accepted_v1.json`, `rejected_v1.json`.

## Tests required

- List v1 + list v2 each cover golden + empty + an error.
- Open: golden path (open → confirm = ACCEPTED) and a rejected case.
- Update / close: golden path each.
- Confirm: golden + at least one retry-then-200 case.
