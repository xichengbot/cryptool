use crate::binance::endpoints::{API_V3_AGG_TRADES, API_V3_HISTORICAL_TRADES};
use crate::domain::market::{ExchangeInfo, SymbolStatus, SymbolStatusFilter};
use crate::domain::trade::{PublicTrade, TradeWindowQuery};
use crate::error::AppError;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Deserialize;

use super::{client::BinanceClient, endpoints::API_V3_EXCHANGE_INFO, parse_decimal};

/// 现货市场 REST adapter。
///
/// 将 `/api/v3/exchangeInfo` 的响应转换为现货领域交易规则。
pub struct BinanceMarketApi<'a> {
    client: &'a BinanceClient,
}

impl<'a> BinanceMarketApi<'a> {
    pub fn new(client: &'a BinanceClient) -> Self {
        Self { client }
    }

    /// 查询指定现货交易对的交易规则。
    pub async fn exchange_info_symbol(&self, symbol: &str) -> Result<ExchangeInfo, AppError> {
        let exchange_info: ExchangeInfoResponse = self
            .client
            .get(
                self.client.spot_base_url(),
                API_V3_EXCHANGE_INFO,
                &[("symbol", symbol)],
            )
            .await?;

        try_into_exchange_info(exchange_info.symbols, symbol)
    }

    /// 查询现货交易规则；`All` 拉取全量响应，其余状态由 Binance 在服务端筛选。
    pub async fn exchange_info(
        &self,
        filter: SymbolStatusFilter,
    ) -> Result<Vec<ExchangeInfo>, AppError> {
        let exchange_info: ExchangeInfoResponse = match filter {
            SymbolStatusFilter::All => {
                self.client
                    .get(
                        self.client.spot_base_url(),
                        API_V3_EXCHANGE_INFO,
                        &EmptyQuery,
                    )
                    .await?
            }
            SymbolStatusFilter::Status(status) => {
                let status = status.as_binance_param().ok_or_else(|| {
                    AppError::InvalidInput("unknown symbol status cannot be queried".to_string())
                })?;
                self.client
                    .get(
                        self.client.spot_base_url(),
                        API_V3_EXCHANGE_INFO,
                        &[("symbolStatus", status)],
                    )
                    .await?
            }
        };
        exchange_info
            .symbols
            .into_iter()
            .map(try_into_exchange_info_symbol)
            .collect()
    }

    pub async fn trade_id_at(&self, symbol: &str, time: DateTime<Utc>) -> Result<u64, AppError> {
        let start_time = time.timestamp_millis();
        let trades: Vec<SpotAggregateTradeResponse> = self
            .client
            .api_key_get(
                API_V3_AGG_TRADES,
                &[
                    ("symbol", symbol.to_string()),
                    ("startTime", start_time.to_string()),
                    ("endTime", (start_time + 1_000).to_string()),
                    ("limit", "1".to_string()),
                ],
            )
            .await?;

        first_spot_trade_id(trades)
    }

    pub async fn historical_trades(
        &self,
        query: &TradeWindowQuery,
    ) -> Result<Vec<PublicTrade>, AppError> {
        let trades: Vec<SpotHistoricalTradeResponse> = self
            .client
            .api_key_get(
                API_V3_HISTORICAL_TRADES,
                &[
                    ("symbol", query.symbol.clone()),
                    ("fromId", query.start_id().to_string()),
                    ("limit", query.limit().to_string()),
                ],
            )
            .await?;

        trades.into_iter().map(map_spot_historical_trade).collect()
    }
}

