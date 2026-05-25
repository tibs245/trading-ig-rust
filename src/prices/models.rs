//! Request and response types for the historical prices domain.

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// Candlestick resolution supported by the IG prices API.
///
/// IG's wire values use an underscore before the numeric part
/// (`MINUTE_15`, `HOUR_2`), which `SCREAMING_SNAKE_CASE` does NOT emit for
/// letter→digit boundaries (it would produce `MINUTE15`, which IG rejects
/// with `Invalid parameter=MINUTE15`). So every numbered variant carries an
/// explicit `#[serde(rename = ...)]`. The unnumbered ones (`SECOND`,
/// `MINUTE`, `HOUR`, `DAY`, `WEEK`, `MONTH`) match the default rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Resolution {
    /// One-second bars.
    Second,
    /// One-minute bars.
    Minute,
    /// Two-minute bars.
    #[serde(rename = "MINUTE_2")]
    Minute2,
    /// Three-minute bars.
    #[serde(rename = "MINUTE_3")]
    Minute3,
    /// Five-minute bars.
    #[serde(rename = "MINUTE_5")]
    Minute5,
    /// Ten-minute bars.
    #[serde(rename = "MINUTE_10")]
    Minute10,
    /// Fifteen-minute bars.
    #[serde(rename = "MINUTE_15")]
    Minute15,
    /// Thirty-minute bars.
    #[serde(rename = "MINUTE_30")]
    Minute30,
    /// One-hour bars.
    Hour,
    /// Two-hour bars.
    #[serde(rename = "HOUR_2")]
    Hour2,
    /// Three-hour bars.
    #[serde(rename = "HOUR_3")]
    Hour3,
    /// Four-hour bars.
    #[serde(rename = "HOUR_4")]
    Hour4,
    /// Daily bars.
    Day,
    /// Weekly bars.
    Week,
    /// Monthly bars.
    Month,
}

impl std::fmt::Display for Resolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Serialise to the IG wire string.
        let s = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(ToOwned::to_owned))
            .unwrap_or_else(|| format!("{self:?}").to_ascii_uppercase());
        f.write_str(&s)
    }
}

/// Request parameters for `GET /prices/{epic}` v3.
///
/// All fields are optional. Build from `Default` and set only what you need:
///
/// ```ignore
/// let req = HistoricalPricesRequest {
///     resolution: Some(Resolution::Hour),
///     from: Some(start),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct HistoricalPricesRequest {
    /// Bar resolution. Defaults to `MINUTE` server-side when not supplied.
    pub resolution: Option<Resolution>,
    /// Start of the date range (ISO format sent to the API).
    pub from: Option<NaiveDateTime>,
    /// End of the date range (ISO format sent to the API).
    pub to: Option<NaiveDateTime>,
    /// Maximum number of data points to return.
    pub max: Option<u32>,
    /// Number of points per page (default 20 server-side).
    pub page_size: Option<u32>,
    /// Page number to fetch (1-based). Leave `None` for page 1.
    pub page_number: Option<u32>,
}

// ── Response types ──────────────────────────────────────────────────────────

/// Bid/ask/last-traded triple for one side of a price candle.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceCandle {
    /// Bid price.
    pub bid: Option<f64>,
    /// Ask (offer) price.
    pub ask: Option<f64>,
    /// Last traded price (equity markets only).
    pub last_traded: Option<f64>,
}

/// One OHLC bar returned by the IG historical-prices API.
///
/// `Deserialize` is hand-written (not derived) because IG does not always
/// include `snapshotTimeUTC` in the response (observed on the v3 endpoint
/// for some epics — a missing field would otherwise fail the whole fetch
/// with `missing field snapshotTimeUTC`). When it is absent we fall back to
/// parsing `snapshotTime` (IG's `YYYY/MM/DD HH:MM:SS` v2 string, treated as
/// UTC). The public field stays a non-optional `NaiveDateTime` so consumers
/// (`DataFrame` conversion, downstream candle mappers) are unaffected.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PricePoint {
    /// Snapshot time in IG's v2 string format (`YYYY/MM/DD HH:MM:SS`).
    pub snapshot_time: String,
    /// Snapshot time in UTC.
    ///
    /// IG uses the non-standard capitalisation `snapshotTimeUTC` (capital UTC)
    /// in the JSON payload. When the field is omitted by IG, this is derived
    /// from [`Self::snapshot_time`] on deserialize (see the type doc).
    #[serde(rename = "snapshotTimeUTC")]
    pub snapshot_time_utc: NaiveDateTime,
    /// Opening prices for this bar.
    pub open_price: PriceCandle,
    /// Closing prices for this bar.
    pub close_price: PriceCandle,
    /// High prices for this bar.
    pub high_price: PriceCandle,
    /// Low prices for this bar.
    pub low_price: PriceCandle,
    /// Last-traded volume (equity markets only).
    pub last_traded_volume: Option<u64>,
}

