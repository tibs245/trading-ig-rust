# Dealing — Working Orders

Four endpoints. `create` is complex — use a **type-state builder**.

## `GET /workingorders` — list (v1 + v2)

Both versions, with different schemas. Implement both.

```rust
client.dealing().working_orders().list_v1().await? -> Vec<WorkingOrderV1>
client.dealing().working_orders().list_v2().await? -> Vec<WorkingOrderV2>
client.dealing().working_orders().list().await?    // alias for list_v2
```

Both responses are `{ workingOrders: [...] }` envelopes; each entry has
`workingOrderData` and `marketData` subfields.

**v2 `workingOrderData`:**
```json
{
  "createdDate": "...", "currencyCode": "EUR", "dealId": "DI...",
  "direction": "BUY", "dma": false, "epic": "CS.D.GBPUSD.TODAY.IP",
  "goodTillDate": null, "goodTillDateISO": null,
  "guaranteedStop": false, "limitDistance": 10.0, "orderLevel": 1.2345,
  "orderSize": 1.0, "orderType": "LIMIT", "stopDistance": 20.0,
  "timeInForce": "GOOD_TILL_CANCELLED"
}
```

`marketData` is identical to `MarketSnapshot` used elsewhere — reuse
the same struct.

Flatten to a single `WorkingOrderV2` with the order fields at top
level and `market: MarketSnapshot` nested.

`WorkingOrderV1` field set differs (uses `goodTill` instead of
`goodTillDate`/`goodTillDateISO`, has `requestType`, `contingentLimit`,
`trailingTriggerIncrement`, `contingentStop`, `controlledRisk`,
`trailingStopIncrement`, `trailingStopDistance`, `trailingTriggerDistance`,
`dma`).

## `POST /workingorders/otc` (v2) — create

**Use a type-state builder.**

Mandatory:
- `currency_code: Currency`
- `direction: Direction`
- `epic: Epic`
- `expiry: String`
- `guaranteed_stop: bool`
- `level: f64`
- `size: f64`
- `time_in_force: TimeInForce`
- `order_type: OrderType` (sent as `type` on the wire — JSON key is
  `type`, **not** `orderType`)

Optional:
- `force_open: bool` (default `false`)
- `good_till_date: Option<NaiveDateTime>` — formatted as
  `YYYY/MM/DD HH:MM:SS` (use `time::format(_, V2)`).
- `limit_distance` / `limit_level`
- `stop_distance` / `stop_level`
- `deal_reference: Option<DealReference>` — caller-supplied tracking
  reference.

Returns `{ dealReference }`. Like `open_position`, **auto-fetch the
confirmation** via `GET /confirms/{ref}` and return
`DealConfirmation`.

```rust
client.dealing().working_orders().create()
    .epic(epic).direction(Direction::Buy).size(1.0)
    .order_type(OrderType::Limit).currency("EUR")
    .expiry("DFB").level(1.2345).time_in_force(TimeInForce::GoodTillCancelled)
    .with_stop_distance(20.0)
    .send().await? -> DealConfirmation
```

Watch the JSON serialization: the `order_type` field maps to wire key
`type`. Use `#[serde(rename = "type")]` on the request struct.

## `PUT /workingorders/otc/{dealId}` (v2) — update

**All fields are required** in the IG request — no partials. Model as
a struct:

```rust
pub struct UpdateWorkingOrderRequest {
    pub good_till_date: Option<NaiveDateTime>,   // sent as null when None
    pub level: f64,
    pub limit_distance: Option<f64>,
    pub limit_level: Option<f64>,
    pub stop_distance: Option<f64>,
    pub stop_level: Option<f64>,
    pub guaranteed_stop: bool,
    pub time_in_force: TimeInForce,
    pub order_type: OrderType,                   // wire key: "type"
}

client.dealing().working_orders().update(&deal_id, req).await? -> DealConfirmation
```

## `DELETE /workingorders/otc/{dealId}` (v2) — delete

Empty body. Returns `{ dealReference }`. Auto-fetch confirmation.

```rust
client.dealing().working_orders().delete(&deal_id).await? -> DealConfirmation
```

## Fixtures to add

- `dealing/working_orders/list_v1.json`, `list_v2.json`,
  `list_v2_empty.json`.
- `dealing/working_orders/create_v2.json` (response: `{ dealReference }`).

## Tests required

- List v1 + v2 each: golden + empty.
- Create: golden + a `DealStatus::Rejected` confirmation case.
- Update: golden.
- Delete: golden.

## Reuse from positions module

Reuse `DealConfirmation`, `MarketSnapshot`, `Direction`, `OrderType`,
`TimeInForce`, `DealId`, `DealReference`, `Epic`, `Currency`. Don't
re-define them — import from their canonical home (`crate::models` or
`crate::dealing::positions::models`).
