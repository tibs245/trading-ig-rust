# IG Markets API Catalog

Extracted from the official [`trading-ig`](https://github.com/ig-python/trading-ig)
Python project (commit at time of extraction). This catalog is the
specification used to scope the Rust port.

For each endpoint we record: HTTP verb, path, required `Version` header,
request/response shape, and notable quirks.

---

## Cross-cutting concerns

### Base URLs (per environment)

| Environment | Base URL                                  |
| ----------- | ----------------------------------------- |
| Demo        | `https://demo-api.ig.com/gateway/deal`    |
| Live        | `https://api.ig.com/gateway/deal`         |

### Required headers (all endpoints)

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

### Date / time formats

| Context                     | Format                                |
| --------------------------- | ------------------------------------- |
| `conv_datetime` v1 (input)  | `YYYY:MM:DD-HH:MM:SS`                 |
| `conv_datetime` v2 (input)  | `YYYY/MM/DD HH:MM:SS`                 |
| `conv_datetime` v3 (input)  | `YYYY-MM-DDTHH:MM:SS` (ISO 8601)      |
| Prices v3 `snapshotTimeUTC` | `YYYY-MM-DDTHH:MM:SS` (UTC)           |
| Activity / Transactions v3  | `YYYY-MM-DDTHH:MM:SS` (ISO 8601)      |

### Errors & rate limiting

- **Error response shape**: `{ "errorCode": "string", ...optional fields }`.
- **Custom exceptions** (Python lib): `IGException`, `ApiExceededException`,
  `TokenInvalidException`, `KycRequiredException`. We map to a single
  `Error::Api { code, message, status }` with helper predicates.
- **Two rate-limit buckets**: trading endpoints (write side: open/close/
  update positions and working orders) vs non-trading. Token-bucket
  limiter recommended.

### Pagination

- v3 `/history/activity`: follow `metadata.paging.next` URL until null.
- v3 `/prices/{epic}`: increment `pageNumber`; total pages in
  `metadata.pageData.totalPages`.

---

## Authentication & Session

### `POST /session` (v2 — CST/XST)
- **Request**: `{ identifier, password, encryptedPassword? }`
- **Response**: includes `lightstreamerEndpoint`, `accountId`,
  `currencyIsoCode`, `locale`. Tokens **`CST`** and **`X-SECURITY-TOKEN`**
  returned as response **headers** (not body). Used for subsequent calls.

### `POST /session` (v3 — OAuth)
- **Request**: same body as v2.
- **Response body**: `oauthToken { access_token, refresh_token, token_type, expires_in (≈ 60 s) }`,
  plus `accountId` etc. Subsequent calls use `Authorization: Bearer …`
  and `IG-ACCOUNT-ID`.

### `POST /session/refresh-token` (v1)
- v3 only. Body: `{ refresh_token }`. Returns a fresh access token.

### `GET /session` (v1)
- Optional query `fetchSessionTokens=true`. Returns `accountId`,
  `clientId`, `accountType`, `currency`, `locale`, `timezone`,
  `lightstreamerEndpoint`.

### `PUT /session` (v1) — switch account
- `{ accountId, defaultAccount }`. Sets active account.

### `DELETE /session` (v1)
- No body. Logs out.

### `GET /session/encryptionKey` (no Version header)
- Returns `{ encryptionKey, timeStamp }`. Used to RSA-encrypt the
  password before login (PKCS#1 v1.5).

---

## Accounts

### `GET /accounts` (v1)
List accounts. Each entry: `{ accountId, accountAlias, accountType, accountName,
canTransferToMA, canTransferFromMA, defaultAccount, preferred,
balance: { balance, deposit, profitLoss, availableCash } }`.

### `GET /accounts/preferences` (v1)
Returns at minimum `{ trailingStopsEnabled }`.

### `PUT /accounts/preferences` (v1)
Body `{ trailingStopsEnabled: "true"|"false" }` (note: stringified bool).

---

## Dealing — Positions (OTC)

### `GET /positions` (v1, v2)
Returns `{ positions: [{ position: {...}, market: {...} }] }`.
- v2 adds `createdDateUTC`.
- v2 `position`: `contractSize, controlledRisk, createdDate, dealId,
  dealReference, direction (BUY|SELL), level, limitLevel?, size, stopLevel?,
  trailingStep?, trailingStopDistance?`.
- `market`: `bid, offer, epic, expiry, instrumentName, instrumentType,
  lotSize, marketStatus, scalingFactor, updateTime`.

### `GET /positions/{dealId}` (v2)
Single position with the same shape as one entry above.

### `POST /positions/otc` (v2)
Body: `{ currencyCode, direction, epic, expiry, forceOpen, guaranteedStop,
level?, limitDistance?, limitLevel?, orderType (LIMIT|MARKET|QUOTE),
quoteId?, size, stopDistance?, stopLevel?, timeInForce?, trailingStop?,
trailingStopIncrement? }`. Response: `{ dealReference }`. Use
`GET /confirms/{dealReference}` to read the result.

### `PUT /positions/otc/{dealId}` (v2)
Body: `{ guaranteedStop, limitLevel?, stopLevel?, trailingStop?,
trailingStopDistance?, trailingStopIncrement? }`.

### `DELETE /positions/otc` (v1)
Body: `{ dealId, direction, epic, expiry, level, orderType, quoteId?, size,
timeInForce? }`. **Sent as DELETE with a body** — Python lib uses
`_method=POST` workaround; reqwest can send DELETE+body directly.

### `GET /confirms/{dealReference}` (v1)
Returns `{ dealReference, dealStatus (ACCEPTED|REJECTED|UNKNOWN), reason?,
direction, epic, expiry, level, limitLevel?, stopLevel?, orderType, size,
timestamp }`. Python retries up to 5× with 1 s back-off on non-200.

---

## Dealing — Working Orders

### `GET /workingorders` (v1, v2)
`v2` items combine `workingOrderData { createdDate, currencyCode, dealId,
direction, dma, epic, goodTillDate, goodTillDateISO, guaranteedStop,
limitDistance, orderLevel, orderSize, orderType, stopDistance,
timeInForce }` and `marketData { instrumentName, exchangeId,
streamingPricesAvailable, offer, low, bid, updateTime, expiry, high,
marketStatus, delayTime, lotSize, percentageChange, epic, netChange,
instrumentType, scalingFactor }`.

### `POST /workingorders/otc` (v2)
Body: `{ currencyCode, direction, epic, expiry, forceOpen, guaranteedStop,
level, size, timeInForce, type (LIMIT|STOP), goodTillDate?,
limitDistance?, limitLevel?, stopDistance?, stopLevel?, dealReference? }`.
`goodTillDate` accepts string or `YYYY/MM/DD HH:MM:SS`.

### `PUT /workingorders/otc/{dealId}` (v2)
All fields required: `{ goodTillDate, level, limitDistance, limitLevel,
stopDistance, stopLevel, guaranteedStop, timeInForce, type }`.

### `DELETE /workingorders/otc/{dealId}` (v2)
Empty body.

---

## Markets & Navigation

### `GET /markets?searchTerm=…` (v1)
Returns `{ markets: [{ bid, offer, epic, expiry, instrumentName,
instrumentType, marketStatus, streamingPricesAvailable }] }`.

### `GET /markets?epics=A,B,C&filter=ALL|SNAPSHOT_ONLY` (v2)
Returns `{ marketDetails: [...] }`. Each entry: `instrument`,
`dealingRules`, `snapshot`. Many sub-fields (lotSize, chartCode,
minDealSize, openingTime, closingTime, scaling factor, etc.).

### `GET /markets/{epic}` (v3)
One market with the same `marketDetails` entry shape.

### `GET /marketnavigation` (v1)
Returns `{ markets: [...], nodes: [{ id, name }] }`.

### `GET /marketnavigation/{node}` (v1)
Same shape, scoped to a node.

---

## Prices (Historical)

### `GET /prices/{epic}` (v3)
Query params: `resolution`, `from` (`YYYY-MM-DDTHH:MM:SS`),
`to` (`YYYY-MM-DDTHH:MM:SS`), `max`, `pageSize` (default 20),
`pageNumber`. Response: `{ prices: [{ snapshotTime, snapshotTimeUTC,
openPrice {bid,ask,lastTraded}, closePrice, highPrice, lowPrice,
lastTradedVolume }], metadata: { pageData { pageNumber, pageSize,
totalPages }, allowance { remainingAllowance, allowanceExpiry } } }`.

Resolutions: `SECOND`, `MINUTE`, `MINUTE_2`, `MINUTE_3`, `MINUTE_5`,
`MINUTE_10`, `MINUTE_15`, `MINUTE_30`, `HOUR`, `HOUR_2`, `HOUR_3`,
`HOUR_4`, `DAY`, `WEEK`, `MONTH`.

### `GET /prices/{epic}/{resolution}/{numpoints}` (v2)
Fixed number of recent points.

### `GET /prices/{epic}/{resolution}` (v1) — query `startdate` & `enddate`
`YYYY:MM:DD-HH:MM:SS` format.

### `GET /prices/{epic}/{resolution}/{startDate}/{endDate}` (v2)
`YYYY/MM/DD HH:MM:SS` format in the path.

---

## Watchlists

### `GET /watchlists` (v1)
Returns `{ watchlists: [{ id, name }] }`.

### `POST /watchlists` (v1)
Body `{ name, epics: [string] }`. Returns `{ id }`.

### `GET /watchlists/{id}` (v1)
Returns `{ markets: [...] }`.

### `PUT /watchlists/{id}` (v1)
Body `{ epic }`. Adds one market.

### `DELETE /watchlists/{id}/{epic}` (v1)
Removes one market.

### `DELETE /watchlists/{id}` (v1)
Deletes the whole watchlist.

---

## Client Sentiment

### `GET /clientsentiment/{marketId}` or `?marketIds=A,B,C` (v1)
Returns `{ marketId, longPositionPercentage, shortPositionPercentage }`.

### `GET /clientsentiment/related/{marketId}` (v1)
Returns `{ clientSentiments: [...] }`.

---

## History — Activity & Transactions

### `GET /history/activity` (v3)
Query: `from`, `to` (ISO), `detailed`, `dealId`, `filter` (FIQL),
`pageSize` (10–500, default 50). Pagination via `metadata.paging.next`.

### `GET /history/activity/{milliseconds}` (v1)
Period in ms from now.

### `GET /history/activity/{fromDate}/{toDate}` (v1)
v1 date format.

### `GET /history/transactions` (v2)
Query: `type`, `from`, `to`, `maxSpanSeconds`, `pageSize`, `pageNumber`.

### `GET /history/transactions/{type}/{milliseconds}` (v1)
Type values: `ALL`, `ALL_DEAL`, `DEPOSIT`, `WITHDRAWAL`.

---

## Operations (Application Management)

### `GET /operations/application` (v1)
Lists app keys.

### `PUT /operations/application` (v1)
`{ allowanceAccountOverall, allowanceAccountTrading, apiKey, status }`.

### `PUT /operations/application/disable` (v1)
Disables current key (re-enable via web).

---

## Repeat Dealing

### `GET /repeat-dealing-window` (v1)
Optional `epic` query.

---

## Streaming (Lightstreamer)

The Python lib bundles a minimal Lightstreamer client (`lightstreamer.py`)
plus a `streamer/manager.py` wrapper.

- **Endpoint**: `lightstreamerEndpoint` from the session response.
- **Auth password format**: `CST-<cst>|XST-<xst>` (v1/v2 session). For v3
  sessions, must call `GET /session?fetchSessionTokens=true` to obtain a
  CST/XST pair anyway.
- **Adapter sets**: `DEFAULT` is the only one in current public docs.
- **Subscription items** (selection):
  - `MARKET:<epic>` — bid/offer/timestamp/etc.
  - `CHART:<epic>:<scale>` — tick chart updates.
  - `ACCOUNT:<accountId>` — balance/PnL stream.
  - `TRADE:<accountId>` — order/position confirmations.
- **Modes**: `MERGE` (latest), `DISTINCT` (every event), `RAW`, `COMMAND`.
- **Wire format**: pipe-delimited TLCP frames; `$` = empty string,
  `#` = null, missing = unchanged.
- **Server commands**: `OK`, `PROBE`, `LOOP` (rebind), `SYNC ERROR`
  (re-subscribe), `ERROR`, `END`.

Implementation strategy: stand-alone TLCP parser + `tokio` reader task,
deserialise items into typed structs per subscription type. The official
JS/Java SDK is a useful reference. Live testing requires demo creds.

---

## Endpoint coverage summary

| Domain            | Endpoints |
| ----------------- | --------- |
| Session           | 6         |
| Accounts          | 3         |
| Positions         | 6         |
| Working orders    | 4         |
| Markets / nav     | 5         |
| Prices            | 3         |
| Watchlists        | 6         |
| Client sentiment  | 2         |
| History           | 5         |
| Operations        | 3         |
| Repeat dealing    | 1         |
| Streaming         | (1 client + N subscriptions) |
| **Total REST**    | **~44**   |