impl<'de> Deserialize<'de> for PricePoint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        /// IG's v2 snapshot string format, used as the UTC fallback when
        /// `snapshotTimeUTC` is absent from the payload.
        const V2_FMT: &str = "%Y/%m/%d %H:%M:%S";

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Raw {
            snapshot_time: String,
            #[serde(rename = "snapshotTimeUTC", default)]
            snapshot_time_utc: Option<NaiveDateTime>,
            open_price: PriceCandle,
            close_price: PriceCandle,
            high_price: PriceCandle,
            low_price: PriceCandle,
            #[serde(default)]
            last_traded_volume: Option<u64>,
        }

        let raw = Raw::deserialize(deserializer)?;
        let snapshot_time_utc = match raw.snapshot_time_utc {
            Some(t) => t,
            None => NaiveDateTime::parse_from_str(&raw.snapshot_time, V2_FMT)
                .map_err(serde::de::Error::custom)?,
        };
        Ok(PricePoint {
            snapshot_time: raw.snapshot_time,
            snapshot_time_utc,
            open_price: raw.open_price,
            close_price: raw.close_price,
            high_price: raw.high_price,
            low_price: raw.low_price,
            last_traded_volume: raw.last_traded_volume,
        })
    }
}

/// Pagination state embedded in every prices response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageData {
    /// Current page number (1-based).
    pub page_number: u32,
    /// Number of points per page.
    pub page_size: u32,
    /// Total number of available pages.
    pub total_pages: u32,
}

/// API allowance data returned with each prices response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceAllowance {
    /// Number of price data points remaining in the current allowance period.
    pub remaining_allowance: u32,
    /// Total price data points permitted per allowance period.
    pub total_allowance: u32,
    /// Seconds until the current allowance resets.
    pub allowance_expiry: u64,
}

/// Metadata block returned alongside every historical-prices response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PricesMetadata {
    /// Pagination state.
    pub page_data: PageData,
    /// Current API allowance.
    pub allowance: PriceAllowance,
}

/// Top-level historical-prices response (v2 and v3 share this shape).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalPrices {
    /// Instrument category (e.g. `"CURRENCIES"`, `"INDICES"`).
    pub instrument_type: String,
    /// Individual OHLC bars.
    pub prices: Vec<PricePoint>,
    /// Pagination and allowance metadata.
    pub metadata: PricesMetadata,
}

// ---------------------------------------------------------------------------
// Polars conversion
// ---------------------------------------------------------------------------

