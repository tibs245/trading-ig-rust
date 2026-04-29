# Session API

> **Already implemented in Vague 0.** This file is kept for reference;
> do not re-port unless adding a missing endpoint.

## `POST /session` — login

Two flavours, both implemented:

**v2 — CST / X-SECURITY-TOKEN** (`SessionApi::login_v2`)

- Request: `{ identifier, password, encryptedPassword? }`.
- Response body: `{ accountId, clientId, lightstreamerEndpoint,
  currencyIsoCode, locale, accountType, … }`.
- Tokens are returned as **response headers**: `CST` and
  `X-SECURITY-TOKEN`. Used by subsequent v1/v2 calls and for the
  Lightstreamer password.

**v3 — OAuth** (`SessionApi::login`)

- Same request body.
- Response body adds `oauthToken { access_token, refresh_token,
  token_type, expires_in (string of seconds, ~60) }`.
- Subsequent calls use `Authorization: Bearer …` and
  `IG-ACCOUNT-ID`.

## `POST /session/refresh-token` (v1) — refresh

`SessionApi::refresh()`. Body `{ refresh_token }`. Returns a fresh
OAuth payload. Update token store atomically.

## `GET /session` (v1) — read session

Optional query `fetchSessionTokens=true`. Returns
`{ accountId, clientId, accountType, currency, locale, timezone,
lightstreamerEndpoint }`. Useful for v3 sessions that need a CST/XST
pair (for streaming).

**Not yet implemented** — add if the streaming work needs it.

## `PUT /session` (v1) — switch account

Body `{ accountId, defaultAccount }`. Sets the active trading account.

**Not yet implemented.**

## `DELETE /session` (v1) — logout

`SessionApi::logout()`. Best-effort: even if the server call fails, we
clear local state.

## `GET /session/encryptionKey` — encryption key (no Version header)

Returns `{ encryptionKey, timeStamp }`. Used to RSA-encrypt the
password (PKCS#1 v1.5) before login. Behind the optional `encryption`
crate feature — **not yet implemented.**
