# Watchlists

Six endpoints, all v1, all non-trading rate bucket.

## `GET /watchlists` (v1) — list all

```rust
client.watchlists().list().await? -> Vec<WatchlistSummary>

pub struct WatchlistSummary {
    pub id: String,
    pub name: String,
    pub editable: bool,           // false for system-defined watchlists
    pub deleteable: bool,
    pub default_system_watchlist: bool,
}
```

Response envelope: `{ watchlists: [...] }`.

## `POST /watchlists` (v1) — create

Body: `{ name: String, epics: [Epic, ...] }`.

```rust
pub struct CreateWatchlistRequest {
    pub name: String,
    pub epics: Vec<Epic>,
}

client.watchlists().create(req).await? -> CreateWatchlistResponse

pub struct CreateWatchlistResponse {
    pub watchlist_id: String,             // wire: "watchlistId"
    pub status: CreateWatchlistStatus,    // SUCCESS | SUCCESS_NOT_ALL_INSTRUMENTS_ADDED
}
```

The status enum matters: a partial success (some epics rejected) is
still HTTP 200.

## `GET /watchlists/{id}` (v1) — list markets

```rust
client.watchlists().markets(&watchlist_id).await? -> Vec<MarketSummary>
```

Response envelope: `{ markets: [...] }`. Reuse `MarketSummary` from
`markets`.

## `PUT /watchlists/{id}` (v1) — add market

Body: `{ epic: Epic }`.

```rust
client.watchlists().add_market(&watchlist_id, &epic).await? -> AddMarketResponse

pub struct AddMarketResponse { pub status: String }
```

## `DELETE /watchlists/{id}/{epic}` (v1) — remove market

```rust
client.watchlists().remove_market(&watchlist_id, &epic).await? -> RemoveMarketResponse
```

## `DELETE /watchlists/{id}` (v1) — delete

```rust
client.watchlists().delete(&watchlist_id).await? -> DeleteWatchlistResponse
```

## Fixtures to add

- `watchlists/list_v1.json` (mix of editable + system).
- `watchlists/markets_v1.json`.
- `watchlists/create_v1.json` (status SUCCESS).
- `watchlists/create_partial_v1.json`
  (status SUCCESS_NOT_ALL_INSTRUMENTS_ADDED).

## Tests required

- List + markets: golden.
- Create: SUCCESS + partial-success.
- Add / remove / delete: golden each.
- A 4xx error with `error.invalid.watchlist.name.too.long`.

## Notes

- URL-encode `epic` values when building paths (they contain dots).
  Prefer `format!("watchlists/{watchlist_id}/{epic}")` and let the
  reqwest URL parser handle escaping.
- System watchlists (`editable: false`) cannot be modified — let IG
  return the error rather than client-side validation.
