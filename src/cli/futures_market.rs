use crate::app;
use crate::app::context::AppContext;
use crate::cli::trades::TradeWindowArgs;
use crate::domain::market::SymbolStatusFilter;
use crate::error::AppError;
use crate::output::Output;
use clap::{Args, Subcommand};

/// USDⓈ-M 合约市场命令的参数集合。
#[derive(Debug, Args)]
pub struct FuturesMarketCommand {
    /// 要执行的合约市场查询。
    #[command(subcommand)]
    pub action: FuturesMarketAction,
}

/// USDⓈ-M 合约市场支持的查询操作。
#[derive(Debug, Subcommand)]
pub enum FuturesMarketAction {
    /// 查询单个 USDⓈ-M 合约交易对的交易规则。
    Symbol {
        /// Binance USDⓈ-M 合约交易对代码，例如 `BTCUSDT`。
        symbol: String,
    },
    /// 查询 USDⓈ-M 合约交易规则，并可按交易状态筛选。
    All {
        #[arg(long, default_value = "trading")]
        status: SymbolStatusFilter,
    },
    /// 查询围绕 trade ID 或 UTC 时间锚点的 USDⓈ-M 合约公开历史成交。
    Trades {
        /// Binance USDⓈ-M 合约交易对代码，例如 `BTCUSDT`。
        symbol: String,
        #[command(flatten)]
        query: TradeWindowArgs,
    },
}

impl FuturesMarketCommand {
    /// 将合约 CLI 参数编排为应用服务调用，并转换为统一输出协议。
    pub async fn execute(self, ctx: &AppContext) -> Result<Output, AppError> {
        match self.action {
            FuturesMarketAction::Symbol { symbol } => {
                let rule = app::futures_market::handle_symbol(ctx, &symbol).await?;
                Output::json(rule)
            }
            FuturesMarketAction::All { status } => {
                let rules = app::futures_market::handle_all(ctx, status).await?;
                Output::json(rules)
            }
            FuturesMarketAction::Trades { symbol, query } => {
                // 时间锚点需要先映射为合约聚合成交 ID，历史接口仅接受 ID 分页。
                let query = query
                    .to_query(symbol)
                    .map_err(|error| AppError::InvalidInput(error.to_string()))?;
                let trades = app::futures_market::trades(ctx, query).await?;
                Output::json(trades)
            }
        }
    }
}
