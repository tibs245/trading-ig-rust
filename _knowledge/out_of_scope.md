# Out of scope

Things this crate **does not** do, by design.

## No sync API

Async-only on `tokio`. The Python lib has both `IGService` (sync) and
`IGStreamService` (async-ish via threads); we don't mirror the sync
side. Users who need sync can `block_on` themselves.

## No pandas / DataFrame conversions in core

Python's `IGService` returns `pandas.DataFrame` for many list
endpoints. We return `Vec<TypedStruct>`. A `polars` feature may be
added later to convert results into `polars::DataFrame`, but it is not
in the initial scope. **Don't preempt it** — no `#[cfg(feature =
"polars")]` blocks until we explicitly add the feature.

## No automatic OAuth refresh on 401

`Transport` does not retry-with-refresh on 401. Callers can call
`session().refresh()` themselves, or check `tokens.needs_refresh(skew)`
before a sensitive call. Auto-refresh-on-401 is on the roadmap and
will be added centrally in `Transport`.

## No rate limiter

The Python lib implements a leaky-bucket limiter (two buckets:
trading vs non-trading). We don't yet. When we add it, it will live
under `src/rate_limit.rs` and be wired into `Transport`. Don't open-
code rate limiting in domain modules.

## No retry policy

Same logic. Retries are a transport-level concern. The one Python
exception (`fetch_deal_by_deal_reference` retries 5× with 1s back-off)
should be implemented inline in that one endpoint method, not as a
generic mechanism.

## No CLI / no daemon

This is a library. We don't ship a binary, a server, or a TUI.
