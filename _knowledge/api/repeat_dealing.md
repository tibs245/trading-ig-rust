# Repeat dealing

One endpoint, v1.

## `GET /repeat-dealing-window` (v1)

Optional `epic` query parameter. Returns the time window during which
a recently traded instrument can be re-traded with the same dealing
characteristics.

```rust
client.repeat_dealing().window().await? -> Vec<RepeatDealingWindow>
client.repeat_dealing().window_for(&epic).await? -> Vec<RepeatDealingWindow>
```

Schema (best-effort — confirm against a real response):

```rust
pub struct RepeatDealingWindow {
    pub epic: Epic,
    pub valid_from: NaiveDateTime,
    pub valid_to: NaiveDateTime,
    // …other fields TBD on first real call
}
```

## Retry behaviour

Python retries up to 5× with 1-second back-off on non-200. Mirror it
**inline** in this method (don't generalise — same as
`fetch_deal_by_deal_reference`).

## Fixtures to add

- `repeat_dealing/window_v1.json`.

## Tests required

- Golden, both with and without `epic` param.
- Retry behaviour: 500 → 500 → 200 should succeed.
