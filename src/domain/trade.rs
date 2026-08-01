use std::fmt;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 委托方向。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

impl fmt::Display for OrderSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Buy => write!(f, "buy"),
            Self::Sell => write!(f, "sell"),
        }
    }
}

/// 委托类型。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum OrderType {
    Market,
    Limit,
}

impl fmt::Display for OrderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Market => write!(f, "market"),
            Self::Limit => write!(f, "limit"),
        }
    }
}

/// 委托生命周期状态。
///
/// `Unknown` 保留 Binance 将来新增的状态值，以维持响应兼容性。
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum OrderStatus {
    New,
    PartiallyFilled,
    Filled,
    Canceled,
    Rejected,
    Expired,
    Unknown(String),
}

/// 账户持仓方向。
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum PositionSide {
    Long,
    Short,
    Flat,
}

/// 标准化的账户持仓快照。
///
/// 价格和数量使用 Decimal，避免风控或 PnL 计算中出现浮点舍入误差。
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Position {
    pub symbol: String,
    pub side: PositionSide,
    pub quantity: Decimal,
    pub entry_price: Decimal,
    pub mark_price: Decimal,
    pub unrealized_pnl: Decimal,
}

/// 标准化的委托快照。
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Order {
    pub id: String,
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub price: Option<Decimal>,
    pub quantity: Decimal,
    pub executed_quantity: Decimal,
    pub status: OrderStatus,
}

/// 公开历史成交的领域表示。
///
/// `is_best_match` 与 `is_rpi_trade` 并非所有 Binance 市场都返回，因此
/// 使用 Option 保留“字段缺失”与 false 的区别。
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PublicTrade {
    pub id: u64,
    pub price: Decimal,
    pub quantity: Decimal,
    pub quote_quantity: Decimal,
    pub timestamp: DateTime<Utc>,
    pub is_buyer_maker: bool,
    pub is_best_match: Option<bool>,
    pub is_rpi_trade: Option<bool>,
}

/// 历史成交窗口的中心锚点。
///
/// 时间锚点必须先解析为最近成交 ID，才能请求按 trade ID 分页的历史接口。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TradeAnchor {
    TradeId(u64),
    Time(DateTime<Utc>),
}

/// 历史成交查询的领域参数。
///
/// 窗口总大小为 `before + after + 1`，并被限制在 Binance 支持的单次
/// 最大 500 条以内。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TradeWindowQuery {
    pub symbol: String,
    pub anchor: TradeAnchor,
    pub before: u16,
    pub after: u16,
}

impl TradeWindowQuery {
    /// 创建并验证查询窗口，防止生成超过 Binance 历史接口上限的请求。
    pub fn new(
        symbol: String,
        anchor: TradeAnchor,
        before: u16,
        after: u16,
    ) -> Result<Self, &'static str> {
        let limit = u32::from(before) + u32::from(after) + 1;
        if !(1..=500).contains(&limit) {
            return Err("before + after + 1 must be in 1..=500");
        }

        Ok(Self {
            symbol,
            anchor,
            before,
            after,
        })
    }

    /// 返回历史接口的起始 trade ID；下溢时饱和到零。
    pub fn start_id(&self) -> u64 {
        match self.anchor {
            TradeAnchor::TradeId(trade_id) => trade_id.saturating_sub(u64::from(self.before)),
            TradeAnchor::Time(_) => 0,
        }
    }

    /// 返回包含锚点在内的请求条数；`new` 已保证结果不超过 500。
    pub fn limit(&self) -> u16 {
        u16::try_from(u32::from(self.before) + u32::from(self.after) + 1)
            .expect("validated trade windows never exceed u16")
    }

    /// 用时间查询解析得到的 trade ID 替换锚点，同时保留窗口范围。
    pub fn with_trade_id(&self, trade_id: u64) -> Self {
        Self {
            symbol: self.symbol.clone(),
            anchor: TradeAnchor::TradeId(trade_id),
            before: self.before,
            after: self.after,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::{TradeAnchor, TradeWindowQuery};

    #[test]
    fn trade_id_window_saturates_at_zero_and_includes_anchor() {
        let query =
            TradeWindowQuery::new("BTCUSDT".to_string(), TradeAnchor::TradeId(3), 20, 20).unwrap();

        assert_eq!(query.start_id(), 0);
        assert_eq!(query.limit(), 41);
    }

    #[test]
    fn trade_id_window_uses_requested_before_and_after_counts() {
        let query = TradeWindowQuery::new(
            "BTCUSDT".to_string(),
            TradeAnchor::TradeId(6_533_409_332),
            20,
            20,
        )
        .unwrap();

        assert_eq!(query.start_id(), 6_533_409_312);
        assert_eq!(query.limit(), 41);
    }

    #[test]
    fn accepts_the_shared_500_trade_window_limit() {
        let query = TradeWindowQuery::new("BTCUSDT".to_string(), TradeAnchor::TradeId(500), 499, 0)
            .unwrap();

        assert_eq!(query.limit(), 500);
    }

    #[test]
    fn rejects_a_window_larger_than_500_trades() {
        let error =
            TradeWindowQuery::new("BTCUSDT".to_string(), TradeAnchor::TradeId(500), 250, 250)
                .unwrap_err();

        assert_eq!(error, "before + after + 1 must be in 1..=500");
    }

    #[test]
    fn preserves_a_utc_time_anchor() {
        let time = Utc.with_ymd_and_hms(2026, 7, 26, 12, 34, 56).unwrap();
        let query =
            TradeWindowQuery::new("BTCUSDT".to_string(), TradeAnchor::Time(time), 20, 20).unwrap();

        assert!(matches!(query.anchor, TradeAnchor::Time(value) if value == time));
    }
}

/// 创建订单所需的最小领域输入。
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PlaceOrderRequest {
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub price: Option<Decimal>,
    pub quantity: Decimal,
}
