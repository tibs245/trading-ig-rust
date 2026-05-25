//! Pure TLCP frame parser — no I/O, no async.
//!
//! Lightstreamer uses a newline-delimited text protocol.  Each line is
//! either a server command (`OK`, `PROBE`, `LOOP`, …) or a subscription
//! update whose first pipe-delimited token is the item index.
//!
//! # Special tokens
//!
//! | Wire token | Meaning             |
//! | ---------- | ------------------- |
//! | `$`        | Empty string `""`   |
//! | `#`        | `null` / `None`     |
//! | *(absent)* | Unchanged from last update |
//!
//! Unchanged trailing fields are represented by [`FieldValue::Unchanged`].
//!
//! # Usage
//!
//! ```
//! use trading_ig::streaming::protocol::{Frame, parse_line};
//!
//! let frame = parse_line(b"1|100.5|101.0|$|#");
//! assert!(matches!(frame, Frame::Update { .. }));
//! ```

/// A parsed value for a single subscription field.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    /// A concrete non-null string value (possibly empty, decoded from `$`).
    Value(String),
    /// The field is explicitly `null` (wire token `#`).
    Null,
    /// The field was absent in this frame — callers should keep the previous
    /// value.
    Unchanged,
}

impl FieldValue {
    /// Return `Some(&str)` if this is a concrete non-null value, otherwise `None`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            FieldValue::Value(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Parse into an `f64`, returning `None` for null/unchanged/non-numeric values.
    pub fn parse_f64(&self) -> Option<f64> {
        self.as_str()?.parse().ok()
    }

    /// Parse into an `i64`, returning `None` for null/unchanged/non-numeric values.
    pub fn parse_i64(&self) -> Option<i64> {
        self.as_str()?.parse().ok()
    }

    /// Parse into a `bool` (`"true"`/`"false"`), returning `None` otherwise.
    pub fn parse_bool(&self) -> Option<bool> {
        match self.as_str()? {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }
    }
}

/// A decoded frame from the Lightstreamer streaming channel.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// A subscription update: item index (1-based) and field values.
    Update {
        /// 1-based subscription item index, as returned by the server's OK response.
        item_index: usize,
        /// Field values in the order declared at subscription time.
        fields: Vec<FieldValue>,
    },
    /// Server keep-alive; no action needed.
    Probe,
    /// Server asks client to call `bind_session` — reconnect is required.
    Loop,
    /// Session out-of-sync; all subscriptions must be re-registered.
    SyncError,
    /// Server signals a fatal error.
    Error {
        /// Error code from the server.
        code: String,
        /// Human-readable message.
        message: String,
    },
    /// Session has been terminated by the server.
    End {
        /// Optional server-supplied reason.
        cause: Option<String>,
    },
    /// Handshake acknowledgement; contains the assigned session ID.
    Ok {
        /// Lightstreamer session identifier.
        session_id: String,
    },
    /// A line that was not recognised (defensive: callers should ignore).
    Unknown(String),
}

