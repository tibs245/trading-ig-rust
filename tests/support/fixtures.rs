use std::path::PathBuf;

/// Load a fixture file from `tests/fixtures/json/`.
///
/// Panics if the file is missing — fixtures are part of the test source tree
/// and a missing file is always a programming error.
pub fn load(rel_path: &str) -> String {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("fixtures");
    p.push("json");
    p.push(rel_path);
    std::fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("could not read fixture {}: {e}", p.display()))
}

/// Same as [`load`] but parses to a `serde_json::Value` so callers can patch
/// fields before mounting (e.g. inject a fresh deal reference).
pub fn load_json(rel_path: &str) -> serde_json::Value {
    serde_json::from_str(&load(rel_path)).expect("fixture must be valid JSON")
}
