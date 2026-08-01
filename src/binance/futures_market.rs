use crate::binance::endpoints::{
    FAPI_V1_AGG_TRADES, FAPI_V1_EXCHANGE_INFO, FAPI_V1_HISTORICAL_TRADES,
};
use crate::binance::{BinanceClient, parse_decimal};
use crate::domain::market::{
    FuturesContractType, FuturesExchangeInfo, SymbolStatus, SymbolStatusFilter,
};
use crate::domain::trade::{PublicTrade, TradeWindowQuery};
use crate::error::AppError;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Deserialize;

/// USDⓈ-M 合约市场 REST adapter。
///
/// Binance futures `exchangeInfo` 不支持按状态查询，因此全量拉取后在
/// 本地应用 `SymbolStatusFilter`。
pub struct BinanceFuturesMarketApi<'a> {
    client: &'a BinanceClient,
}

impl<'a> BinanceFuturesMarketApi<'a> {
    pub fn new(client: &'a BinanceClient) -> Self {
        Self { client }
    }

    /// 查询指定 USDⓈ-M 合约交易对的交易规则。
    pub async fn exchange_info_symbol(
        &self,
        symbol: &str,
    ) -> Result<FuturesExchangeInfo, AppError> {
        let exchange_info = self.execute_request().await?;
        let symbol_info = exchange_info
            .symbols
            .into_iter()
            .find(|candidate| candidate.symbol == symbol)
            .ok_or_else(|| AppError::NotFound(format!("futures symbol not found: {symbol}")))?;

        try_into_exchange_info_symbol(symbol_info)
    }

    /// 查询 USDⓈ-M 合约交易规则，并在客户端按状态过滤。
    pub async fn exchange_info(
        &self,
        filter: SymbolStatusFilter,
    ) -> Result<Vec<FuturesExchangeInfo>, AppError> {
        let exchange_info = self.execute_request().await?;

        exchange_info
            .symbols
            .into_iter()
            .filter(|symbol| matches_filter(&symbol.status, &filter))
            .map(try_into_exchange_info_symbol)
            .collect()
    }

    /// 拉取完整合约 exchangeInfo；筛选必须在响应映射后执行。
    async fn execute_request(&self) -> Result<FuturesExchangeInfoResponse, AppError> {
        self.client
            .get(
                self.client.usdm_futures_base_url(),
                FAPI_V1_EXCHANGE_INFO,
                &EmptyQuery,
            )
            .await
    }

    pub async fn trade_id_at(&self, symbol: &str, time: DateTime<Utc>) -> Result<u64, AppError> {
        let start_time = time.timestamp_millis();
        let trades: Vec<FuturesAggregateTradeResponse> = self
            .client
            .api_key_get_at(
                self.client.usdm_futures_base_url(),
                FAPI_V1_AGG_TRADES,
                &[
                    ("symbol", symbol.to_string()),
                    ("startTime", start_time.to_string()),
                    ("endTime", (start_time + 1_000).to_string()),
                    ("limit", "1".to_string()),
                ],
            )
            .await?;

        first_futures_trade_id(trades)
    }

    pub async fn historical_trades(
        &self,
        query: &TradeWindowQuery,
    ) -> Result<Vec<PublicTrade>, AppError> {
        let trades: Vec<FuturesHistoricalTradeResponse> = self
            .client
            .api_key_get_at(
                self.client.usdm_futures_base_url(),
                FAPI_V1_HISTORICAL_TRADES,
                &[
                    ("symbol", query.symbol.clone()),
                    ("fromId", query.start_id().to_string()),
                    ("limit", query.limit().to_string()),
                ],
            )
            .await?;

        trades
            .into_iter()
            .map(map_futures_historical_trade)
            .collect()
    }
}

/// 对不包含 query 参数的合约请求使用空可序列化对象。
#[derive(serde::Serialize)]
struct EmptyQuery;