/// Parse a single raw line (without its trailing newline) into a [`Frame`].
///
/// The `line` argument must not include the `\n` terminator.
#[must_use]
pub fn parse_line(line: &[u8]) -> Frame {
    let s = match std::str::from_utf8(line) {
        Ok(s) => s.trim_end_matches('\r'),
        Err(_) => return Frame::Unknown(String::from_utf8_lossy(line).into_owned()),
    };

    // --- Server commands ---
    if s == "PROBE" {
        return Frame::Probe;
    }
    if s == "LOOP" {
        return Frame::Loop;
    }
    if s == "SYNC ERROR" {
        return Frame::SyncError;
    }
    // OK response: "OK\nSessionId:...\nControlAddress:...\n..."
    // When streaming responses include multiple response lines, the session_id
    // comes on the line "SessionId:<id>" after the "OK".  But in practice the
    // server sends one logical block.  We handle the inline variant here; the
    // multi-line OK block is assembled by the connection layer.
    if s == "OK" {
        return Frame::Ok {
            session_id: String::new(),
        };
    }
    // "OK\r\n" in chunked; multi-line OK values are handled by parse_ok_block.
    if let Some(rest) = s.strip_prefix("SessionId:") {
        return Frame::Ok {
            session_id: rest.to_owned(),
        };
    }
    if let Some(code_rest) = s.strip_prefix("ERROR") {
        // ERROR\n<code>\n<message>  — again multi-line; we handle the inline case.
        let trimmed = code_rest.trim();
        let (code, message) = if let Some(idx) = trimmed.find('\n') {
            (trimmed[..idx].to_owned(), trimmed[idx + 1..].to_owned())
        } else {
            (trimmed.to_owned(), String::new())
        };
        return Frame::Error { code, message };
    }
    if let Some(rest) = s.strip_prefix("END") {
        let cause = {
            let trimmed = rest.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_owned())
            }
        };
        return Frame::End { cause };
    }

    // --- Update line: "<table>,<item>|<f1>|<f2>|..." (TLCP streaming form)
    //     or the bare "<item_index>|<f1>|..." form. The leading key is
    //     "<table>,<itemPos>"; we subscribe one item per table (LS_table =
    //     the registry index), so the routing key is the TABLE — the number
    //     before the comma. Parsing the whole "1,1" as a usize fails on the
    //     comma, which silently dropped EVERY update as `Unknown` (no ticks
    //     ever delivered despite data arriving).
    if let Some(pipe_pos) = s.find('|') {
        let index_str = &s[..pipe_pos];
        let table_str = index_str.split(',').next().unwrap_or(index_str);
        if let Ok(item_index) = table_str.parse::<usize>() {
            let fields_str = &s[pipe_pos + 1..];
            let fields = fields_str.split('|').map(decode_field).collect();
            return Frame::Update { item_index, fields };
        }
    }

    Frame::Unknown(s.to_owned())
}

/// Parse a complete multi-line OK block from `create_session` / `bind_session`.
///
/// The block is the entire text up to and including the blank line that
/// terminates the TLCP header.  Returns the session ID and any extra
/// key-value pairs.
///
/// ```
/// use trading_ig::streaming::protocol::parse_ok_block;
///
/// let block = "OK\r\nSessionId:S1abc\r\nControlAddress:srv1\r\n\r\n";
/// let (session_id, _rest) = parse_ok_block(block).unwrap();
/// assert_eq!(session_id, "S1abc");
/// ```
pub fn parse_ok_block(block: &str) -> Option<(String, Vec<(String, String)>)> {
    let mut lines = block.lines();
    let first = lines.next()?.trim();
    if first != "OK" {
        return None;
    }
    let mut session_id = String::new();
    let mut extras = Vec::new();
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            break;
        }
        if let Some(id) = line.strip_prefix("SessionId:") {
            id.clone_into(&mut session_id);
        } else if let Some((k, v)) = line.split_once(':') {
            extras.push((k.to_owned(), v.to_owned()));
        }
    }
    Some((session_id, extras))
}

/// Decode a single pipe-delimited field token.
///
/// Per the TLCP spec:
/// - `$`   → empty string
/// - `#`   → null
/// - `""`  (empty between pipes) → unchanged
/// - anything else → concrete value
fn decode_field(token: &str) -> FieldValue {
    match token {
        "$" => FieldValue::Value(String::new()),
        "#" => FieldValue::Null,
        "" => FieldValue::Unchanged,
        other => FieldValue::Value(other.to_owned()),
    }
}

