use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// 现货交易对的可下单规则。
///
/// 数量和价格相关字段使用 Decimal，以保证下单校验不受浮点舍入影响。
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExchangeInfo {
    pub symbol: String,
    pub status: SymbolStatus,
    pub base_asset: String,
    pub quote_asset: String,
    pub price_precision: u32,
    pub quantity_precision: u32,
    pub min_quantity: Decimal,
    pub min_notional: Decimal,
    pub tick_size: Decimal,
    pub step_size: Decimal,
}

/// USDⓈ-M 合约交易对的可下单规则。
///
/// 与现货规则相比，合约额外包含保证金资产、合约类型与市价保护边界。
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FuturesExchangeInfo {
    pub symbol: String,
    pub status: SymbolStatus,
    pub base_asset: String,
    pub quote_asset: String,
    pub margin_asset: String,
    pub contract_type: FuturesContractType,
    pub price_precision: u32,
    pub quantity_precision: u32,
    pub min_quantity: Decimal,
    pub min_notional: Decimal,
    pub tick_size: Decimal,
    pub step_size: Decimal,
    pub market_take_bound: Decimal,
}

/// Binance USDⓈ-M 合约交割类型。
///
/// `Unknown` 保留上游新增值，避免 API 扩展导致整个响应反序列化失败。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FuturesContractType {
    Perpetual,
    CurrentMonth,
    NextMonth,
    CurrentQuarter,
    NextQuarter,
    PerpetualDelivering,
    Unknown(String),
}

/// Binance 交易对的运行状态。
///
/// `Unknown` 允许展示但不允许作为服务端查询筛选条件。
#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub enum SymbolStatus {
    Trading,
    Halt,
    Break,
    Unknown(String),
}

impl fmt::Display for SymbolStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Trading => write!(f, "trading"),
            Self::Halt => write!(f, "halt"),
            Self::Break => write!(f, "break"),
            Self::Unknown(value) => write!(f, "{value}"),
        }
    }
}

impl FromStr for SymbolStatus {
    type Err = ParseSymbolStatusError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "trading" => Ok(Self::Trading),
            "halt" => Ok(Self::Halt),
            "break" => Ok(Self::Break),
            _ => Err(ParseSymbolStatusError(value.to_string())),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid symbol status '{0}', expected trading, halt, or break")]
pub struct ParseSymbolStatusError(String);

/// 市场规则查询的状态筛选。
///
/// 默认只查询交易中标的；`All` 表示不向现货 endpoint 传递状态参数。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SymbolStatusFilter {
    All,
    Status(SymbolStatus),
}

impl Default for SymbolStatusFilter {
    fn default() -> Self {
        Self::Status(SymbolStatus::Trading)
    }
}

impl fmt::Display for SymbolStatusFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::All => write!(f, "all"),
            Self::Status(status) => write!(f, "{status}"),
        }
    }
}

impl FromStr for SymbolStatusFilter {
    type Err = ParseRuleStatusFilterError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value == "all" {
            return Ok(Self::All);
        }

        SymbolStatus::from_str(value)
            .map(Self::Status)
            .map_err(|_| ParseRuleStatusFilterError(value.to_string()))
    }
}

impl SymbolStatus {
    /// 返回 Binance `symbolStatus` query 参数需要的上游大写枚举值。
    ///
    /// 未知状态没有可靠的上游参数映射，必须由调用方拒绝查询。
    pub fn as_binance_param(&self) -> Option<&'static str> {
        match self {
            Self::Trading => Some("TRADING"),
            Self::Halt => Some("HALT"),
            Self::Break => Some("BREAK"),
            Self::Unknown(_) => None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid rule status filter '{0}', expected all, trading, halt, or break")]
pub struct ParseRuleStatusFilterError(String);
