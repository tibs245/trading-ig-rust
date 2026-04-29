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

## Subscription items (typical)

| Item                  | Mode      | Fields (typical)                                          |
| --------------------- | --------- | --------------------------------------------------------- |
| `MARKET:<epic>`       | `MERGE`   | `BID`, `OFFER`, `UPDATE_TIME`, `MARKET_STATE`             |
| `CHART:<epic>:<scale>`| `DISTINCT`| `BID`, `OFR`, `LTP`, `LTV`, `UTM`                         |
| `ACCOUNT:<accountId>` | `MERGE`   | `PNL`, `DEPOSIT`, `AVAILABLE_CASH`, `FUNDS`, `MARGIN`     |
| `TRADE:<accountId>`   | `DISTINCT`| `CONFIRMS`, `OPU`, `WOU`                                  |

## Implementation strategy

- Stand-alone TLCP parser in `streaming/protocol.rs`.
- Reader task on a dedicated `tokio` task; deserialise frames into
  typed structs per subscription kind.
- Reconnect / rebind on `LOOP` and `SYNC ERROR` automatically.
- Expose subscriptions as `tokio::sync::mpsc::Receiver<MarketUpdate>`
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
