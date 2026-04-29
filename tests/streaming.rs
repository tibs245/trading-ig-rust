//! Integration tests for the streaming module.
//!
//! These tests are gated behind the `stream` feature:
//! ```
//! cargo test --features stream --test streaming
//! ```
//!
//! They exercise the **public** streaming API surface — protocol parsing and
//! the typed event structs.  Lower-level registry and dispatch tests live as
//! unit tests in `src/streaming/subscription.rs` where they have access to
//! `pub(crate)` types.

#![cfg(feature = "stream")]

use trading_ig::streaming::events::{CandleScale, MarketUpdate};
use trading_ig::streaming::protocol::{FieldValue, Frame, parse_line, parse_ok_block};

// ---------------------------------------------------------------------------
// Protocol parser round-trip tests (full wire-level)
// ---------------------------------------------------------------------------

#[test]
fn parse_market_update_line() {
    // Simulate a real IG MARKET update line.
    let line = b"1|1.08500|1.08510|1.09000|1.08000|1.08505|0.00005|0.00462|12:34:56|0|TRADEABLE";
    let frame = parse_line(line);
    match frame {
        Frame::Update { item_index, fields } => {
            assert_eq!(item_index, 1);
            assert_eq!(fields.len(), 10);
            assert_eq!(fields[0].as_str(), Some("1.08500")); // BID
            assert_eq!(fields[1].as_str(), Some("1.08510")); // OFFER
            assert_eq!(fields[9].as_str(), Some("TRADEABLE")); // MARKET_STATE
        }
        other => panic!("expected Update, got {other:?}"),
    }
}

#[test]
fn parse_update_with_null_and_empty_fields() {
    let line = b"2|#|$|100.0";
    let frame = parse_line(line);
    match frame {
        Frame::Update { item_index, fields } => {
            assert_eq!(item_index, 2);
            assert_eq!(fields[0], FieldValue::Null);
            assert_eq!(fields[1], FieldValue::Value(String::new()));
            assert_eq!(fields[2], FieldValue::Value("100.0".into()));
        }
        other => panic!("expected Update, got {other:?}"),
    }
}

#[test]
fn parse_update_unchanged_trailing_fields() {
    // Only 2 of 10 fields sent — the rest should be treated as Unchanged by callers.
    let line = b"1|1.23456|1.23460";
    let frame = parse_line(line);
    match frame {
        Frame::Update { item_index, fields } => {
            assert_eq!(item_index, 1);
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0], FieldValue::Value("1.23456".into()));
            assert_eq!(fields[1], FieldValue::Value("1.23460".into()));
        }
        other => panic!("expected Update, got {other:?}"),
    }
}

#[test]
fn parse_unchanged_middle_field() {
    // Middle field absent (two consecutive pipes) → Unchanged.
    let line = b"1|100||200";
    let frame = parse_line(line);
    match frame {
        Frame::Update { item_index, fields } => {
            assert_eq!(item_index, 1);
            assert_eq!(fields[0], FieldValue::Value("100".into()));
            assert_eq!(fields[1], FieldValue::Unchanged);
            assert_eq!(fields[2], FieldValue::Value("200".into()));
        }
        other => panic!("expected Update, got {other:?}"),
    }
}

#[test]
fn server_commands_round_trip() {
    assert_eq!(parse_line(b"PROBE"), Frame::Probe);
    assert_eq!(parse_line(b"LOOP"), Frame::Loop);
    assert_eq!(parse_line(b"SYNC ERROR"), Frame::SyncError);
    assert!(matches!(parse_line(b"END"), Frame::End { cause: None }));
    assert!(
        matches!(parse_line(b"END reason"), Frame::End { cause: Some(ref s) } if s == "reason")
    );
}

#[test]
fn ok_block_round_trip() {
    let block = "OK\r\nSessionId:Sabc123\r\nControlAddress:push.lightstreamer.com\r\n\r\n";
    let (sid, extras) = parse_ok_block(block).unwrap();
    assert_eq!(sid, "Sabc123");
    assert!(extras.iter().any(|(k, _)| k == "ControlAddress"));
}

// ---------------------------------------------------------------------------
// Typed event construction from raw fields
// ---------------------------------------------------------------------------

#[test]
fn market_update_from_raw_all_fields() {
    let state: Vec<Option<String>> = vec![
        Some("1.08500".into()),   // BID
        Some("1.08510".into()),   // OFFER
        Some("1.09000".into()),   // HIGH
        Some("1.08000".into()),   // LOW
        Some("1.08505".into()),   // MID_OPEN
        Some("0.00005".into()),   // CHANGE
        Some("0.00462".into()),   // CHANGE_PCT
        Some("12:34:56".into()),  // UPDATE_TIME
        Some("0".into()),         // MARKET_DELAY
        Some("TRADEABLE".into()), // MARKET_STATE
    ];
    let update = MarketUpdate::from_raw("CS.D.EURUSD.CFD.IP", &state);
    assert_eq!(update.epic, "CS.D.EURUSD.CFD.IP");
    assert_eq!(update.bid, Some(1.085));
    assert_eq!(update.offer, Some(1.0851));
    assert_eq!(update.high, Some(1.09));
    assert_eq!(update.market_delay, Some(false));
    assert_eq!(update.market_state.as_deref(), Some("TRADEABLE"));
}

#[test]
fn market_update_from_raw_null_fields() {
    let state: Vec<Option<String>> = vec![None; 10];
    let update = MarketUpdate::from_raw("TEST", &state);
    assert_eq!(update.bid, None);
    assert_eq!(update.offer, None);
    assert_eq!(update.market_state, None);
    assert_eq!(update.market_delay, None);
}

#[test]
fn market_update_market_delay_truthy() {
    // MARKET_DELAY = "1" means delayed quotes.
    let mut state: Vec<Option<String>> = vec![None; 10];
    state[8] = Some("1".into()); // MARKET_DELAY
    let update = MarketUpdate::from_raw("TEST", &state);
    assert_eq!(update.market_delay, Some(true));
}

#[test]
fn candle_scale_display() {
    assert_eq!(CandleScale::OneMinute.as_str(), "1MINUTE");
    assert_eq!(CandleScale::FiveMinute.as_str(), "5MINUTE");
    assert_eq!(CandleScale::Hour.as_str(), "HOUR");
    assert_eq!(CandleScale::OneMinute.to_string(), "1MINUTE");
}

#[test]
fn field_value_helpers() {
    assert_eq!(FieldValue::Value("1.23".into()).parse_f64(), Some(1.23));
    assert_eq!(FieldValue::Value("42".into()).parse_i64(), Some(42));
    assert_eq!(FieldValue::Value("true".into()).parse_bool(), Some(true));
    assert_eq!(FieldValue::Null.parse_f64(), None);
    assert_eq!(FieldValue::Unchanged.parse_f64(), None);
}
