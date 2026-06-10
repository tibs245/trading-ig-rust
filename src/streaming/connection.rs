//! Lightstreamer session management: `create_session`, `bind_session`,
//! `control` (subscribe / unsubscribe), and the read-loop task.
//!
//! This layer owns the raw HTTP interaction with the Lightstreamer server
//! using `reqwest`'s streaming `chunk()` API.  Subscribers are managed by
//! the [`Registry`] in `subscription.rs`.

use std::collections::HashMap;
use std::sync::Arc;

use reqwest::Client;
use tokio::sync::{RwLock, mpsc, watch};
use tracing::{debug, error, info, warn};

use crate::error::{Error, Result};
use crate::session::SessionHandle;
use crate::streaming::protocol::{Frame, parse_line, parse_ok_block};
use crate::streaming::reconnect::{AutoReconnect, StreamingEvent};
use crate::streaming::subscription::Registry;

// ---------------------------------------------------------------------------
// LsStream — wraps reqwest::Response for line-at-a-time reading
// ---------------------------------------------------------------------------

/// Wrapper around a streaming `reqwest::Response` that buffers bytes and
/// yields complete `\n`-terminated lines.
struct LsStream {
    resp: reqwest::Response,
    buf: String,
}

impl LsStream {
    fn new(resp: reqwest::Response) -> Self {
        Self {
            resp,
            buf: String::new(),
        }
    }

