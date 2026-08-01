use clap::Subcommand;

use super::account::AccountCommand;
use super::futures_market::FuturesMarketCommand;
use super::market::MarketCommand;

/// 一级业务子命令。
///
/// 每个 variant 携带自身的参数集合，并在入口处调用对应的 `execute` 方法。
#[derive(Debug, Subcommand)]
pub enum Command {
    /// 查询 Binance 现货市场交易规则。
    Market(MarketCommand),
    /// 查询 Binance USDⓈ-M 合约市场交易规则。
    FuturesMarket(FuturesMarketCommand),
    /// 查询账户相关信息。
    Account(AccountCommand),
}