/// 仅声明构造合约领域规则所需的 `exchangeInfo` 字段。
#[derive(Debug, Deserialize)]
struct FuturesExchangeInfoResponse {
    symbols: Vec<FuturesExchangeSymbol>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FuturesExchangeSymbol {
    symbol: String,
    status: String,
    base_asset: String,
    quote_asset: String,
    margin_asset: String,
    contract_type: String,
    price_precision: u32,
    quantity_precision: u32,
    market_take_bound: String,
    filters: Vec<FuturesExchangeFilter>,
}

/// Binance 合约 filter 的最小集合；未知变体用于兼容未来扩展。
#[derive(Debug, Deserialize)]
#[serde(tag = "filterType", rename_all = "SCREAMING_SNAKE_CASE")]
enum FuturesExchangeFilter {
    #[serde(rename_all = "camelCase")]
    PriceFilter { tick_size: String },
    #[serde(rename_all = "camelCase")]
    LotSize { min_qty: String, step_size: String },
    #[serde(rename_all = "camelCase")]
    MinNotional { notional: String },
    #[serde(rename_all = "camelCase")]
    Notional { notional: String },
    #[serde(other)]
    Unknown,
}

/// 判断 Binance 原始状态是否满足 CLI 请求的筛选条件。
fn matches_filter(status: &str, filter: &SymbolStatusFilter) -> bool {
    match filter {
        SymbolStatusFilter::All => true,
        SymbolStatusFilter::Status(status_filter) => status_filter
            .as_binance_param()
            .is_some_and(|expected| status == expected),
    }
}

/// 将 Binance 合约字段和 filter 集合映射为领域交易规则。
fn try_into_exchange_info_symbol(
    symbol: FuturesExchangeSymbol,
) -> Result<FuturesExchangeInfo, AppError> {
    let tick_size = extract_tick_size(&symbol.filters)?;
    let (min_quantity, step_size) = extract_lot_size(&symbol.filters)?;
    let min_notional = extract_min_notional(&symbol.filters)?;

    Ok(FuturesExchangeInfo {
        symbol: symbol.symbol,
        status: parse_status(symbol.status),
        base_asset: symbol.base_asset,
        quote_asset: symbol.quote_asset,
        margin_asset: symbol.margin_asset,
        contract_type: parse_contract_type(symbol.contract_type),
        price_precision: symbol.price_precision,
        quantity_precision: symbol.quantity_precision,
        min_quantity,
        min_notional,
        tick_size,
        step_size,
        market_take_bound: parse_decimal("marketTakeBound", &symbol.market_take_bound)?,
    })
}

fn parse_status(status: String) -> SymbolStatus {
    match status.as_str() {
        "TRADING" => SymbolStatus::Trading,
        "HALT" => SymbolStatus::Halt,
        "BREAK" => SymbolStatus::Break,
        _ => SymbolStatus::Unknown(status),
    }
}

fn parse_contract_type(contract_type: String) -> FuturesContractType {
    match contract_type.as_str() {
        "PERPETUAL" => FuturesContractType::Perpetual,
        "CURRENT_MONTH" => FuturesContractType::CurrentMonth,
        "NEXT_MONTH" => FuturesContractType::NextMonth,
        "CURRENT_QUARTER" => FuturesContractType::CurrentQuarter,
        "NEXT_QUARTER" => FuturesContractType::NextQuarter,
        "PERPETUAL_DELIVERING" => FuturesContractType::PerpetualDelivering,
        _ => FuturesContractType::Unknown(contract_type),
    }
}

fn extract_tick_size(filters: &[FuturesExchangeFilter]) -> Result<Decimal, AppError> {
    filters
        .iter()
        .find_map(|filter| match filter {
            FuturesExchangeFilter::PriceFilter { tick_size } => {
                Some(parse_decimal("tickSize", tick_size))
            }
            _ => None,
        })
        .unwrap_or_else(|| Err(AppError::InvalidInput("missing PRICE_FILTER".to_string())))
}

fn extract_lot_size(filters: &[FuturesExchangeFilter]) -> Result<(Decimal, Decimal), AppError> {
    for filter in filters {
        if let FuturesExchangeFilter::LotSize { min_qty, step_size } = filter {
            return Ok((
                parse_decimal("minQty", min_qty)?,
                parse_decimal("stepSize", step_size)?,
            ));
        }
    }

    Err(AppError::InvalidInput("missing LOT_SIZE".to_string()))
}

fn extract_min_notional(filters: &[FuturesExchangeFilter]) -> Result<Decimal, AppError> {
    filters
        .iter()
        .find_map(|filter| match filter {
            FuturesExchangeFilter::MinNotional { notional }
            | FuturesExchangeFilter::Notional { notional } => {
                Some(parse_decimal("notional", notional))
            }
            _ => None,
        })
        .unwrap_or_else(|| Err(AppError::InvalidInput("missing NOTIONAL".to_string())))
}

#[derive(Debug, Deserialize)]
struct FuturesAggregateTradeResponse {
    #[serde(rename = "f")]
    first_trade_id: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FuturesHistoricalTradeResponse {
    id: u64,
    price: String,
    qty: String,
    quote_qty: String,
    time: i64,
    is_buyer_maker: bool,
    #[serde(rename = "isRPITrade")]
    is_rpi_trade: Option<bool>,
}

fn first_futures_trade_id(trades: Vec<FuturesAggregateTradeResponse>) -> Result<u64, AppError> {
    trades
        .into_iter()
        .next()
        .map(|trade| trade.first_trade_id)
        .ok_or_else(|| {
            AppError::NotFound("no trade found within the requested time window".to_string())
        })
}

fn map_futures_historical_trade(
    trade: FuturesHistoricalTradeResponse,
) -> Result<PublicTrade, AppError> {
    Ok(PublicTrade {
        id: trade.id,
        price: parse_decimal("price", &trade.price)?,
        quantity: parse_decimal("qty", &trade.qty)?,
        quote_quantity: parse_decimal("quoteQty", &trade.quote_qty)?,
        timestamp: parse_timestamp(trade.time)?,
        is_buyer_maker: trade.is_buyer_maker,
        is_best_match: None,
        is_rpi_trade: trade.is_rpi_trade,
    })
}

fn parse_timestamp(timestamp_ms: i64) -> Result<DateTime<Utc>, AppError> {
    DateTime::from_timestamp_millis(timestamp_ms)
        .ok_or_else(|| AppError::Serialization(format!("invalid trade timestamp: {timestamp_ms}")))
}