    /// Read the next complete line from the stream (without the trailing `\n`).
    ///
    /// Returns `None` when the server closes the connection.
    async fn next_line(&mut self) -> Option<Result<String>> {
        loop {
            // Return any fully-buffered line first.
            if let Some(nl_pos) = self.buf.find('\n') {
                let line = self.buf[..nl_pos].trim_end_matches('\r').to_owned();
                self.buf.drain(..=nl_pos);
                return Some(Ok(line));
            }

            // Need more data.
            match self.resp.chunk().await {
                Ok(Some(bytes)) => {
                    self.buf.push_str(&String::from_utf8_lossy(&bytes));
                }
                Ok(None) => {
                    // Stream closed by server.
                    return None;
                }
                Err(e) => {
                    return Some(Err(Error::Http(e)));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// CreateParams — bundles the arguments to LsConnection::create
// ---------------------------------------------------------------------------

/// Bundle of parameters for [`LsConnection::create`].
///
/// Introduced to keep the argument count within clippy's threshold.
pub(crate) struct CreateParams {
    pub(crate) endpoint: String,
    pub(crate) username: String,
    pub(crate) password: String,
    pub(crate) registry: Registry,
    pub(crate) shutdown_tx: watch::Sender<bool>,
    pub(crate) policy: AutoReconnect,
    pub(crate) event_tx: Option<mpsc::Sender<StreamingEvent>>,
    pub(crate) session_handle: SessionHandle,
}

// ---------------------------------------------------------------------------
// LsConnection
// ---------------------------------------------------------------------------

/// A live Lightstreamer connection.
///
/// Wraps a `reqwest::Client` configured for streaming, the server endpoint,
/// and the auth credentials.  Does not own subscriptions — the caller's
/// [`crate::streaming::StreamingClient`] holds the [`Registry`].
#[derive(Debug, Clone)]
pub(crate) struct LsConnection {
    pub(crate) client: Client,
    pub(crate) endpoint: String,
    /// Lightstreamer username (IG account ID) — retained for session re-create
    /// after a fatal server-side termination.
    pub(crate) username: String,
    /// Lightstreamer password (`CST-…|XST-…`) — retained for session re-create
    /// after a fatal server-side termination.
    pub(crate) password: String,
    pub(crate) session_id: String,
    pub(crate) control_address: Option<String>,
}

/// Connection state shared between the [`StreamingClient`] and its background
/// read-loop. A reconnect swaps the inner `LsConnection` under the write lock,
/// so every subsequent `control`/`unsubscribe`/`subscribe_*` reads the CURRENT
/// session id from the same lock instead of a stale clone (P1-15).
pub(crate) type SharedConn = Arc<RwLock<LsConnection>>;

/// Owned snapshot of the fields a single `control.txt` request needs, taken
/// under a short read lock and used after the lock is dropped (DD-9).
struct RequestSnapshot {
    client: Client,
    control_url: String,
    session_id: String,
}

impl LsConnection {
    /// Open a new Lightstreamer session via `create_session.txt`.
    ///
    /// Returns the connection object.  A background task is spawned that
    /// reads frames and dispatches them to registered subscribers.
    /// Sending `true` on `shutdown_tx` stops the read-loop.
    pub(crate) async fn create(params: CreateParams) -> Result<SharedConn> {
        let CreateParams {
            endpoint,
            username,
            password,
            registry,
            shutdown_tx,
            policy,
            event_tx,
            session_handle,
        } = params;

        let client = Client::builder().build().map_err(Error::Http)?;

        let url = format!("{endpoint}/lightstreamer/create_session.txt");
        debug!(%url, "opening Lightstreamer session");

        let mut form = HashMap::new();
        form.insert("LS_op2", "create");
        form.insert("LS_cid", "mgQkwtwdysogQz2BJ4Ji kOj2Bg");
        form.insert("LS_adapter_set", "DEFAULT");
        form.insert("LS_user", username.as_str());
        form.insert("LS_password", password.as_str());
        form.insert("LS_polling", "false");

        let resp = client
            .post(&url)
            .form(&form)
            .send()
            .await
            .map_err(Error::Http)?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.bytes().await.map_err(Error::Http)?;
            let snippet = String::from_utf8_lossy(&body).into_owned();
            return Err(Error::Auth(format!(
                "Lightstreamer create_session failed ({status}): {snippet}"
            )));
        }

        // Read the OK block from the streaming response.
        let (session_id, control_address, stream) = read_ok_block(resp).await?;

        info!(%session_id, "Lightstreamer session created");

        // Single shared connection: the returned client and the read-loop both
        // hold this Arc, so a reconnect-driven swap is visible to every later
        // control/subscribe (P1-15).
        let conn: SharedConn = Arc::new(RwLock::new(LsConnection {
            client,
            endpoint,
            username,
            password,
            session_id,
            control_address,
        }));

        // Spawn the background read-loop on a clone of the same shared handle.
        let registry2 = registry.clone();
        let conn2 = Arc::clone(&conn);
        let mut shutdown_rx = shutdown_tx.subscribe();
        tokio::spawn(async move {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    debug!("Lightstreamer read-loop: shutdown signal received");
                }
                () = read_loop(stream, registry2, conn2, policy, event_tx, session_handle) => {}
            }
        });

        Ok(conn)
    }

    /// Return the base URL for `control.txt` calls.
    ///
    /// The Lightstreamer server returns a `ControlAddress` header that may be
    /// just a hostname (no scheme).  We normalise it by prepending `https://`
    /// when no scheme is present.
    fn control_base_url(&self) -> String {
        match &self.control_address {
            None => self.endpoint.clone(),
            Some(addr) => {
                if addr.starts_with("http://") || addr.starts_with("https://") {
                    addr.clone()
                } else {
                    format!("https://{addr}")
                }
            }
        }
    }

    /// Snapshot the small per-request fields (client + url + session id) under
    /// a read lock so the HTTP call below NEVER holds the lock across `.await`
    /// (DD-9). The returned owned values reflect the CURRENT session.
    fn request_snapshot(&self) -> RequestSnapshot {
        RequestSnapshot {
            client: self.client.clone(),
            control_url: format!("{}/lightstreamer/control.txt", self.control_base_url()),
            session_id: self.session_id.clone(),
        }
    }

    /// Open a `bind_session.txt` connection and return the streaming response.
    async fn bind_inner(&self) -> Result<LsStream> {
        let url = format!("{}/lightstreamer/bind_session.txt", self.endpoint);
        debug!(%url, session_id = %self.session_id, "binding Lightstreamer session");

        let mut params = HashMap::new();
        params.insert("LS_session", self.session_id.as_str());
        params.insert("LS_polling", "false");

        let resp = self
            .client
            .post(&url)
            .form(&params)
            .send()
            .await
            .map_err(Error::Http)?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.bytes().await.map_err(Error::Http)?;
            return Err(Error::Auth(format!(
                "Lightstreamer bind_session failed ({status}): {}",
                String::from_utf8_lossy(&body)
            )));
        }

        Ok(LsStream::new(resp))
    }

    /// Create a brand-new Lightstreamer session (used during auto-reconnect).
    ///
    /// On success, the caller must replace the current stream and update
    /// `self` with the returned connection's session ID / control address.
    async fn create_session_inner(&self) -> Result<(LsConnection, LsStream)> {
        let url = format!("{}/lightstreamer/create_session.txt", self.endpoint);
        debug!(%url, "re-creating Lightstreamer session after END");

        let mut params = HashMap::new();
        params.insert("LS_op2", "create");
        params.insert("LS_cid", "mgQkwtwdysogQz2BJ4Ji kOj2Bg");
        params.insert("LS_adapter_set", "DEFAULT");
        params.insert("LS_user", self.username.as_str());
        params.insert("LS_password", self.password.as_str());
        params.insert("LS_polling", "false");

        let resp = self
            .client
            .post(&url)
            .form(&params)
            .send()
            .await
            .map_err(Error::Http)?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.bytes().await.map_err(Error::Http)?;
            return Err(Error::Auth(format!(
                "Lightstreamer create_session (reconnect) failed ({status}): {}",
                String::from_utf8_lossy(&body)
            )));
        }

        let (session_id, control_address, stream) = read_ok_block(resp).await?;

        let new_conn = LsConnection {
            client: self.client.clone(),
            endpoint: self.endpoint.clone(),
            username: self.username.clone(),
            password: self.password.clone(),
            session_id,
            control_address,
        };
        Ok((new_conn, stream))
    }
}

