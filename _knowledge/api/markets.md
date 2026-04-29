# Markets & Navigation

Five endpoints across versions v1 / v2 / v3.

## `GET /markets?searchTerm=…` (v1) — search

```rust
client.markets().search("EUR/USD").await? -> Vec<MarketSummary>
```

Returns `{ markets: [...] }` envelope.

`MarketSummary` (lightweight):
```rust
pub struct MarketSummary {
    pub epic: Epic,
    pub instrument_name: String,
    pub instrument_type: InstrumentType,    // CURRENCIES | SHARES | INDICES | …
    pub expiry: String,
    pub bid: Option<f64>,
    pub offer: Option<f64>,
    pub market_status: MarketStatus,        // TRADEABLE | EDITS_ONLY | …
    pub streaming_prices_available: bool,
}
```

## `GET /markets?epics=A,B,C&filter=ALL|SNAPSHOT_ONLY` (v2) — bulk

Comma-separated `epics` query (≤50 per call per IG). `filter` is
`ALL` (default, full details) or `SNAPSHOT_ONLY` (just snapshot).

```rust
client.markets().get_many(epics, MarketDetailFilter::All).await?
    -> Vec<MarketDetails>
```

Returns `{ marketDetails: [MarketDetails, ...] }`. Each entry has
sub-objects `instrument`, `dealingRules`, `snapshot`. Don't flatten —
the sub-objects each have ≥10 fields and are conceptually distinct.

## `GET /markets/{epic}` (v3) — single

```rust
client.markets().get(&epic).await? -> MarketDetails
```

Same shape as one entry of the v2 bulk fetch.

## `GET /marketnavigation` (v1) — top-level

```rust
client.markets().navigation().await? -> NavigationNode
```

Returns `{ markets: [MarketSummary]?, nodes: [{ id, name }]? }`.
Both fields are optional (top-level node only has children, leaf
nodes have markets but no children). Model as:

```rust
pub struct NavigationNode {
    pub markets: Vec<MarketSummary>,
    pub nodes: Vec<NavigationChild>,
}

pub struct NavigationChild {
    pub id: String,
    pub name: String,
}
```

Use `#[serde(default)]` on both list fields.

## `GET /marketnavigation/{node}` (v1) — sub-node

```rust
client.markets().navigation_node(&node_id).await? -> NavigationNode
```

Same shape as the top-level call.

## Shared types worth defining once

- `InstrumentType` — enum, `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]`.
- `MarketStatus` — enum, same.
- `MarketSnapshot` — used here AND in positions / working orders. Put
  it in a shared place: either `src/models/common.rs` or
  `src/markets/models.rs` (and re-export from `crate::models`).

## Fixtures to add

- `markets/search_eurusd_v1.json`.
- `markets/get_many_v2.json` (2–3 epics).
- `markets/get_v3.json` (full details for one epic).
- `markets/navigation_root_v1.json` (only `nodes`, no `markets`).
- `markets/navigation_leaf_v1.json` (only `markets`).

## Tests required

- Search: golden + empty result.
- Bulk: golden with multiple epics, an empty list, and the
  `SNAPSHOT_ONLY` filter case.
- Single: golden + a 404 with `error.public-api.failure.market.not-found`.
- Navigation: top-level + leaf.
