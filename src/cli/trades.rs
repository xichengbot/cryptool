use crate::domain::trade::{TradeAnchor, TradeWindowQuery};
use chrono::{DateTime, Utc};
use clap::Args;
use clap::error::ErrorKind;

/// 历史公开成交查询的共享窗口参数。
///
/// 用户必须提供 trade ID 或 RFC3339 UTC 时间之一作为锚点；`before` 和
/// `after` 控制锚点两侧包含的成交数量。
#[derive(Debug, Args)]
pub struct TradeWindowArgs {
    /// 原始成交 ID；与 `--time` 互斥。
    #[arg(long, conflicts_with = "time", required_unless_present = "time")]
    trade_id: Option<u64>,

    /// RFC3339 UTC 时间锚点（必须以 `Z` 结尾）；与 `--trade-id` 互斥。
    #[arg(
        long,
        conflicts_with = "trade_id",
        required_unless_present = "trade_id",
        value_parser = parse_utc_time,
    )]
    time: Option<DateTime<Utc>>,

    /// 锚点之前最多包含的成交数。
    #[arg(long, default_value_t = 20)]
    before: u16,

    /// 锚点之后最多包含的成交数。
    #[arg(long, default_value_t = 20)]
    after: u16,
}

/// 仅接受明确的 UTC 时间，避免本地时区导致历史成交窗口偏移。
fn parse_utc_time(value: &str) -> Result<DateTime<Utc>, String> {
    if !value.ends_with('Z') {
        return Err("time must be an RFC3339 UTC timestamp ending in Z".to_string());
    }

    DateTime::parse_from_rfc3339(value)
        .map(|time| time.with_timezone(&Utc))
        .map_err(|error| format!("invalid RFC3339 UTC timestamp: {error}"))
}

impl TradeWindowArgs {
    /// 复用完整查询构造逻辑验证 clap 参数组合。
    pub fn validate(&self) -> Result<(), clap::Error> {
        self.to_query("validation".to_string()).map(|_| ())
    }

    /// 将 CLI 锚点和窗口参数转换为经过范围验证的领域查询。
    ///
    /// `unreachable!` 依赖 clap 已强制两个锚点参数恰好出现一个。
    pub fn to_query(&self, symbol: String) -> Result<TradeWindowQuery, clap::Error> {
        let anchor = match (self.trade_id, self.time) {
            (Some(trade_id), None) => TradeAnchor::TradeId(trade_id),
            (None, Some(time)) => TradeAnchor::Time(time),
            _ => unreachable!("clap enforces exactly one trade anchor"),
        };

        TradeWindowQuery::new(symbol, anchor, self.before, self.after)
            .map_err(|message| clap::Error::raw(ErrorKind::ValueValidation, message))
    }
}