// ---------------------------------------------------------------------------
// control.txt requests against the shared connection
// ---------------------------------------------------------------------------

/// Send a `control.txt` request (subscribe / unsubscribe) using the CURRENT
/// session in `conn`. Snapshots session id + url under a short read lock, drops
/// the guard, THEN awaits the HTTP — never holding the lock across `.await`.
pub(crate) async fn control(
    conn: &SharedConn,
    op: &str,
    item_index: usize,
    item_name: &str,
    fields: &str,
    mode: &str,
) -> Result<()> {
    let snap = conn.read().await.request_snapshot();

    let item_index_str = item_index.to_string();
    let mut params = HashMap::new();
    params.insert("LS_session", snap.session_id.as_str());
    params.insert("LS_op", op);
    params.insert("LS_table", item_index_str.as_str());
    params.insert("LS_id", item_name);
    params.insert("LS_schema", fields);
    params.insert("LS_mode", mode);

    let resp = snap
        .client
        .post(&snap.control_url)
        .form(&params)
        .send()
        .await
        .map_err(Error::Http)?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.bytes().await.map_err(Error::Http)?;
        return Err(Error::Auth(format!(
            "Lightstreamer control failed ({status}): {}",
            String::from_utf8_lossy(&body)
        )));
    }

    // Lightstreamer answers `200 OK` with body `ERROR\n<code>\n<msg>` for a
    // dead/unknown session — the HTTP status alone is not the truth. Without
    // this the `add` silently "succeeds" and the feed stays dead.
    let body = resp.bytes().await.map_err(Error::Http)?;
    classify_control_body(&String::from_utf8_lossy(&body))
}

/// Send a `control.txt` unsubscribe for `item_index` using the CURRENT session.
pub(crate) async fn unsubscribe(conn: &SharedConn, item_index: usize) -> Result<()> {
    let snap = conn.read().await.request_snapshot();

    let item_index_str = item_index.to_string();
    let mut params = HashMap::new();
    params.insert("LS_session", snap.session_id.as_str());
    params.insert("LS_op", "delete");
    params.insert("LS_table", item_index_str.as_str());

    let resp = snap
        .client
        .post(&snap.control_url)
        .form(&params)
        .send()
        .await
        .map_err(Error::Http)?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.bytes().await.map_err(Error::Http)?;
        return Err(Error::Auth(format!(
            "Lightstreamer unsubscribe failed ({status}): {}",
            String::from_utf8_lossy(&body)
        )));
    }
    Ok(())
}

/// Open a `bind_session.txt` connection on the CURRENT session in `conn`.
///
/// Snapshots the connection under a short read lock, drops the guard, then
/// awaits the HTTP — the lock is never held across `.await`.
async fn bind(conn: &SharedConn) -> Result<LsStream> {
    let snapshot = conn.read().await.clone();
    snapshot.bind_inner().await
}

/// Create a brand-new Lightstreamer session from the CURRENT credentials in
/// `conn`. Same snapshot-then-release discipline as [`bind`].
async fn create_session(conn: &SharedConn) -> Result<(LsConnection, LsStream)> {
    let snapshot = conn.read().await.clone();
    snapshot.create_session_inner().await
}

