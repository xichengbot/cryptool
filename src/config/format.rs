use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Output format shared by native configuration and the CLI adapter.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Table => "table",
            Self::Json => "json",
        })
    }
}

impl FromStr for OutputFormat {
    type Err = ParseOutputFormatError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "table" => Ok(Self::Table),
            "json" => Ok(Self::Json),
            _ => Err(ParseOutputFormatError(input.to_string())),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid output format '{0}', expected table or json")]
pub struct ParseOutputFormatError(String);
