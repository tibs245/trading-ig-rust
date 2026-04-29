//! Lightstreamer session management: `create_session`, `bind_session`,
//! `control` (subscribe / unsubscribe), and the read-loop task.
//!
//! This layer owns the raw HTTP interaction with the Lightstreamer server
//! using `reqwest`'s streaming `chunk()` API.  Subscribers are managed by
//! the [`Registry`] in `subscription.rs`.

use std::collections::HashMap;

use reqwest::Client;
use tokio::sync::watch;
use tracing::{debug, error, info, warn};

use crate::error::{Error, Result};
use crate::streaming::protocol::{Frame, parse_line, parse_ok_block};
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
    /// Lightstreamer username (IG account ID) — retained for potential
    /// session re-create after a fatal server-side termination.
    #[allow(dead_code)]
    pub(crate) username: String,
    /// Lightstreamer password (`CST-…|XST-…`) — retained for potential
    /// session re-create after a fatal server-side termination.
    #[allow(dead_code)]
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
    pub(crate) async fn create(
        endpoint: &str,
        username: &str,
        password: &str,
        registry: Registry,
        shutdown_tx: watch::Sender<bool>,
    ) -> Result<Self> {
        let client = Client::builder().build().map_err(Error::Http)?;

        let url = format!("{endpoint}/lightstreamer/create_session.txt");
        debug!(%url, "opening Lightstreamer session");

        let mut params = HashMap::new();
        params.insert("LS_op2", "create");
        params.insert("LS_cid", "mgQkwtwdysogQz2BJ4Ji kOj2Bg");
        params.insert("LS_adapter_set", "DEFAULT");
        params.insert("LS_user", username);
        params.insert("LS_password", password);
        params.insert("LS_polling", "false");

        let resp = client
            .post(&url)
            .form(&params)
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
            endpoint: endpoint.to_owned(),
            username: username.to_owned(),
            password: password.to_owned(),
            session_id: session_id.clone(),
            control_address: control_address.clone(),
        };

        // Spawn the background read-loop.
        let registry2 = registry.clone();
        let conn2 = LsConnection {
            client,
            endpoint: endpoint.to_owned(),
            username: username.to_owned(),
            password: password.to_owned(),
            session_id,
            control_address,
        };
        let mut shutdown_rx = shutdown_tx.subscribe();
        tokio::spawn(async move {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    debug!("Lightstreamer read-loop: shutdown signal received");
                }
                () = read_loop(stream, registry2, conn2) => {}
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

/// Background task: consume a line-stream from Lightstreamer, parse frames,
/// and dispatch to the registry.  Handles `LOOP` by rebinding automatically.
async fn read_loop(mut stream: LsStream, registry: Registry, mut conn: LsConnection) {
    loop {
        // Drain the current stream until we need to rebind or terminate.
        let rebind = drain_stream(&mut stream, &registry, &mut conn).await;
        if !rebind {
            return;
        }

        // Attempt to rebind.
        debug!("attempting bind_session");
        match conn.bind().await {
            Ok(new_stream) => {
                stream = new_stream;
                // Re-subscribe all active subscriptions.
                let subs = registry.snapshot_for_resubscribe();
                for (idx, name, fields, mode) in subs {
                    if let Err(e) = conn.control("add", idx, &name, &fields, mode).await {
                        warn!(error = %e, "failed to re-subscribe {name} after rebind");
                    }
                }
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
}

/// Drain a stream until rebind is needed (`true`) or the session should terminate (`false`).
async fn drain_stream(stream: &mut LsStream, registry: &Registry, conn: &mut LsConnection) -> bool {
    loop {
        match stream.next_line().await {
            Some(Ok(line)) => {
                let frame = parse_line(line.as_bytes());
                match handle_frame(frame, registry, conn).await {
                    FrameAction::Continue => {}
                    FrameAction::Rebind => return true,
                    FrameAction::Terminate => return false,
                }
            }
            Some(Err(e)) => {
                error!(error = %e, "Lightstreamer stream error");
                return true;
            }
            None => {
                debug!("Lightstreamer stream EOF; will rebind");
                return true;
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
            FrameAction::Terminate
        }
        Frame::Ok { session_id } if !session_id.is_empty() => {
            debug!(%session_id, "Lightstreamer OK (bind acknowledged)");
            conn.session_id = session_id;
            FrameAction::Continue
        }
        Frame::Ok { .. } | Frame::Unknown(_) => FrameAction::Continue,
    }
}
