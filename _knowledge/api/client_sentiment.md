# Client Sentiment

Two endpoints, v1, non-trading bucket.

## `GET /clientsentiment/{marketId}` (v1) — single

```rust
client.client_sentiment().get(&market_id).await? -> Sentiment

pub struct Sentiment {
    pub market_id: String,
    pub long_position_percentage: f64,
    pub short_position_percentage: f64,
}
```

## `GET /clientsentiment?marketIds=A,B,C` (v1) — bulk

Same path with a comma-separated query parameter. The Python lib
overloads the same method name; we expose two methods:

```rust
client.client_sentiment().get(&market_id).await? -> Sentiment
client.client_sentiment().get_many(&market_ids).await? -> Vec<Sentiment>
```

The bulk response is `{ clientSentiments: [...] }` — unwrap the
envelope.

## `GET /clientsentiment/related/{marketId}` (v1) — related markets

```rust
client.client_sentiment().related(&market_id).await? -> Vec<Sentiment>
```

Returns `{ clientSentiments: [...] }` — same shape as the bulk
response.

## Fixtures to add

- `client_sentiment/get_v1.json`.
- `client_sentiment/get_many_v1.json` (2–3 markets).
- `client_sentiment/related_v1.json`.

## Tests required

- Single + bulk + related: golden each.
- Validation: ensure `long + short ≈ 100.0` in fixtures (sanity).
- 404 when the market id doesn't exist.

## Note

`marketId` is **not** an `Epic` — it's a different identifier (e.g.
`"CC.D.LCO.UNC.IP"` for an instrument's umbrella id). Don't reuse
`Epic` here. Define a `MarketId(String)` newtype if it's worth typing
this distinction; otherwise use `String`.
