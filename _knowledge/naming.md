# Naming conventions

## Types

- PascalCase: `OpenPositionRequest`, `MarketDetails`, `PriceSnapshot`.
- Suffix conventions:
  - `…Request` for endpoint inputs.
  - `…Response` only when the response is *not* a domain entity (e.g.
    `OpenPositionResponse { deal_reference }`). When it *is* a domain
    entity, use the entity name (`Account`, `Position`, `Market`).
  - `…Builder` for type-state builders.
  - `…Api` for the typed domain accessors (`AccountsApi`, `MarketsApi`).

## Methods on the typed APIs

- Short verbs: `list`, `get`, `search`, `open`, `close`, `update`,
  `delete`, `create`.
- When several versions of the same endpoint must coexist, suffix with
  the version number: `list_v1`, `list_v2`. The unsuffixed method is the
  alias of the canonical (highest) version.

## Modules

- Singular for a single concept: `session`, `time`, `error`.
- Plural for groupings: `accounts`, `markets`, `watchlists`.
- Nested domains live under their parent: `dealing/positions/`,
  `dealing/working_orders/`. The accessor chain mirrors that nesting:
  `client.dealing().positions().open()`.

## Field names (Rust ↔ IG JSON)

- snake_case in Rust.
- Map to IG's camelCase using `#[serde(rename_all = "camelCase")]` at
  the struct level when the whole struct is camelCase.
- Use per-field `#[serde(rename = "…")]` only for irregular cases
  (IG sometimes mixes camelCase and snake_case in the same payload —
  see the v3 OAuth payload).
- Booleans: prefer `is_…` only when needed for disambiguation (Rust
  convention is bare nouns for fields: `pub trailing_stop: bool`).

## Enum variants

- PascalCase variants serialised as IG strings via `#[serde(rename_all
  = "SCREAMING_SNAKE_CASE")]` for IG-style enums (`BUY`, `SELL`,
  `LIMIT`, `MARKET`).
- Use `#[serde(rename_all = "UPPERCASE")]` for short IG codes (e.g.
  `Direction { Buy, Sell }` → `"BUY"`, `"SELL"`).

## Public re-exports

- Re-export the top-level types from `lib.rs` so that
  `use trading_ig::{IgClient, Error, Direction}` is enough for common
  code. Don't re-export deep types — make callers reach for them via
  their module path (e.g. `trading_ig::accounts::Account`).
