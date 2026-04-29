# Time formats

IG uses three different date/time formats depending on the endpoint
version. **Don't reimplement this** — use the helpers in `src/time.rs`.

## Input formats per version

| Version | Format               | Example                |
| ------- | -------------------- | ---------------------- |
| v1      | `%Y:%m:%d-%H:%M:%S`  | `2014:12:15-09:30:45`  |
| v2      | `%Y/%m/%d %H:%M:%S`  | `2014/12/15 09:30:45`  |
| v3      | `%Y-%m-%dT%H:%M:%S`  | `2014-12-15T09:30:45`  |

## Output formats

Output formats vary even within a single response:

- v3 prices include both `snapshotTime` (v2-shaped) and
  `snapshotTimeUTC` (ISO-8601).
- v3 activity / transactions use ISO-8601 (`%Y-%m-%dT%H:%M:%S`).
- v1/v2 endpoints use their input format on output too.

## Helpers

```rust
use crate::time::{ApiVersion, format, parse};

let s = format(dt, ApiVersion::V2);            // "2014/12/15 09:30:45"
let dt = parse(&s, ApiVersion::V2)?;
```

`parse` falls back to ISO-8601 if the version-specific format fails —
useful when an endpoint returns mixed shapes.

## In serde models

For response fields, prefer `chrono::NaiveDateTime` and add a custom
`#[serde(deserialize_with = "...")]` only when the wire format isn't
the chrono default. For most v3 responses, the default ISO-8601
deserializer works.

For v1/v2 fields where the wire format is non-standard, write a small
helper module:

```rust
mod ig_v2_dt {
    use chrono::NaiveDateTime;
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(dt: &NaiveDateTime, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&crate::time::format(*dt, crate::time::ApiVersion::V2))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<NaiveDateTime, D::Error> {
        let s = String::deserialize(d)?;
        crate::time::parse(&s, crate::time::ApiVersion::V2)
            .map_err(serde::de::Error::custom)
    }
}

#[serde(with = "ig_v2_dt")]
pub created_date: NaiveDateTime,
```

Place these helpers next to the model struct that uses them, or move
them to `src/time.rs` if reused across domains.
