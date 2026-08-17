# Streaming (Lightstreamer) — Vague 3

> **Skip this file unless you are working on Vague 3.** Streaming is
> not part of the Vague 1 (REST) effort.

The Python `trading-ig` library bundles a minimal Lightstreamer client
(`lightstreamer.py` ~17 KB) plus a thin wrapper (`streamer/`). We need
to port both.

## Connection

- **Endpoint URL**: `lightstreamerEndpoint` from the session response
  (`/session` v2 or v3).
- **Auth password format**: `CST-<cst>|XST-<xst>` (constructed from
  the v1/v2 session).
- For **v3 (OAuth) sessions**: must call `GET /session?fetchSessionTokens=true`
  to obtain a CST/XST pair before opening the stream.
- **Adapter set**: `DEFAULT` is the only public set documented.
- **Username**: the IG account id.

## Lightstreamer protocol notes

The library implements Lightstreamer's TLCP protocol (text/streaming).
Reference: <https://www.lightstreamer.com/api/ls-server/latest/proto.html>.

**Endpoints (HTTP/POST + chunked-streaming response):**

- `POST .../lightstreamer/create_session.txt`
- `POST .../lightstreamer/bind_session.txt`
- `POST .../lightstreamer/control.txt`

**Frame format** — pipe-delimited:
```
[item_index]|[field1_value]|[field2_value]|...
```
Special tokens:
- `$` → empty string
- `#` → null
- missing trailing values → "unchanged from last update"

**Server commands** (received on the streaming channel):

| Command       | Meaning                          |
| ------------- | -------------------------------- |
| `OK`          | Successful subscription          |
| `PROBE`       | Keep-alive (every ~5 s)          |
| `LOOP`        | Server requests `bind_session`   |
| `SYNC ERROR`  | Re-subscribe required            |
| `ERROR`       | Connection failed                |
| `END`         | Server closed the session        |

## Subscription items

Source of truth: <https://labs.ig.com/streaming-api-reference.html>.

| Item                             | Mode      | Data adapter | Ported |
| -------------------------------- | --------- | ------------ | ------ |
| `PRICE:<accountId>:<epic>`       | `MERGE`   | `Pricing`    | yes    |
| `MARKET:<epic>` *(deprecated)*   | `MERGE`   | *(default)*  | yes    |
| `CHART:<epic>:TICK`              | `DISTINCT`| *(default)*  | yes    |
| `CHART:<epic>:<scale>`           | `MERGE`   | *(default)*  | yes    |
| `ACCOUNT:<accountId>`            | `MERGE`   | *(default)*  | yes    |
| `TRADE:<accountId>`              | `DISTINCT`| *(default)*  | yes    |

`PRICE` is the **only** family that needs a non-default data adapter.
`control()` sends `LS_data_adapter=Pricing` for it and omits the
parameter for every other family — see
`streaming::subscription::kind_wire_params`.

`<scale>` is `SECOND`, `1MINUTE`, `5MINUTE`, or `HOUR`.

## MARKET is deprecated — migrate to PRICE

IG's reference page carries this warning on the `MARKET` subscription:

> This subscription reaches end of life on 1 May 2026 and will be
> decommissioned on 8 May 2026. L1, an alias for MARKET, is also
> affected. Please migrate to the PRICE subscription before then.

`StreamingClient::subscribe_market` is `#[deprecated]` and also logs a
`tracing::warn!` once per process, because a compile-time attribute is
invisible to an already-deployed bot.

### Field mapping

| `MARKET` field | `PRICE` field | `PriceUpdate` field |
| -------------- | ------------- | ------------------- |
| `BID`          | `BIDPRICE1`   | `bid`               |
| `OFFER`        | `ASKPRICE1`   | `offer`             |
| `HIGH`         | `HIGH`        | `high`              |
| `LOW`          | `LOW`         | `low`               |
| `MID_OPEN`     | `MID_OPEN`    | `mid_open`          |
| `CHANGE`       | `NET_CHG`     | `change`            |
| `CHANGE_PCT`   | `NET_CHG_PCT` | `change_pct`        |
| `UPDATE_TIME`  | `TIMESTAMP`   | `timestamp`         |
| `MARKET_DELAY` | `DELAY`       | `delayed`           |
| `MARKET_STATE` | `DLG_FLAG`    | `dlg_flag`          |
| `STRIKE_PRICE` | *(none)*      | —                   |
| `ODDS`         | *(none)*      | —                   |

Three traps:

1. **The item name gains the account id.** `MARKET:<epic>` becomes
   `PRICE:<accountId>:<epic>`.
2. **`timestamp` is UTC milliseconds.** `UPDATE_TIME` was a UK-local
   (GMT/BST) `HH:MM:SS` string. Anything parsing that string breaks.
3. **`DLG_FLAG` is not `MARKET_STATE`.** The vocabularies differ and
   there is no exact 1:1 mapping — match on the `PRICE` values, do not
   translate.

| `MARKET_STATE`    | `DLG_FLAG` (closest)         |
| ----------------- | ---------------------------- |
| `TRADEABLE`       | `DEAL` / `DEALNOEDIT`        |
| `CLOSED`          | `CLOSED`                     |
| `SUSPENDED`       | `SUSPEND`                    |
| `EDIT`            | `EDIT`                       |
| `AUCTION`         | `AUCTION`                    |
| `AUCTION_NO_EDIT` | `AUCTIONNOEDIT`              |
| `OFFLINE`         | *(no equivalent)*            |
| *(none)*          | `CALL`, `CLOSINGSONLY`       |

`PRICE` also adds a 5-tier dealing ladder (`bid_prices`, `ask_prices`,
`bid_sizes`, `ask_sizes`), the quote IDs needed to deal on a streamed
price (`bid_quote_id` / `ask_quote_id`), and the ladder currency
(`currency`, from `CURRENCY0`).

### Deliberately not ported

`CURRENCY1`–`CURRENCY5` and the `C1`–`C5` `BIDSIZE{1-5}` /
`ASKSIZE{1-5}` blocks (55 fields) carry per-currency ladder *trading
size thresholds*, not prices. They are only meaningful on
multi-currency ladders. Add them if a caller needs them — the wire
order in `PRICE_FIELDS` is append-friendly.

### Live validation still owed

`LS_data_adapter=Pricing` is taken from IG's reference page ("Subscription
data adapter: Pricing"). It has **not** been confirmed against a live
demo session — do that before shipping to production, and check whether
the account has PRICE permissions ("price availability subject to
account permissions").

## Implementation strategy

- Stand-alone TLCP parser in `streaming/protocol.rs`.
- Reader task on a dedicated `tokio` task; deserialise frames into
  typed structs per subscription kind.
- Reconnect / rebind on `LOOP` and `SYNC ERROR` automatically.
- Expose subscriptions as `tokio::sync::mpsc::Receiver<PriceUpdate>`
  (or similar) per subscription.

## Live testing

The protocol is non-trivial to mock correctly. Real demo-account
testing is required — wait until credentials are provided.

## Reference implementations

- Python `trading-ig/lightstreamer.py` (this lib's source).
- Lightstreamer Java SDK (their reference impl).
- `lightstreamer-rs` on crates.io — small community port that may
  serve as inspiration. Decide on a per-feature basis whether to
  depend on it or vendor specific bits.
