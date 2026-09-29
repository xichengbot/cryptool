use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{ErrorCode, QueryError, Timestamp};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Market {
    Spot,
    Usdm,
}

impl Market {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Spot => "spot",
            Self::Usdm => "usdm",
        }
    }
}

impl fmt::Display for Market {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for Market {
    type Err = QueryError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "spot" => Ok(Self::Spot),
            "usdm" => Ok(Self::Usdm),
            _ => Err(invalid(format!(
                "invalid market '{input}', expected spot or usdm"
            ))),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value")]
pub enum TradeAnchor {
    Id(#[serde(with = "u64_string")] u64),
    Time(Timestamp),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TradeQuery {
    pub market: Market,
    pub symbol: String,
    pub anchor: TradeAnchor,
    pub before: u16,
    pub after: u16,
}

impl TradeQuery {
    pub fn new(
        market: Market,
        symbol: impl Into<String>,
        anchor: TradeAnchor,
        before: u16,
        after: u16,
    ) -> Result<Self, QueryError> {
        let query = Self {
            market,
            symbol: symbol.into().trim().to_ascii_uppercase(),
            anchor,
            before,
            after,
        };
        query.validate()?;
        Ok(query)
    }

    pub fn validate(&self) -> Result<(), QueryError> {
        if self.symbol.is_empty()
            || self.symbol.len() > 40
            || !self
                .symbol
                .chars()
                .all(|character| character.is_alphanumeric() || character == '_')
        {
            return Err(invalid(
                "symbol must contain 1..=40 bytes of letters, numbers, or '_'",
            ));
        }

        if let TradeAnchor::Time(timestamp) = self.anchor
            && timestamp.to_micros()? < 0
        {
            return Err(invalid("trade query timestamps must be non-negative"));
        }

        self.limit()?;
        Ok(())
    }

    pub fn limit(&self) -> Result<u16, QueryError> {
        let limit = u32::from(self.before) + u32::from(self.after) + 1;
        if limit > 1_000 {
            return Err(invalid("before + after + 1 must be at most 1000"));
        }
        u16::try_from(limit).map_err(|_| invalid("trade query limit is out of range"))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TradeRow {
    pub id: String,
    pub price: String,
    pub qty: String,
    pub quote_qty: String,
    pub timestamp: Timestamp,
    pub maker: bool,
    pub best_match: Option<bool>,
    #[serde(default)]
    pub is_rpi_trade: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TradeWindow {
    pub items: Vec<TradeRow>,
    pub meta: TradeWindowMeta,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TradeWindowMeta {
    pub query: TradeQuery,
    pub anchor_id: Option<String>,
    pub requested: u16,
    pub received: usize,
    pub actual_before: usize,
    pub actual_after: usize,
    pub boundary: Option<String>,
    pub upstream_calls: usize,
    pub partial: bool,
}

fn invalid(message: impl Into<String>) -> QueryError {
    QueryError::new(ErrorCode::InvalidInput, message)
}

mod u64_string {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}
