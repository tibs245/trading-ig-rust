//! Cross-cutting domain primitives.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::Error;

macro_rules! string_newtype {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            pub fn new(s: impl Into<String>) -> Self { Self(s.into()) }
            pub fn as_str(&self) -> &str { &self.0 }
            pub fn into_inner(self) -> String { self.0 }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self { Self(s) }
        }
        impl From<&str> for $name {
            fn from(s: &str) -> Self { Self(s.to_owned()) }
        }
        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str { &self.0 }
        }
    };
}

string_newtype! {
    /// IG market identifier (e.g. `"CS.D.GBPUSD.TODAY.IP"`).
    Epic
}
string_newtype! {
    /// Opaque deal identifier returned by IG when a position/order is created.
    DealId
}
string_newtype! {
    /// Client-side reference for tracking a deal request through the
    /// asynchronous confirmation flow.
    DealReference
}

/// ISO-4217 currency code.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Currency(pub String);

impl Currency {
    pub fn new(code: impl Into<String>) -> Self { Self(code.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for Currency {
    fn from(s: String) -> Self { Self(s) }
}

impl From<&str> for Currency {
    fn from(s: &str) -> Self { Self(s.to_owned()) }
}

impl AsRef<str> for Currency {
    fn as_ref(&self) -> &str { &self.0 }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Direction {
    Buy,
    Sell,
}

impl Direction {
    pub fn opposite(self) -> Self {
        match self {
            Self::Buy => Self::Sell,
            Self::Sell => Self::Buy,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderType {
    Limit,
    Market,
    Quote,
    Stop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TimeInForce {
    /// Good till cancelled (default for working orders).
    GoodTillCancelled,
    /// Good till the specified date.
    GoodTillDate,
    /// Execute and eliminate any unfilled remainder.
    ExecuteAndEliminate,
    /// Fill or kill.
    FillOrKill,
}

impl FromStr for Direction {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Error> {
        match s.to_ascii_uppercase().as_str() {
            "BUY" => Ok(Self::Buy),
            "SELL" => Ok(Self::Sell),
            other => Err(Error::InvalidInput(format!("unknown direction '{other}'"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_serialises_uppercase() {
        let s = serde_json::to_string(&Direction::Buy).unwrap();
        assert_eq!(s, "\"BUY\"");
        let d: Direction = serde_json::from_str("\"SELL\"").unwrap();
        assert_eq!(d, Direction::Sell);
    }

    #[test]
    fn epic_round_trips() {
        let e: Epic = serde_json::from_str("\"CS.D.GBPUSD.TODAY.IP\"").unwrap();
        assert_eq!(e.as_str(), "CS.D.GBPUSD.TODAY.IP");
        assert_eq!(serde_json::to_string(&e).unwrap(), "\"CS.D.GBPUSD.TODAY.IP\"");
    }
}
