# API versions

## Policy

We expose **every version of every endpoint** that the Python
`trading-ig` library exposes — no more, no less. The Rust crate is meant
to be a true 1:1 alternative.

## How to model multiple versions

When an endpoint has multiple versions with **different response
schemas**, expose them as **separate methods with explicit names**:

```rust
client.dealing().positions().list_v1().await?;   // → PositionsV1
client.dealing().positions().list_v2().await?;   // → PositionsV2
```

The method **without a `_vN` suffix** aliases the highest version (this
matches the Python default `version="2"` argument behaviour) and is
documented as such:

```rust
/// Alias for [`Self::list_v2`] — the canonical recent version of this endpoint.
pub async fn list(&self) -> Result<PositionsV2> { self.list_v2().await }
```

For endpoints that are *distinct paths* per version (e.g. historical
prices), expose them as named methods that document their version:

```rust
client.prices().history_by_epic_v3(epic, opts).await?;
client.prices().history_by_num_points_v2(epic, res, n).await?;
client.prices().history_by_date_range_v1(...).await?;
client.prices().history_by_date_range_v2(...).await?;
```

## Versions to cover (per the Python source)

| Domain               | Versions to implement                                       |
| -------------------- | ----------------------------------------------------------- |
| Session              | v2 (CST/XST headers) **and** v3 (OAuth) + refresh-token v1  |
| Positions list       | v1 + v2 (alias = v2)                                        |
| Position by deal id  | v2                                                          |
| Open / update        | v2                                                          |
| Close                | v1                                                          |
| Working orders list  | v1 + v2 (alias = v2)                                        |
| Working orders CUD   | v2                                                          |
| Markets search       | v1                                                          |
| Markets bulk fetch   | v2                                                          |
| Market by epic       | v3                                                          |
| Historical prices    | v1, v2, v3 (three distinct endpoints)                       |
| Activity history     | v1 (period + date range) + v3 (paginated)                   |
| Transactions         | v1 (type+period) + v2 (paginated)                           |
| Watchlists           | v1 (single version)                                         |
| Client sentiment     | v1 (single version)                                         |
| Accounts             | v1 (single version)                                         |
| Operations           | v1 (single version)                                         |
| Repeat dealing       | v1 (single version)                                         |

## Why not a `version: u8` argument?

Two reasons:

1. The response schemas are different per version — a single method
   would need a sum type or `serde_json::Value`. Both leak into the
   public API.
2. Method names are searchable and IDE-discoverable. `list_v2` is
   self-documenting; `list(2)` is not.