#[cfg(feature = "polars")]
impl crate::dataframe::IntoDataFrame for HistoricalPrices {
    /// Convert historical prices into a `polars::prelude::DataFrame` with one row per
    /// [`PricePoint`].
    ///
    /// Column layout:
    ///
    /// | column                   | dtype      | nullable |
    /// | ------------------------ | ---------- | -------- |
    /// | `snapshot_time`          | `Utf8`     | no       |
    /// | `snapshot_time_utc`      | `Datetime` | no       |
    /// | `open_bid`               | `Float64`  | yes      |
    /// | `open_ask`               | `Float64`  | yes      |
    /// | `high_bid`               | `Float64`  | yes      |
    /// | `high_ask`               | `Float64`  | yes      |
    /// | `low_bid`                | `Float64`  | yes      |
    /// | `low_ask`                | `Float64`  | yes      |
    /// | `close_bid`              | `Float64`  | yes      |
    /// | `close_ask`              | `Float64`  | yes      |
    /// | `last_traded_volume`     | `UInt64`   | yes      |
    fn to_dataframe(&self) -> crate::Result<polars::prelude::DataFrame> {
        use polars::prelude::*;

        let snapshot_time: Vec<&str> = self
            .prices
            .iter()
            .map(|p| p.snapshot_time.as_str())
            .collect();
        let snapshot_time_utc: Vec<NaiveDateTime> =
            self.prices.iter().map(|p| p.snapshot_time_utc).collect();
        let open_bid: Vec<Option<f64>> = self.prices.iter().map(|p| p.open_price.bid).collect();
        let open_ask: Vec<Option<f64>> = self.prices.iter().map(|p| p.open_price.ask).collect();
        let high_bid: Vec<Option<f64>> = self.prices.iter().map(|p| p.high_price.bid).collect();
        let high_ask: Vec<Option<f64>> = self.prices.iter().map(|p| p.high_price.ask).collect();
        let low_bid: Vec<Option<f64>> = self.prices.iter().map(|p| p.low_price.bid).collect();
        let low_ask: Vec<Option<f64>> = self.prices.iter().map(|p| p.low_price.ask).collect();
        let close_bid: Vec<Option<f64>> = self.prices.iter().map(|p| p.close_price.bid).collect();
        let close_ask: Vec<Option<f64>> = self.prices.iter().map(|p| p.close_price.ask).collect();
        let last_traded_volume: Vec<Option<u64>> =
            self.prices.iter().map(|p| p.last_traded_volume).collect();

        let snapshot_time_utc_series = Series::new("snapshot_time_utc".into(), snapshot_time_utc);

        DataFrame::new(vec![
            Column::new("snapshot_time".into(), snapshot_time),
            snapshot_time_utc_series.into(),
            Column::new("open_bid".into(), open_bid),
            Column::new("open_ask".into(), open_ask),
            Column::new("high_bid".into(), high_bid),
            Column::new("high_ask".into(), high_ask),
            Column::new("low_bid".into(), low_bid),
            Column::new("low_ask".into(), low_ask),
            Column::new("close_bid".into(), close_bid),
            Column::new("close_ask".into(), close_ask),
            Column::new("last_traded_volume".into(), last_traded_volume),
        ])
        .map_err(|e| crate::Error::Config(format!("polars conversion failed: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Resolution wire format (regression: IG rejects MINUTE15) ---
    #[test]
    fn resolution_serialises_numbered_with_underscore() {
        let cases = [
            (Resolution::Second, "SECOND"),
            (Resolution::Minute, "MINUTE"),
            (Resolution::Minute2, "MINUTE_2"),
            (Resolution::Minute5, "MINUTE_5"),
            (Resolution::Minute10, "MINUTE_10"),
            (Resolution::Minute15, "MINUTE_15"),
            (Resolution::Minute30, "MINUTE_30"),
            (Resolution::Hour, "HOUR"),
            (Resolution::Hour2, "HOUR_2"),
            (Resolution::Hour4, "HOUR_4"),
            (Resolution::Day, "DAY"),
        ];
        for (res, wire) in cases {
            // serde JSON value
            assert_eq!(
                serde_json::to_value(res).unwrap().as_str().unwrap(),
                wire,
                "serde wire for {res:?}"
            );
            // Display (query-param path)
            assert_eq!(res.to_string(), wire, "Display for {res:?}");
        }
    }

    #[test]
    fn resolution_roundtrips_from_ig_wire() {
        let r: Resolution = serde_json::from_str("\"MINUTE_15\"").unwrap();
        assert_eq!(r, Resolution::Minute15);
    }

    // --- PricePoint: snapshotTimeUTC present vs absent ---
    fn price_json(with_utc: bool) -> String {
        let utc = if with_utc {
            r#""snapshotTimeUTC":"2024-06-03T14:00:00","#
        } else {
            ""
        };
        format!(
            r#"{{"snapshotTime":"2024/06/03 14:00:00",{utc}
               "openPrice":{{"bid":1.0,"ask":1.1,"lastTraded":null}},
               "closePrice":{{"bid":1.2,"ask":1.3,"lastTraded":null}},
               "highPrice":{{"bid":1.4,"ask":1.5,"lastTraded":null}},
               "lowPrice":{{"bid":0.9,"ask":1.0,"lastTraded":null}},
               "lastTradedVolume":42}}"#
        )
    }

    #[test]
    fn price_point_uses_snapshot_time_utc_when_present() {
        let p: PricePoint = serde_json::from_str(&price_json(true)).unwrap();
        assert_eq!(
            p.snapshot_time_utc,
            NaiveDateTime::parse_from_str("2024-06-03T14:00:00", "%Y-%m-%dT%H:%M:%S").unwrap()
        );
    }

    #[test]
    fn price_point_falls_back_to_snapshot_time_when_utc_absent() {
        // Regression: IG omitting snapshotTimeUTC must NOT fail the fetch.
        let p: PricePoint = serde_json::from_str(&price_json(false)).unwrap();
        assert_eq!(
            p.snapshot_time_utc,
            NaiveDateTime::parse_from_str("2024/06/03 14:00:00", "%Y/%m/%d %H:%M:%S").unwrap()
        );
        assert_eq!(p.snapshot_time, "2024/06/03 14:00:00");
        assert_eq!(p.last_traded_volume, Some(42));
    }
}
