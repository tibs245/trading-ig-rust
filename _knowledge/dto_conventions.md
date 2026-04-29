# DTO conventions

The Python library uses dicts and `**kwargs` everywhere. In Rust we
expose typed DTOs — but we choose **the right shape for each endpoint's
shape**, rather than applying one pattern uniformly.

## Three request shapes

### 1. Direct method arguments — **trivial endpoints**

For endpoints with **0 to 2 mandatory parameters and no optional ones**.

```rust
client.accounts().list().await?;
client.markets().get(&epic).await?;
client.watchlists().delete(&watchlist_id).await?;
client.session().logout().await?;
```

No struct overhead, the call site reads naturally.

### 2. Public `Request` struct + `Default` — **many optional params**

For endpoints where the caller picks among many optional fields and
none of the optional combinations are illegal.

```rust
pub struct HistoricalPricesRequest {
    pub resolution: Option<Resolution>,
    pub from: Option<NaiveDateTime>,
    pub to: Option<NaiveDateTime>,
    pub max: Option<u32>,
    pub page_size: Option<u32>,
}

impl Default for HistoricalPricesRequest { /* … */ }

let req = HistoricalPricesRequest {
    resolution: Some(Resolution::Hour),
    from: Some(start),
    ..Default::default()
};
let prices = client.prices().history_v3(&epic, req).await?;
```

Mandatory params (the `epic` here) stay as method arguments — they're
not in the struct.

### 3. Type-state builder — **complex create/open endpoints**

For endpoints with **≥3 mandatory parameters AND ≥3 optional ones**, or
where some field combinations are illegal at the API level. Use a
type-state builder so missing mandatory fields are **compile errors**,
not runtime errors.

This applies to:

- `dealing.positions.open` — 16 fields, complex stop/limit rules.
- `dealing.working_orders.create` — similar.
- (Add others here if you discover the shape warrants it.)

Sketch:

```rust
client.dealing().positions().open()
    .epic(epic).direction(Direction::Buy).size(1.0)
    .order_type(OrderType::Market).currency("EUR").expiry("DFB")
    .with_stop_distance(20.0)              // optional
    .with_limit_distance(10.0)             // optional
    .send().await?;                        // <- only callable when
                                           //    all mandatory fields set
```

The builder lives next to the endpoint method (`open_position.rs`),
not in `models.rs`.

#### Type-state sketch

```rust
pub struct OpenPositionBuilder<E, D, S, O, C, X> { /* PhantomData markers */ }

// Marker types
pub struct Missing;
pub struct Set<T>(T);

impl<...> OpenPositionBuilder<Missing, ...> {
    pub fn epic(self, e: Epic) -> OpenPositionBuilder<Set<Epic>, ...> { ... }
}
// `send` is implemented only on the fully-specified type alias.
```

A single-field-at-a-time type-state explosion is overkill — group
mandatories: `WithMandatories` vs `MissingMandatory` is enough in
practice.

## Response types

- **Public structs** in `<domain>/models.rs`, derive
  `Debug, Clone, Deserialize, Serialize`.
- **Flatten IG envelopes** when the nested form hurts ergonomics.
  IG often returns:
  ```json
  { "position": { "dealId": "...", "size": 1 },
    "market":   { "epic": "...", "bid": 1.2 } }
  ```
  Expose:
  ```rust
  pub struct Position {
      pub deal_id: DealId,
      pub size: f64,
      pub market: MarketSnapshot,   // keep the inner struct for the market subfields
  }
  ```
  Use `#[serde(flatten)]` and intermediate private structs as needed.

- If you flatten or rename, **provide an `into_raw()` method** that
  returns the close-to-wire shape. Some users will want exact API
  parity for diffing.

## Forbidden

- Don't accept `&str` where a typed newtype exists (`Epic`, `DealId`,
  `Currency`). `impl Into<Epic>` is fine if convenient.
- Don't return `serde_json::Value` from a public method.
- Don't re-export `reqwest` types in public APIs.
- Don't expose a builder for a 2-field endpoint. Don't expose a struct
  for a 1-field endpoint. Match the shape to the complexity.