// ---------------------------------------------------------------------------
// control.txt body classification
// ---------------------------------------------------------------------------

/// Classify a `control.txt` response body.
///
/// Lightstreamer returns `200 OK` even for a rejected control request: the
/// body's first whitespace token discriminates `OK` from `ERROR`/`SYNC ERROR`.
/// Returns [`Error::Streaming`] (NOT [`Error::Auth`]) on an error body so the
/// caller stays unsubscribed and retries rather than refreshing tokens.
fn classify_control_body(body: &str) -> Result<()> {
    let first_token = body.split_whitespace().next().unwrap_or("");
    if first_token == "ERROR" || body.trim_start().starts_with("SYNC ERROR") {
        return Err(Error::Streaming(format!(
            "Lightstreamer control returned ERROR: {}",
            body.trim()
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// OK-block reader
// ---------------------------------------------------------------------------

/// Read the initial OK block from a streaming response, then return the
/// remainder of the stream wrapped in an `LsStream`.
async fn read_ok_block(resp: reqwest::Response) -> Result<(String, Option<String>, LsStream)> {
    let mut stream = LsStream::new(resp);
    let mut header_lines: Vec<String> = Vec::new();
    let mut header_done = false;

    loop {
        match stream.next_line().await {
            Some(Ok(line)) => {
                if line.is_empty() {
                    // Blank line terminates the header block.
                    header_done = true;
                    break;
                }
                header_lines.push(line);
            }
            Some(Err(e)) => return Err(e),
            None => break, // stream closed before we got a full header
        }
    }

    if !header_done {
        return Err(Error::Auth("Lightstreamer OK block not received".into()));
    }

    // Reconstruct block string and parse it.
    let block = format!("{}\r\n\r\n", header_lines.join("\r\n"));
    let (session_id, extras) = parse_ok_block(&block).ok_or_else(|| {
        Error::Auth(format!(
            "Lightstreamer session error: {}",
            header_lines.join(", ")
        ))
    })?;

    let control_address = extras
        .into_iter()
        .find(|(k, _)| k == "ControlAddress")
        .map(|(_, v)| v);

    Ok((session_id, control_address, stream))
}

// ---------------------------------------------------------------------------
// Read-loop
// ---------------------------------------------------------------------------

/// What to do when `bind_session` fails in the rebind path (U4 / DD-11).
#[derive(Debug, PartialEq, Eq)]
enum BindFailureAction {
    /// Auto-reconnect enabled: full session re-create with token refresh.
    Reconnect,
    /// Auto-reconnect disabled: emit `Disconnected` and stop (mirrors the
    /// disabled `SessionEnded` branch).
    Disconnect,
}

/// Decide the bind-failure route from the reconnect policy. Pure so the U4
/// routing is unit-testable without a live Lightstreamer stream (AC-7).
fn bind_failure_action(policy_enabled: bool) -> BindFailureAction {
    if policy_enabled {
        BindFailureAction::Reconnect
    } else {
        BindFailureAction::Disconnect
    }
}

/// Outcome of draining a stream — determines the read-loop's next action.
enum DrainOutcome {
    /// Server sent `LOOP` or EOF: call `bind_session` on the same session.
    Rebind,
    /// Server sent `END` or fatal error: a full session reconnect is needed.
    SessionEnded { reason: Option<String> },
    /// Caller sent a shutdown signal or an unrecoverable internal error.
    Terminate,
}

/// Background task: consume a line-stream from Lightstreamer, parse frames,
/// and dispatch to the registry.  Handles:
///
/// - `LOOP` / EOF → rebind (same session).
/// - `END` / fatal error → attempt a full reconnect if `policy.enabled`.
async fn read_loop(
    mut stream: LsStream,
    registry: Registry,
    conn: SharedConn,
    policy: AutoReconnect,
    event_tx: Option<mpsc::Sender<StreamingEvent>>,
    session_handle: SessionHandle,
) {
    loop {
        match drain_stream(&mut stream, &registry, &conn).await {
            DrainOutcome::Rebind => {
                // ---- Rebind path (existing LOOP behaviour) ----
                debug!("attempting bind_session");
                match bind(&conn).await {
                    Ok(new_stream) => {
                        stream = new_stream;
                        resubscribe_all(&conn, &registry).await;
                        let rebound_id = conn.read().await.session_id.clone();
                        info!(session_id = %rebound_id, "Lightstreamer session rebound");
                    }
                    Err(e) => {
                        // U4 (DD-11): a bind failure must not die silently. Route
                        // it the same way as SessionEnded — full reconnect when
                        // the policy is enabled, otherwise emit Disconnected.
                        error!(error = %e, "Lightstreamer bind_session failed");
                        let reason = format!("bind_session failed: {e}");
                        match bind_failure_action(policy.enabled) {
                            BindFailureAction::Disconnect => {
                                emit_event(
                                    event_tx.as_ref(),
                                    StreamingEvent::Disconnected {
                                        reason: Some(reason),
                                    },
                                )
                                .await;
                                return;
                            }
                            BindFailureAction::Reconnect => {
                                let reconnected = attempt_reconnect(
                                    &conn,
                                    &mut stream,
                                    &registry,
                                    &policy,
                                    event_tx.as_ref(),
                                    &session_handle,
                                    Some(reason),
                                )
                                .await;
                                if !reconnected {
                                    // attempt_reconnect already emitted ReconnectFailed.
                                    return;
                                }
                                // else: fall through and read the fresh stream.
                            }
                        }
                    }
                }
            }

            DrainOutcome::SessionEnded { reason } => {
                // ---- END / fatal path ----
                if !policy.enabled {
                    // Auto-reconnect disabled — close as before.
                    debug!(reason = ?reason, "END received; auto-reconnect disabled");
                    emit_event(
                        event_tx.as_ref(),
                        StreamingEvent::Disconnected {
                            reason: reason.clone(),
                        },
                    )
                    .await;
                    return;
                }

                // Auto-reconnect loop.
                let reconnected = attempt_reconnect(
                    &conn,
                    &mut stream,
                    &registry,
                    &policy,
                    event_tx.as_ref(),
                    &session_handle,
                    reason,
                )
                .await;

                if !reconnected {
                    return;
                }
                // Reconnected — fall through to the outer loop and keep
                // reading the new stream.
            }

            DrainOutcome::Terminate => {
                return;
            }
        }
    }
}

/// Try to re-establish the Lightstreamer session up to `policy.max_attempts`
/// times, refreshing CST/XST tokens on each attempt.
///
/// Returns `true` if reconnect succeeded, `false` if permanently exhausted.
async fn attempt_reconnect(
    conn: &SharedConn,
    stream: &mut LsStream,
    registry: &Registry,
    policy: &AutoReconnect,
    event_tx: Option<&mpsc::Sender<StreamingEvent>>,
    session_handle: &SessionHandle,
    initial_reason: Option<String>,
) -> bool {
    let mut attempt: u32 = 0;
    let mut last_error = initial_reason
        .clone()
        .unwrap_or_else(|| "session ended".to_owned());

    loop {
        attempt += 1;

        // Check attempt limit before doing anything expensive.
        if let Some(max) = policy.max_attempts
            && attempt > max
        {
            error!(
                attempts = attempt - 1,
                "Lightstreamer auto-reconnect: max attempts exceeded; giving up"
            );
            emit_event(
                event_tx,
                StreamingEvent::ReconnectFailed {
                    attempts: attempt - 1,
                    error: last_error,
                },
            )
            .await;
            return false;
        }

        let backoff = policy.backoff_for_attempt(attempt);
        warn!(
            attempt,
            backoff_ms = backoff.as_millis(),
            reason = %last_error,
            "Lightstreamer auto-reconnect: will retry"
        );
        tokio::time::sleep(backoff).await;

        // Branch on session shape : login_v2() on a v3 session would
        // wipe the OAuth bearer + refresh_token and demote REST to CST.
        let api = session_handle.session_api();
        let session_kind = if session_handle
            .session
            .snapshot()
            .await
            .tokens
            .refresh
            .is_some()
        {
            "v3"
        } else {
            "v2"
        };
        let refresh_result = if session_kind == "v3" {
            match api.login().await {
                Ok(_) => api.read(true).await.map(|_| ()),
                Err(e) => Err(e),
            }
        } else {
            api.login_v2().await.map(|_| ())
        };
        if let Err(e) = refresh_result {
            last_error = format!("session refresh failed ({session_kind}): {e}");
            warn!(
                attempt,
                session_kind,
                error = %e,
                "Lightstreamer auto-reconnect: token refresh failed"
            );
            continue; // back off and retry
        }

        let state = session_handle.session.snapshot().await;
        let Some(streaming) = state.tokens.streaming.as_ref() else {
            format!("no streaming tokens after {session_kind} refresh").clone_into(&mut last_error);
            warn!(attempt, "Lightstreamer auto-reconnect: {}", last_error);
            continue;
        };
        let new_password = format!("CST-{}|XST-{}", streaming.cst, streaming.x_security_token);

        // Update the password on the shared connection under a short write
        // lock (dropped before the create_session await — DD-9).
        conn.write().await.password = new_password;

        match create_session(conn).await {
            Ok((new_conn, new_stream)) => {
                let new_session_id = new_conn.session_id.clone();
                info!(
                    attempt,
                    session_id = %new_session_id,
                    "Lightstreamer auto-reconnect: new session established"
                );
                // Swap the connection under the write lock, then DROP the guard
                // synchronously (no await held) so the per-subscription control
                // calls in resubscribe_all can take the read lock (DD-9).
                *conn.write().await = new_conn;
                *stream = new_stream;

                // Re-subscribe all active subscriptions on the fresh session.
                resubscribe_all(conn, registry).await;

                emit_event(event_tx, StreamingEvent::Reconnected { attempt }).await;
                return true;
            }
            Err(e) => {
                last_error = format!("create_session failed: {e}");
                warn!(
                    attempt,
                    error = %e,
                    "Lightstreamer auto-reconnect: create_session failed"
                );
                // continue to next attempt
            }
        }
    }
}

/// Re-subscribe all active entries in the registry on the current connection.
async fn resubscribe_all(conn: &SharedConn, registry: &Registry) {
    let subs = registry.snapshot_for_resubscribe();
    for (idx, name, fields, mode) in subs {
        // Each control() snapshots the CURRENT session under a short read lock
        // (DD-9) — never the pre-swap one.
        if let Err(e) = control(conn, "add", idx, &name, &fields, mode).await {
            warn!(error = %e, "failed to re-subscribe {name} after reconnect");
        }
    }
}

/// Emit a [`StreamingEvent`] on the optional channel, ignoring a closed receiver.
async fn emit_event(event_tx: Option<&mpsc::Sender<StreamingEvent>>, event: StreamingEvent) {
    if let Some(tx) = event_tx {
        // Best-effort; if the caller has dropped the receiver we simply skip.
        let _ = tx.send(event).await;
    }
}

/// Drain a stream until a rebind or session-level action is required.
async fn drain_stream(
    stream: &mut LsStream,
    registry: &Registry,
    conn: &SharedConn,
) -> DrainOutcome {
    loop {
        match stream.next_line().await {
            Some(Ok(line)) => {
                let frame = parse_line(line.as_bytes());
                match handle_frame(frame, registry, conn).await {
                    FrameAction::Continue => {}
                    FrameAction::Rebind => return DrainOutcome::Rebind,
                    FrameAction::SessionEnded { reason } => {
                        return DrainOutcome::SessionEnded { reason };
                    }
                    FrameAction::Terminate => return DrainOutcome::Terminate,
                }
            }
            Some(Err(e)) => {
                error!(error = %e, "Lightstreamer stream error");
                return DrainOutcome::Rebind;
            }
            None => {
                debug!("Lightstreamer stream EOF; will rebind");
                return DrainOutcome::Rebind;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Frame dispatch
// ---------------------------------------------------------------------------

/// Possible outcomes of processing a single frame.
enum FrameAction {
    Continue,
    Rebind,
    SessionEnded { reason: Option<String> },
    Terminate,
}

async fn handle_frame(frame: Frame, registry: &Registry, conn: &SharedConn) -> FrameAction {
    match frame {
        Frame::Update { item_index, fields } => {
            let alive = registry.apply_update(item_index, &fields);
            if !alive {
                registry.remove(item_index);
                // Best-effort unsubscribe — ignore errors.
                let _ = unsubscribe(conn, item_index).await;
            }
            FrameAction::Continue
        }
        Frame::Probe => {
            debug!("PROBE received");
            FrameAction::Continue
        }
        Frame::Loop => {
            info!("LOOP received — rebinding session");
            FrameAction::Rebind
        }
        Frame::SyncError => {
            warn!("SYNC ERROR received — rebinding and re-subscribing");
            FrameAction::Rebind
        }
        Frame::Error { code, message } => {
            error!(code = %code, message = %message, "Lightstreamer ERROR");
            FrameAction::Terminate
        }
        Frame::End { cause } => {
            info!(cause = ?cause, "Lightstreamer END — session terminated by server");
            FrameAction::SessionEnded {
                reason: cause.clone(),
            }
        }
        Frame::Ok { session_id } if !session_id.is_empty() => {
            debug!(%session_id, "Lightstreamer OK (bind acknowledged)");
            // Short write lock to record the bind-acknowledged session id;
            // dropped immediately (no await held).
            conn.write().await.session_id = session_id;
            FrameAction::Continue
        }
        Frame::Ok { .. } | Frame::Unknown(_) => FrameAction::Continue,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- U2: control.txt body classification (DD-10 / AC-9) ---

    #[test]
    fn control_body_error_returns_streaming_err() {
        let err = classify_control_body("ERROR\n22\nunknown session")
            .expect_err("ERROR body must be an error");
        assert!(matches!(err, Error::Streaming(_)));
        // Must NOT be classified as auth — a control-ERROR must not trigger a
        // token refresh in the bot's resubscribe path.
        assert!(!err.is_auth(), "control ERROR must not be is_auth()");
        assert!(!err.is_rate_limited());
    }

    #[test]
    fn control_body_sync_error_returns_streaming_err() {
        let err = classify_control_body("SYNC ERROR").expect_err("SYNC ERROR is fatal");
        assert!(matches!(err, Error::Streaming(_)));
        assert!(!err.is_auth());
    }

    #[test]
    fn control_body_ok_returns_ok() {
        assert!(classify_control_body("OK").is_ok());
    }

    #[test]
    fn control_body_numeric_table_ok_returns_ok() {
        // A subscription confirmation streams a numeric table line, not "ERROR".
        assert!(classify_control_body("1,1|...").is_ok());
    }

    #[test]
    fn control_body_empty_returns_ok() {
        assert!(classify_control_body("").is_ok());
    }

    // --- U3: shared connection session swap (DD-9 / AC-8) ---

    fn test_conn(session_id: &str, control_address: Option<&str>) -> LsConnection {
        LsConnection {
            client: Client::new(),
            endpoint: "https://demo.example".to_owned(),
            username: "acct".to_owned(),
            password: "CST-a|XST-b".to_owned(),
            session_id: session_id.to_owned(),
            control_address: control_address.map(str::to_owned),
        }
    }

    #[tokio::test]
    async fn request_snapshot_reads_current_session_after_swap() {
        // A reconnect swaps the inner LsConnection under the write lock; any
        // subsequent control/subscribe must snapshot the NEW session id, not a
        // pre-swap clone (the P1-15 stale-conn bug).
        let conn: SharedConn = Arc::new(RwLock::new(test_conn("S-OLD", Some("ctrl-old"))));

        let before = conn.read().await.request_snapshot();
        assert_eq!(before.session_id, "S-OLD");
        assert_eq!(
            before.control_url,
            "https://ctrl-old/lightstreamer/control.txt"
        );

        // Simulate attempt_reconnect's swap: write lock, replace, drop guard.
        *conn.write().await = test_conn("S-NEW", Some("ctrl-new"));

        let after = conn.read().await.request_snapshot();
        assert_eq!(after.session_id, "S-NEW");
        assert_eq!(
            after.control_url,
            "https://ctrl-new/lightstreamer/control.txt"
        );
    }

    #[tokio::test]
    async fn request_snapshot_falls_back_to_endpoint_without_control_address() {
        let conn: SharedConn = Arc::new(RwLock::new(test_conn("S1", None)));
        let snap = conn.read().await.request_snapshot();
        assert_eq!(
            snap.control_url,
            "https://demo.example/lightstreamer/control.txt"
        );
    }

    // --- U4: bind-failure routing (DD-11 / AC-7) ---

    #[test]
    fn bind_failure_routes_to_reconnect_when_policy_enabled() {
        // A bind failure with auto-reconnect on must enter attempt_reconnect
        // (which emits ReconnectFailed on exhaustion) — never a silent return.
        assert_eq!(bind_failure_action(true), BindFailureAction::Reconnect);
    }

    #[test]
    fn bind_failure_emits_disconnected_when_policy_disabled() {
        assert_eq!(bind_failure_action(false), BindFailureAction::Disconnect);
    }
}
