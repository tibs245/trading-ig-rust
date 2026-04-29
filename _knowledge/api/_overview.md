# API overview — cross-cutting

Read this before any specific domain file.

## Base URLs (per environment)

| Environment | Base URL                                  |
| ----------- | ----------------------------------------- |
| Demo        | `https://demo-api.ig.com/gateway/deal/`   |
| Live        | `https://api.ig.com/gateway/deal/`        |

`Environment::Custom(Url)` exists for tests (point at wiremock).

## Required headers (all endpoints)

| Header             | When             | Notes                                               |
| ------------------ | ---------------- | --------------------------------------------------- |
| `X-IG-API-KEY`     | always           | Issued via My IG > API keys                         |
| `Accept`           | always           | `application/json; charset=UTF-8`                   |
| `Content-Type`     | always           | `application/json; charset=UTF-8`                   |
| `Version`          | almost always    | Per-endpoint, values `1`, `2`, or `3`               |
| `CST`              | v1/v2 sessions   | Returned as response header by `POST /session`      |
| `X-SECURITY-TOKEN` | v1/v2 sessions   | Returned as response header by `POST /session`      |
| `Authorization`    | v3 sessions      | `Bearer <access_token>` (60 s TTL, refreshable)     |
| `IG-ACCOUNT-ID`    | v3 sessions      | The active account number                           |

`Transport` injects all of these automatically — domain code does not
touch headers.

## Error payload shape

Non-2xx responses carry:

```json
{ "errorCode": "error.security.invalid-details" }
```

Some include extra fields (`errorMessage`, etc.). `Transport` parses
this into `ApiError { error_code, extra }` and surfaces it as
`Error::Api { status, source }`.

Common error codes:

- `error.security.invalid-details` — bad credentials.
- `error.public-api.failure.kyc.required` — account not yet KYC-cleared.
- `error.public-api.failure.client-token-invalid` — CST/XST expired.
- `error.public-api.failure.oauth-token-invalid` — bearer expired.
- `error.public-api.exceeded-api-key-allowance`,
  `error.public-api.exceeded-account-allowance`,
  `error.public-api.exceeded-account-trading-allowance` — rate limited.

## Rate limit buckets (for reference)

Two distinct quotas:

1. **Trading** — open / close / update positions and working orders.
2. **Non-trading** — everything else.

We do not yet implement client-side rate limiting (see
`_knowledge/out_of_scope.md`).

## Pagination patterns

- **v3 `/history/activity`** — follow `metadata.paging.next` URL until
  null.
- **v3 `/prices/{epic}`** — increment `pageNumber`; total pages in
  `metadata.pageData.totalPages`.

## Date / time formats

See `_knowledge/time_formats.md`. Use the helpers in `src/time.rs`.

## EPICs and deal IDs

- **EPICs** can contain dots (`CS.D.GBPUSD.TODAY.IP`). URL-encode if
  you ever build a path manually — but prefer `format!()` and let
  reqwest do the encoding via the URL parser.
- **Deal IDs** are opaque strings; treat them as such.