/// 对不包含 query 参数的请求使用空可序列化对象。
#[derive(serde::Serialize)]
struct EmptyQuery;
/// 仅声明映射领域规则所需的 `exchangeInfo` 响应字段。
#[derive(Debug, Deserialize)]
struct ExchangeInfoResponse {
    symbols: Vec<ExchangeSymbol>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExchangeSymbol {
    symbol: String,
    status: String,
    base_asset: String,
    quote_asset: String,
    base_asset_precision: u32,
    quote_precision: u32,
    filters: Vec<ExchangeFilter>,
}

/// Binance filter 的最小集合。
///
/// 未识别的 filter 被显式忽略，以兼容 Binance 将来添加的规则类型。
#[derive(Debug, Deserialize)]
#[serde(tag = "filterType", rename_all = "SCREAMING_SNAKE_CASE")]
enum ExchangeFilter {
    #[serde(rename_all = "camelCase")]
    PriceFilter { tick_size: String },
    #[serde(rename_all = "camelCase")]
    LotSize { min_qty: String, step_size: String },
    #[serde(rename_all = "camelCase")]
    MinNotional { min_notional: String },
    #[serde(rename_all = "camelCase")]
    Notional { min_notional: String },
    #[serde(other)]
    Unknown,
}

/// 从指定交易对查询响应中取出唯一规则；空 symbols 列表视为不存在。
fn try_into_exchange_info(
    symbols: Vec<ExchangeSymbol>,
    requested_symbol: &str,
) -> Result<ExchangeInfo, AppError> {
    let symbol_info = symbols
        .into_iter()
        .next()
        .ok_or_else(|| AppError::NotFound(format!("symbol not found: {requested_symbol}")))?;
    try_into_exchange_info_symbol(symbol_info)
}

/// 将 Binance 的外部字段和 filter 集合映射为稳定的现货领域模型。
fn try_into_exchange_info_symbol(symbol_info: ExchangeSymbol) -> Result<ExchangeInfo, AppError> {
    let tick_size = extract_tick_size(&symbol_info.filters)?;
    let (min_quantity, step_size) = extract_lot_size(&symbol_info.filters)?;
    let min_notional = extract_min_notional(&symbol_info.filters)?;

    Ok(ExchangeInfo {
        symbol: symbol_info.symbol,
        status: parse_status(symbol_info.status),
        base_asset: symbol_info.base_asset,
        quote_asset: symbol_info.quote_asset,
        price_precision: symbol_info.quote_precision,
        quantity_precision: symbol_info.base_asset_precision,
        min_quantity,
        min_notional,
        tick_size,
        step_size,
    })
}

fn parse_timestamp(timestamp_ms: i64) -> Result<DateTime<Utc>, AppError> {
    DateTime::from_timestamp_millis(timestamp_ms)
        .ok_or_else(|| AppError::Serialization(format!("invalid trade timestamp: {timestamp_ms}")))
}

fn parse_status(status: String) -> SymbolStatus {
    match status.as_str() {
        "TRADING" => SymbolStatus::Trading,
        "HALT" => SymbolStatus::Halt,
        "BREAK" => SymbolStatus::Break,
        _ => SymbolStatus::Unknown(status),
    }
}

fn extract_tick_size(filters: &[ExchangeFilter]) -> Result<Decimal, AppError> {
    filters
        .iter()
        .find_map(|filter| match filter {
            ExchangeFilter::PriceFilter { tick_size } => Some(parse_decimal("tickSize", tick_size)),
            _ => None,
        })
        .unwrap_or_else(|| Err(AppError::InvalidInput("missing PRICE_FILTER".to_string())))
}

fn extract_lot_size(filters: &[ExchangeFilter]) -> Result<(Decimal, Decimal), AppError> {
    for filter in filters {
        if let ExchangeFilter::LotSize { min_qty, step_size } = filter {
            return Ok((
                parse_decimal("minQty", min_qty)?,
                parse_decimal("stepSize", step_size)?,
            ));
        }
    }

    Err(AppError::InvalidInput("missing LOT_SIZE".to_string()))
}

fn extract_min_notional(filters: &[ExchangeFilter]) -> Result<Decimal, AppError> {
    filters
        .iter()
        .find_map(|filter| match filter {
            ExchangeFilter::MinNotional { min_notional }
            | ExchangeFilter::Notional { min_notional } => {
                Some(parse_decimal("minNotional", min_notional))
            }
            _ => None,
        })
        .unwrap_or_else(|| {
            Err(AppError::InvalidInput(
                "missing MIN_NOTIONAL or NOTIONAL".to_string(),
            ))
        })
}

#[derive(Debug, Deserialize)]
struct SpotAggregateTradeResponse {
    #[serde(rename = "f")]
    first_trade_id: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpotHistoricalTradeResponse {
    id: u64,
    price: String,
    qty: String,
    quote_qty: String,
    time: i64,
    is_buyer_maker: bool,
    is_best_match: bool,
}

fn first_spot_trade_id(trades: Vec<SpotAggregateTradeResponse>) -> Result<u64, AppError> {
    trades
        .into_iter()
        .next()
        .map(|trade| trade.first_trade_id)
        .ok_or_else(|| {
            AppError::NotFound("no trade found within the requested time window".to_string())
        })
}

fn map_spot_historical_trade(trade: SpotHistoricalTradeResponse) -> Result<PublicTrade, AppError> {
    Ok(PublicTrade {
        id: trade.id,
        price: parse_decimal("price", &trade.price)?,
        quantity: parse_decimal("qty", &trade.qty)?,
        quote_quantity: parse_decimal("quoteQty", &trade.quote_qty)?,
        timestamp: parse_timestamp(trade.time)?,
        is_buyer_maker: trade.is_buyer_maker,
        is_best_match: Some(trade.is_best_match),
        is_rpi_trade: None,
    })
}