/// Apply a new frame's fields on top of a running state vector, respecting
/// the "absent = unchanged" rule.
///
/// `state` is modified in-place.  If `state` is shorter than `fields`, it
/// is extended.
pub fn merge_fields(state: &mut Vec<Option<String>>, fields: &[FieldValue]) {
    if state.len() < fields.len() {
        state.resize(fields.len(), None);
    }
    for (i, fv) in fields.iter().enumerate() {
        match fv {
            FieldValue::Value(s) => state[i] = Some(s.clone()),
            FieldValue::Null => state[i] = None,
            FieldValue::Unchanged => { /* keep current */ }
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // parse_line – server commands
    // ------------------------------------------------------------------

    #[test]
    fn test_probe() {
        assert_eq!(parse_line(b"PROBE"), Frame::Probe);
    }

    #[test]
    fn test_loop() {
        assert_eq!(parse_line(b"LOOP"), Frame::Loop);
    }

    #[test]
    fn test_sync_error() {
        assert_eq!(parse_line(b"SYNC ERROR"), Frame::SyncError);
    }

    #[test]
    fn test_ok_bare() {
        assert_eq!(
            parse_line(b"OK"),
            Frame::Ok {
                session_id: String::new()
            }
        );
    }

    #[test]
    fn test_session_id_line() {
        assert_eq!(
            parse_line(b"SessionId:Sabc123"),
            Frame::Ok {
                session_id: "Sabc123".into()
            }
        );
    }

    #[test]
    fn test_error_bare() {
        let f = parse_line(b"ERROR42 bad connection");
        assert!(matches!(f, Frame::Error { .. }));
    }

    #[test]
    fn test_end_no_cause() {
        assert_eq!(parse_line(b"END"), Frame::End { cause: None });
    }

    #[test]
    fn test_end_with_cause() {
        assert_eq!(
            parse_line(b"END session expired"),
            Frame::End {
                cause: Some("session expired".into())
            }
        );
    }

    // ------------------------------------------------------------------
    // parse_line – update frames
    // ------------------------------------------------------------------

    #[test]
    fn test_simple_update() {
        let f = parse_line(b"1|100.5|101.0");
        assert_eq!(
            f,
            Frame::Update {
                item_index: 1,
                fields: vec![
                    FieldValue::Value("100.5".into()),
                    FieldValue::Value("101.0".into()),
                ]
            }
        );
    }

    #[test]
    fn test_table_item_update_key() {
        // TLCP streaming sends "<table>,<itemPos>|fields". The routing key is
        // the TABLE (before the comma) = the LS_table we subscribed with.
        // Regression: parsing "1,1" as a whole usize failed → every update was
        // dropped as Unknown → zero ticks delivered despite data arriving.
        let f = parse_line(b"1,1|100.5|101.0");
        assert_eq!(
            f,
            Frame::Update {
                item_index: 1,
                fields: vec![
                    FieldValue::Value("100.5".into()),
                    FieldValue::Value("101.0".into()),
                ]
            }
        );
        // A real IG chart-tick line (table 2) routes by its table.
        let real = parse_line(b"2,1|1.16458|1.16468|#|1|#|1779709103150");
        assert!(matches!(real, Frame::Update { item_index: 2, .. }));
    }

    #[test]
    fn test_empty_string_dollar() {
        let f = parse_line(b"1|$|101.0");
        assert_eq!(
            f,
            Frame::Update {
                item_index: 1,
                fields: vec![
                    FieldValue::Value(String::new()),
                    FieldValue::Value("101.0".into()),
                ]
            }
        );
    }

    #[test]
    fn test_null_hash() {
        let f = parse_line(b"1|#|101.0");
        assert_eq!(
            f,
            Frame::Update {
                item_index: 1,
                fields: vec![FieldValue::Null, FieldValue::Value("101.0".into()),]
            }
        );
    }

    #[test]
    fn test_unchanged_trailing_fields() {
        // Only the first two fields are present; remaining are "unchanged".
        let f = parse_line(b"2|BID_VALUE|OFR_VALUE");
        assert_eq!(
            f,
            Frame::Update {
                item_index: 2,
                fields: vec![
                    FieldValue::Value("BID_VALUE".into()),
                    FieldValue::Value("OFR_VALUE".into()),
                ]
            }
        );
        // If the subscriber expects 10 fields they should treat indices 2..9 as Unchanged.
    }

    #[test]
    fn test_update_with_empty_middle_field() {
        // "||" → middle field is empty string decoded from "$"? No — bare empty is
        // actually the "unchanged" token (absent between pipes).  Wire spec:
        // absent = between consecutive pipes with nothing in between.
        let f = parse_line(b"1|100||200");
        assert_eq!(
            f,
            Frame::Update {
                item_index: 1,
                fields: vec![
                    FieldValue::Value("100".into()),
                    FieldValue::Unchanged,
                    FieldValue::Value("200".into()),
                ]
            }
        );
    }

    #[test]
    fn test_update_all_special() {
        let f = parse_line(b"3|$|#|$");
        assert_eq!(
            f,
            Frame::Update {
                item_index: 3,
                fields: vec![
                    FieldValue::Value(String::new()),
                    FieldValue::Null,
                    FieldValue::Value(String::new()),
                ]
            }
        );
    }

    #[test]
    fn test_high_item_index() {
        let f = parse_line(b"42|val");
        assert_eq!(
            f,
            Frame::Update {
                item_index: 42,
                fields: vec![FieldValue::Value("val".into())]
            }
        );
    }

    #[test]
    fn test_crlf_stripped() {
        // parse_line trims trailing CR (caller already strips LF)
        let f = parse_line(b"PROBE\r");
        assert_eq!(f, Frame::Probe);
    }

    #[test]
    fn test_unknown_line() {
        let f = parse_line(b"SOMETHING_WEIRD");
        assert!(matches!(f, Frame::Unknown(_)));
    }

    // ------------------------------------------------------------------
    // parse_ok_block
    // ------------------------------------------------------------------

    #[test]
    fn test_parse_ok_block_basic() {
        let block = "OK\r\nSessionId:Sabc\r\nControlAddress:push.lightstreamer.com\r\n\r\n";
        let (sid, extras) = parse_ok_block(block).unwrap();
        assert_eq!(sid, "Sabc");
        assert!(extras.iter().any(|(k, _)| k == "ControlAddress"));
    }

    #[test]
    fn test_parse_ok_block_missing_ok() {
        assert!(parse_ok_block("ERROR\r\nSomething\r\n").is_none());
    }

    #[test]
    fn test_parse_ok_block_no_session_id() {
        let block = "OK\r\nControlAddress:push.ls.com\r\n\r\n";
        let (sid, _) = parse_ok_block(block).unwrap();
        assert_eq!(sid, "");
    }

    // ------------------------------------------------------------------
    // merge_fields
    // ------------------------------------------------------------------

    #[test]
    fn test_merge_fields_all_new() {
        let mut state: Vec<Option<String>> = vec![];
        merge_fields(
            &mut state,
            &[
                FieldValue::Value("100.0".into()),
                FieldValue::Value("101.0".into()),
            ],
        );
        assert_eq!(state, vec![Some("100.0".into()), Some("101.0".into())]);
    }

    #[test]
    fn test_merge_fields_unchanged_preserved() {
        let mut state = vec![Some("100.0".into()), Some("101.0".into())];
        merge_fields(
            &mut state,
            &[FieldValue::Unchanged, FieldValue::Value("102.0".into())],
        );
        assert_eq!(state, vec![Some("100.0".into()), Some("102.0".into())]);
    }

    #[test]
    fn test_merge_fields_null_clears() {
        let mut state = vec![Some("100.0".into()), Some("101.0".into())];
        merge_fields(&mut state, &[FieldValue::Null, FieldValue::Unchanged]);
        assert_eq!(state, vec![None, Some("101.0".into())]);
    }

    #[test]
    fn test_merge_fields_extends_state() {
        let mut state: Vec<Option<String>> = vec![Some("a".into())];
        merge_fields(
            &mut state,
            &[
                FieldValue::Unchanged,
                FieldValue::Value("b".into()),
                FieldValue::Value("c".into()),
            ],
        );
        assert_eq!(
            state,
            vec![Some("a".into()), Some("b".into()), Some("c".into())]
        );
    }

    // ------------------------------------------------------------------
    // FieldValue helpers
    // ------------------------------------------------------------------

    #[test]
    fn test_field_value_parse_f64() {
        assert_eq!(FieldValue::Value("1.5".into()).parse_f64(), Some(1.5));
        assert_eq!(FieldValue::Null.parse_f64(), None);
        assert_eq!(FieldValue::Unchanged.parse_f64(), None);
    }

    #[test]
    fn test_field_value_parse_i64() {
        assert_eq!(FieldValue::Value("42".into()).parse_i64(), Some(42));
    }

    #[test]
    fn test_field_value_parse_bool() {
        assert_eq!(FieldValue::Value("true".into()).parse_bool(), Some(true));
        assert_eq!(FieldValue::Value("false".into()).parse_bool(), Some(false));
        assert_eq!(FieldValue::Value("yes".into()).parse_bool(), None);
    }
}
