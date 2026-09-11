//! Stable data contracts shared by native and web clients.

mod error;
mod time;
mod trade;

pub use error::{ErrorCode, QueryError};
pub use time::{TimeUnit, Timestamp, format_beijing, format_utc, parse_time};
pub use trade::{Market, TradeAnchor, TradeQuery, TradeRow, TradeWindow, TradeWindowMeta};
