//! Lightstreamer session management: `create_session`, `bind_session`,
//! `control` (subscribe / unsubscribe), and the read-loop task.
//!
//! This layer owns the raw HTTP interaction with the Lightstreamer server
//! using `reqwest`'s streaming `chunk()` API.  Subscribers are managed by
//! the [`Registry`] in `subscription.rs`.

use std::collections::HashMap;

use reqwest::Client;
use tokio::sync::{mpsc, watch};
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

impl LsConnection {
    /// Open a new Lightstreamer session via `create_session.txt`.
    ///
    /// Returns the connection object.  A background task is spawned that
    /// reads frames and dispatches them to registered subscribers.
    /// Sending `true` on `shutdown_tx` stops the read-loop.
    pub(crate) async fn create(params: CreateParams) -> Result<Self> {
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

        let conn = LsConnection {
            client: client.clone(),
            endpoint: endpoint.clone(),
            username: username.clone(),
            password: password.clone(),
            session_id: session_id.clone(),
            control_address: control_address.clone(),
        };

        // Spawn the background read-loop.
        let registry2 = registry.clone();
        let conn2 = LsConnection {
            client,
            endpoint,
            username,
            password,
            session_id,
            control_address,
        };
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

    /// Send a `control.txt` request (subscribe / unsubscribe).
    pub(crate) async fn control(
        &self,
        op: &str,
        item_index: usize,
        item_name: &str,
        fields: &str,
        mode: &str,
    ) -> Result<()> {
        let base = self.control_base_url();
        let url = format!("{base}/lightstreamer/control.txt");

        let item_index_str = item_index.to_string();
        let mut params = HashMap::new();
        params.insert("LS_session", self.session_id.as_str());
        params.insert("LS_op", op);
        params.insert("LS_table", item_index_str.as_str());
        params.insert("LS_id", item_name);
        params.insert("LS_schema", fields);
        params.insert("LS_mode", mode);

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
                "Lightstreamer control failed ({status}): {}",
                String::from_utf8_lossy(&body)
            )));
        }
        Ok(())
    }

    /// Send a `control.txt` unsubscribe for the given item index.
    pub(crate) async fn unsubscribe(&self, item_index: usize) -> Result<()> {
        let base = self.control_base_url();
        let url = format!("{base}/lightstreamer/control.txt");
        let item_index_str = item_index.to_string();
        let mut params = HashMap::new();
        params.insert("LS_session", self.session_id.as_str());
        params.insert("LS_op", "delete");
        params.insert("LS_table", item_index_str.as_str());

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
                "Lightstreamer unsubscribe failed ({status}): {}",
                String::from_utf8_lossy(&body)
            )));
        }
        Ok(())
    }

    /// Open a `bind_session.txt` connection and return the streaming response.
    async fn bind(&self) -> Result<LsStream> {
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
    async fn create_session(&self) -> Result<(LsConnection, LsStream)> {
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
    mut conn: LsConnection,
    policy: AutoReconnect,
    event_tx: Option<mpsc::Sender<StreamingEvent>>,
    session_handle: SessionHandle,
) {
    loop {
        match drain_stream(&mut stream, &registry, &mut conn).await {
            DrainOutcome::Rebind => {
                // ---- Rebind path (existing LOOP behaviour) ----
                debug!("attempting bind_session");
                match conn.bind().await {
                    Ok(new_stream) => {
                        stream = new_stream;
                        resubscribe_all(&conn, &registry).await;
                        info!(
                            session_id = %conn.session_id,
                            "Lightstreamer session rebound"
                        );
                    }
                    Err(e) => {
                        error!(error = %e, "Lightstreamer bind_session failed; giving up");
                        return;
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
                    &mut conn,
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
    conn: &mut LsConnection,
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

        // Refresh the session tokens so `create_session` uses fresh CST/XST.
        if let Err(e) = session_handle.session_api().login_v2().await {
            last_error = format!("session refresh failed: {e}");
            warn!(
                attempt,
                error = %e,
                "Lightstreamer auto-reconnect: token refresh failed"
            );
            continue; // back off and retry
        }

        // Read the new password from the refreshed streaming surface.
        // login_v2() populates `tokens.streaming` directly.
        let state = session_handle.session.snapshot().await;
        let Some(streaming) = state.tokens.streaming.as_ref() else {
            "no streaming tokens after login_v2".clone_into(&mut last_error);
            warn!(attempt, "Lightstreamer auto-reconnect: {}", last_error);
            continue;
        };
        let new_password = format!("CST-{}|XST-{}", streaming.cst, streaming.x_security_token);

        // Update the stored password so future reconnects use the new tokens.
        conn.password = new_password;

        // Open a new Lightstreamer session.
        match conn.create_session().await {
            Ok((new_conn, new_stream)) => {
                info!(
                    attempt,
                    session_id = %new_conn.session_id,
                    "Lightstreamer auto-reconnect: new session established"
                );
                *conn = new_conn;
                *stream = new_stream;

                // Re-subscribe all active subscriptions.
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
async fn resubscribe_all(conn: &LsConnection, registry: &Registry) {
    let subs = registry.snapshot_for_resubscribe();
    for (idx, name, fields, mode) in subs {
        if let Err(e) = conn.control("add", idx, &name, &fields, mode).await {
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
    conn: &mut LsConnection,
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

async fn handle_frame(frame: Frame, registry: &Registry, conn: &mut LsConnection) -> FrameAction {
    match frame {
        Frame::Update { item_index, fields } => {
            let alive = registry.apply_update(item_index, &fields);
            if !alive {
                registry.remove(item_index);
                // Best-effort unsubscribe — ignore errors.
                let _ = conn.unsubscribe(item_index).await;
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
            conn.session_id = session_id;
            FrameAction::Continue
        }
        Frame::Ok { .. } | Frame::Unknown(_) => FrameAction::Continue,
    }
}
