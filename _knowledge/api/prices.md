# Prices (Historical)

Three **distinct** endpoints — v1, v2, v3 — not just versions of one
endpoint. Implement all three with named methods.

## `GET /prices/{epic}` (v3) — flexible query

The modern, recommended endpoint. Query parameters:

| Param        | Type        | Required | Notes                        |
| ------------ | ----------- | -------- | ---------------------------- |
| `resolution` | `Resolution`| no       | Defaults to `MINUTE` server-side |
| `from`       | datetime    | no       | ISO `YYYY-MM-DDTHH:MM:SS`    |
| `to`         | datetime    | no       | ISO                          |
| `max`        | u32         | no       | Max number of points         |
| `pageSize`   | u32         | no       | Default 20                   |
| `pageNumber` | u32         | no       | For pagination               |

Use the **Request struct + Default** pattern:

```rust
pub struct HistoricalPricesRequest {
    pub resolution: Option<Resolution>,
    pub from: Option<NaiveDateTime>,
    pub to: Option<NaiveDateTime>,
    pub max: Option<u32>,
    pub page_size: Option<u32>,
    pub page_number: Option<u32>,
}
impl Default for HistoricalPricesRequest { /* all None */ }

client.prices().history_v3(&epic, req).await? -> HistoricalPrices
```

Response:
```rust
pub struct HistoricalPrices {
    pub prices: Vec<PricePoint>,
    pub instrument_type: InstrumentType,
    pub metadata: PricesMetadata,
}

pub struct PricePoint {
    pub snapshot_time: String,           // raw IG string (v2 format)
    pub snapshot_time_utc: NaiveDateTime,// ISO, v3 only
    pub open_price: PriceCandle,
    pub close_price: PriceCandle,
    pub high_price: PriceCandle,
    pub low_price: PriceCandle,
    pub last_traded_volume: Option<u64>,
}

pub struct PriceCandle {
    pub bid: Option<f64>,
    pub ask: Option<f64>,
    pub last_traded: Option<f64>,
}

pub struct PricesMetadata {
    pub page_data: PageData,
    pub allowance: PriceAllowance,
}

pub struct PageData { page_number: u32, page_size: u32, total_pages: u32 }
pub struct PriceAllowance { remaining_allowance: u32, total_allowance: u32, allowance_expiry: u64 }
```

`Resolution` enum (matches IG's strings):
```
SECOND, MINUTE, MINUTE_2, MINUTE_3, MINUTE_5, MINUTE_10, MINUTE_15,
MINUTE_30, HOUR, HOUR_2, HOUR_3, HOUR_4, DAY, WEEK, MONTH
```

### Auto-pagination

The Python lib auto-paginates when `pageNumber` is unset. **Provide a
helper** alongside the basic call:

```rust
// Returns just one page (the one the request asked for, default page 1).
client.prices().history_v3(&epic, req).await?

// Returns all pages, fetched sequentially with a small delay.
client.prices().history_v3_all(&epic, req).await? -> HistoricalPrices
```

The `_all` variant should respect a 1-second delay between pages
(Python default) and stop when `pageNumber == totalPages`.

Don't auto-paginate the basic call — surprising side effects.

## `GET /prices/{epic}/{resolution}/{numpoints}` (v2) — fixed N points

```rust
client.prices().history_by_num_points_v2(
    &epic, Resolution::Hour, 100
).await? -> HistoricalPrices
```

Same response shape as v3 (the `metadata.allowance` block exists in
both).

## `GET /prices/{epic}/{resolution}/{startDate}/{endDate}` (v2) — date range

```rust
client.prices().history_by_date_range_v2(
    &epic, Resolution::Hour, start, end
).await? -> HistoricalPrices
```

`start` / `end` are `NaiveDateTime`, formatted as
`YYYY/MM/DD HH:MM:SS` (v2 format) when building the URL path.

## `GET /prices/{epic}/{resolution}` (v1) — date range, query params

```rust
client.prices().history_by_date_range_v1(
    &epic, Resolution::Hour, start, end
).await? -> HistoricalPricesV1   // possibly different shape — check on impl
```

Query params `startdate` / `enddate` (lowercase, **not** camelCase)
formatted as `YYYY:MM:DD-HH:MM:SS` (v1 format). Use
`time::format(_, V1)`.

## Allowance logging

The Python lib logs the `remainingAllowance` at INFO. **Mirror this**
— emit a `tracing::info!` with `remaining_allowance` and
`allowance_expiry` after each successful price call. Helps users avoid
surprise rate-limit hits (price quotas are tight).

## Fixtures to add

- `prices/history_v3_basic.json` (a few minutes of bars).
- `prices/history_v3_paged_p1.json`, `..._p2.json` (multi-page).
- `prices/history_num_points_v2.json`.
- `prices/history_date_range_v2.json`.
- `prices/history_date_range_v1.json`.

## Tests required

- `history_v3` golden + pagination (mount p1+p2, assert `_all` returns
  combined `Vec<PricePoint>`).
- `history_by_num_points_v2` golden + a 0-points response.
- Both `history_by_date_range_*` versions.
- A 4xx with `error.public-api.exceeded-account-historical-data-allowance`
  → `Error::Api`.
