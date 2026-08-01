//! Binance REST adapter。
//!
//! 该层负责选择现货或 USDⓈ-M 基地址、发送 HTTP 请求、转换 Binance
//! 错误响应，并把外部 JSON 映射为领域模型。

mod account;
pub mod client;
mod endpoints;
mod futures_market;
mod market;
mod response;
mod signer;

use rust_decimal::Decimal;

use crate::error::AppError;

/// 将 Binance 以字符串传输的十进制数解析为精确 Decimal。
///
/// 交易规则不能经由浮点数中转，否则 tick size 和最小下单量可能产生
/// 精度损失。
pub(super) fn parse_decimal(field: &str, value: &str) -> Result<Decimal, AppError> {
    value.parse::<Decimal>().map_err(|err| {
        AppError::Serialization(format!("failed to parse {field} as decimal: {err}"))
    })
}

pub use client::BinanceClient;
