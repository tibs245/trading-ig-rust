//! Date/time conversions for the various IG API formats.
//!
//! IG uses three input formats depending on the endpoint version:
//!
//! | Version | Format                  | Example               |
//! | ------- | ----------------------- | --------------------- |
//! | v1      | `%Y:%m:%d-%H:%M:%S`     | `2014:12:15-00:00:00` |
//! | v2      | `%Y/%m/%d %H:%M:%S`     | `2014/12/15 00:00:00` |
//! | v3      | `%Y-%m-%dT%H:%M:%S`     | `2014-12-15T00:00:00` |
//!
//! Output formats vary even more: see [`docs/API_CATALOG.md`] for details.

use chrono::NaiveDateTime;

use crate::error::{Error, Result};

const FMT_V1: &str = "%Y:%m:%d-%H:%M:%S";
const FMT_V2: &str = "%Y/%m/%d %H:%M:%S";
const FMT_V3: &str = "%Y-%m-%dT%H:%M:%S";

/// Which endpoint version the formatter should target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiVersion {
    V1,
    V2,
    V3,
}

/// Format a `NaiveDateTime` for use as an IG request parameter.
pub fn format(dt: NaiveDateTime, version: ApiVersion) -> String {
    let fmt = match version {
        ApiVersion::V1 => FMT_V1,
        ApiVersion::V2 => FMT_V2,
        ApiVersion::V3 => FMT_V3,
    };
    dt.format(fmt).to_string()
}

/// Parse a `NaiveDateTime` from an IG response, trying the version-specific
/// format and falling back to ISO-8601.
pub fn parse(s: &str, version: ApiVersion) -> Result<NaiveDateTime> {
    let primary = match version {
        ApiVersion::V1 => FMT_V1,
        ApiVersion::V2 => FMT_V2,
        ApiVersion::V3 => FMT_V3,
    };
    NaiveDateTime::parse_from_str(s, primary)
        .or_else(|_| NaiveDateTime::parse_from_str(s, FMT_V3))
        .map_err(|e| Error::InvalidInput(format!("could not parse '{s}': {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn dt() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2014, 12, 15)
            .unwrap()
            .and_hms_opt(9, 30, 45)
            .unwrap()
    }

    #[test]
    fn formats_per_version() {
        assert_eq!(format(dt(), ApiVersion::V1), "2014:12:15-09:30:45");
        assert_eq!(format(dt(), ApiVersion::V2), "2014/12/15 09:30:45");
        assert_eq!(format(dt(), ApiVersion::V3), "2014-12-15T09:30:45");
    }

    #[test]
    fn parses_each_format() {
        assert_eq!(parse("2014:12:15-09:30:45", ApiVersion::V1).unwrap(), dt());
        assert_eq!(parse("2014/12/15 09:30:45", ApiVersion::V2).unwrap(), dt());
        assert_eq!(parse("2014-12-15T09:30:45", ApiVersion::V3).unwrap(), dt());
    }

    #[test]
    fn parse_falls_back_to_iso() {
        // V1 reader still accepts an ISO timestamp (common in mixed responses).
        assert_eq!(parse("2014-12-15T09:30:45", ApiVersion::V1).unwrap(), dt());
    }
}
