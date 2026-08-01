use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// CLI 与业务层支持的 Binance 账户类别。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AccountType {
    Spot,
    Funding,
    Futures,
}

impl fmt::Display for AccountType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spot => write!(f, "spot"),
            Self::Funding => write!(f, "funding"),
            Self::Futures => write!(f, "futures"),
        }
    }
}

impl FromStr for AccountType {
    type Err = ParseAccountTypeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "spot" => Ok(Self::Spot),
            "funding" => Ok(Self::Funding),
            "futures" => Ok(Self::Futures),
            _ => Err(ParseAccountTypeError(value.to_string())),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid account type '{0}', expected spot, funding, or futures")]
pub struct ParseAccountTypeError(String);
