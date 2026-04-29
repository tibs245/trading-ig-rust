# Knowledge index

This is the project's knowledge base. **Don't read every file** — each
entry below has a one-line summary so you can pick only what matches
your task. If you are an agent, your task brief lists the files you
need.

## Project conventions

| File | Summary |
| ---- | ------- |
| [`architecture.md`](architecture.md) | Crate layout, module hierarchy, the `IgClient` → typed domain-API pattern. |
| [`naming.md`](naming.md) | Type / method / module naming rules and serde renaming. |
| [`errors.md`](errors.md) | Single crate-level `Error` enum; how to surface IG `errorCode`s; no per-domain error types. |
| [`tracing.md`](tracing.md) | `tracing` setup, span naming, fields to include, redaction rules. |
| [`http_transport.md`](http_transport.md) | How domain modules call `Transport::request` — single chokepoint, no direct `reqwest`. |
| [`dto_conventions.md`](dto_conventions.md) | Three request shapes (direct args / `Request` struct / type-state builder). Response flattening rules. |
| [`api_versions.md`](api_versions.md) | Per-endpoint version coverage policy (we mirror what Python `trading-ig` exposes). |
| [`time_formats.md`](time_formats.md) | The three IG date formats and the `time` module helpers. |
| [`testing.md`](testing.md) | `IgMockServer`, fixture loader, matchers, what each integration test must cover. |
| [`adding_endpoint.md`](adding_endpoint.md) | Step-by-step checklist for porting a new endpoint. **Read this before writing code.** |
| [`out_of_scope.md`](out_of_scope.md) | What we explicitly do NOT port (sync wrapper, pandas, etc.). |

## API catalog (per domain)

Each file describes the IG endpoints in one functional domain — request
fields, response shape, version coverage, quirks. Read only the
domain(s) you are implementing.

| File | Summary |
| ---- | ------- |
| [`api/_overview.md`](api/_overview.md) | Base URLs, mandatory headers, error payload shape, rate-limit buckets, pagination patterns. **Always read first.** |
| [`api/session.md`](api/session.md) | Login (v2 CST/XST + v3 OAuth), refresh, switch account, logout, encryption key. **Already implemented in Vague 0.** |
| [`api/accounts.md`](api/accounts.md) | `/accounts`, `/accounts/preferences` — list, read, update. |
| [`api/dealing_positions.md`](api/dealing_positions.md) | `/positions`, `/positions/otc`, `/confirms` — list (v1+v2), open, update, close. |
| [`api/dealing_working_orders.md`](api/dealing_working_orders.md) | `/workingorders` — list (v1+v2), create, update, delete. |
| [`api/markets.md`](api/markets.md) | `/markets`, `/marketnavigation` — search v1, bulk v2, single v3, navigation. |
| [`api/prices.md`](api/prices.md) | `/prices/{epic}` — historical prices, three independent endpoints v1/v2/v3. |
| [`api/watchlists.md`](api/watchlists.md) | `/watchlists` — full CRUD on watchlists and their markets. |
| [`api/client_sentiment.md`](api/client_sentiment.md) | `/clientsentiment`, `/clientsentiment/related`. |
| [`api/history.md`](api/history.md) | `/history/activity` (v1+v3) and `/history/transactions` (v1+v2). |
| [`api/operations.md`](api/operations.md) | `/operations/application` — list, update, disable API key. |
| [`api/repeat_dealing.md`](api/repeat_dealing.md) | `/repeat-dealing-window`. |
| [`api/streaming.md`](api/streaming.md) | Lightstreamer protocol details. **Vague 3 only — skip for REST work.** |

## Suggested reading paths

- **Implementing a REST domain (Vague 1)** → `architecture.md`,
  `naming.md`, `errors.md`, `dto_conventions.md`, `testing.md`,
  `adding_endpoint.md`, `api/_overview.md`, plus the `api/<your-domain>.md`.
- **Streaming work (Vague 3)** → `architecture.md`, `errors.md`,
  `tracing.md`, `api/_overview.md`, `api/streaming.md`.
- **Reviewing or extending foundations** → `architecture.md`, `errors.md`,
  `http_transport.md`, `tracing.md`.
