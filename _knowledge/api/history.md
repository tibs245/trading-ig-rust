# History — Activity & Transactions

Five endpoints: three for activity (v1 + v3), two for transactions
(v1 + v2). All non-trading bucket.

## Activity

### `GET /history/activity` (v3) — paginated, filterable

The modern endpoint. Query params:

| Param      | Type   | Notes                                          |
| ---------- | ------ | ---------------------------------------------- |
| `from`     | ISO    | `YYYY-MM-DDTHH:MM:SS`                          |
| `to`       | ISO    | `YYYY-MM-DDTHH:MM:SS`                          |
| `detailed` | bool   | Expand activity sub-objects                    |
| `dealId`   | string | Filter by deal id                              |
| `filter`   | FIQL   | `type==POSITION;status==ACCEPTED` etc.         |
| `pageSize` | u32    | 10–500, default 50                             |

Use **Request struct + Default**:

```rust
pub struct ActivityRequest {
    pub from: Option<NaiveDateTime>,
    pub to: Option<NaiveDateTime>,
    pub detailed: bool,                  // default false
    pub deal_id: Option<DealId>,
    pub filter: Option<String>,          // raw FIQL string
    pub page_size: u32,                  // default 50
}

client.history().activity_v3(req).await? -> Vec<Activity>
```

Pagination is via `metadata.paging.next` URL — **always auto-follow**.
Unlike prices, the Python lib auto-paginates this one by default; we
do too. The next URL contains all needed query params; just GET it.

`Activity` shape:
```rust
pub struct Activity {
    pub date: NaiveDateTime,             // ISO
    pub epic: Epic,
    pub period: String,
    pub deal_id: DealId,
    pub channel: ActivityChannel,        // WEB | MOBILE | …
    pub activity_type: ActivityType,     // POSITION | WORKING_ORDER | …
    pub status: ActivityStatus,          // ACCEPTED | REJECTED | UNKNOWN
    pub description: String,
    pub details: Option<ActivityDetails>,// only when `detailed=true`
}
```

### `GET /history/activity/{milliseconds}` (v1) — period

Period in **milliseconds from now** (so `86400000` = last 24h).

```rust
client.history().activity_by_period_v1(milliseconds).await? -> Vec<ActivityV1>
```

### `GET /history/activity/{fromDate}/{toDate}` (v1) — date range

Dates in v1 format: `YYYY:MM:DD-HH:MM:SS`.

```rust
client.history().activity_by_date_range_v1(from, to).await? -> Vec<ActivityV1>
```

`ActivityV1` likely differs from `Activity` — model on inspection of
fixtures. The Python lib treats them as separate.

## Transactions

### `GET /history/transactions` (v2) — paginated

```rust
pub struct TransactionsRequest {
    pub trans_type: Option<TransactionType>,    // ALL | DEPOSIT | WITHDRAWAL | …
    pub from: Option<NaiveDateTime>,            // ISO
    pub to: Option<NaiveDateTime>,
    pub max_span_seconds: Option<u32>,
    pub page_size: Option<u32>,
    pub page_number: Option<u32>,
}

client.history().transactions_v2(req).await? -> TransactionsResponse
```

Paginated via `metadata.pageData` (page number-based, like prices).

`Transaction` shape (per IG docs):
```rust
pub struct Transaction {
    pub date: String,                 // various formats — keep as String
    pub date_utc: NaiveDateTime,      // ISO
    pub instrument_name: String,
    pub period: String,
    pub profit_and_loss: String,      // includes currency symbol — keep as String
    pub transaction_type: String,
    pub reference: String,
    pub open_level: String,
    pub close_level: String,
    pub size: String,                 // signed string with currency symbol
    pub currency: Currency,
    pub cash_transaction: bool,
}
```

Yes, IG returns most numeric fields as strings here. **Don't fight it**
— keep them as `String` and provide helpers like
`tx.profit_and_loss_value() -> Option<f64>` if useful.

### `GET /history/transactions/{type}/{milliseconds}` (v1) — period

Type values: `ALL`, `ALL_DEAL`, `DEPOSIT`, `WITHDRAWAL`. Period in
ms from now.

```rust
client.history().transactions_by_period_v1(trans_type, milliseconds)
    .await? -> Vec<Transaction>
```

## FIQL filter (activity v3)

IG documents these operators:

- `==` equals
- `!=` not equals
- `,` OR
- `;` AND

Example: `type==POSITION;status==ACCEPTED`. We pass it through as a
raw `String` for now; a typed FIQL builder is overkill at this stage.

## Fixtures to add

- `history/activity_v3_p1.json`, `..._p2.json` (multi-page).
- `history/activity_v1_period.json`.
- `history/transactions_v2.json`.
- `history/transactions_v1_period.json`.

## Tests required

- Activity v3: golden + auto-pagination across 2 pages.
- Activity v1 (both): golden.
- Transactions v2: golden + filter.
- Transactions v1 period: golden.
