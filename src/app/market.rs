use crate::domain::market::{ExchangeInfo, SymbolStatusFilter};
use crate::domain::trade::{PublicTrade, TradeAnchor, TradeWindowQuery};
use crate::error::AppError;

use super::context::AppContext;

/// 查询单个现货交易对的交易规则。
///
/// HTTP 请求和 Binance 响应映射由 adapter 层完成。
pub async fn handle_symbol(ctx: &AppContext, symbol: &str) -> Result<ExchangeInfo, AppError> {
    ctx.binance.market().exchange_info_symbol(symbol).await
}

/// 查询现货交易对，并按交易状态筛选。
pub async fn handle_all(
    ctx: &AppContext,
    filter: SymbolStatusFilter,
) -> Result<Vec<ExchangeInfo>, AppError> {
    ctx.binance.market().exchange_info(filter).await
}

/// 查询围绕 trade ID 或 UTC 时间锚点的现货公开历史成交。
///
/// Binance 历史接口按 trade ID 分页，因此时间锚点先通过聚合成交接口映射为
/// 对应 ID，再沿用同一窗口范围进行查询。
pub async fn trades(
    ctx: &AppContext,
    query: TradeWindowQuery,
) -> Result<Vec<PublicTrade>, AppError> {
    let api = ctx.binance().market();
    let historical = match query.anchor {
        TradeAnchor::TradeId(_) => query,
        TradeAnchor::Time(time) => {
            let trade_id = api.trade_id_at(&query.symbol, time).await?;
            query.with_trade_id(trade_id)
        }
    };

    api.historical_trades(&historical).await
}
