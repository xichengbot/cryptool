use clap::{Args, Subcommand};

use crate::cli::trades::TradeWindowArgs;
use crate::{
    app::{self, context::AppContext},
    domain::market::SymbolStatusFilter,
    error::AppError,
    output::Output,
};

/// 现货市场命令的参数集合。
///
/// 其 `action` 是 `market` 后必须提供的二级子命令。
#[derive(Debug, Args)]
pub struct MarketCommand {
    /// 要执行的现货市场查询。
    #[command(subcommand)]
    pub action: MarketAction,
}

/// 现货市场支持的查询操作。
#[derive(Debug, Subcommand)]
pub enum MarketAction {
    /// 查询指定交易对的交易规则，例如 `market symbol BTCUSDT`。
    Symbol {
        /// Binance 交易对代码，例如 `BTCUSDT`。
        symbol: String,
    },
    /// 查询多个交易对，并可按 Binance 交易状态筛选。
    All {
        /// 交易状态；默认只返回 `trading` 交易对。
        #[arg(long, default_value = "trading")]
        status: SymbolStatusFilter,
    },
    /// 查询围绕 trade ID 或 UTC 时间锚点的现货公开历史成交。
    Trades {
        /// Binance 现货交易对代码，例如 `BTCUSDT`。
        symbol: String,
        #[command(flatten)]
        query: TradeWindowArgs,
    },
}

impl MarketCommand {
    /// 将 CLI 参数编排为应用服务调用，并转换为统一输出协议。
    pub async fn execute(self, ctx: &AppContext) -> Result<Output, AppError> {
        match self.action {
            MarketAction::Symbol { symbol } => {
                let exchange_info = app::market::handle_symbol(ctx, &symbol).await?;
                Output::json(exchange_info)
            }

            MarketAction::All { status } => {
                // TODO
                let exchange_info = app::market::handle_all(ctx, status).await?;
                Output::json(exchange_info)
            }

            MarketAction::Trades { symbol, query } => {
                // 时间锚点先通过聚合成交接口定位 trade ID，再请求历史成交窗口。
                let query = query
                    .to_query(symbol)
                    .map_err(|error| AppError::InvalidInput(error.to_string()))?;
                let trades = app::market::trades(ctx, query).await?;
                Output::json(trades)
            }
        }
    }
}

// MarketAction::Rules { symbol } => {
//     let rule = app::market::rules(ctx, &symbol).await?;
//     Output::json(rule)
// }
// MarketAction::Fees { symbol } => {
//     app::market::fees(ctx, &symbol).await?;
//     Ok(Output::Empty)
// }
