//! Binance REST endpoint 路径常量。
//!
//! 路径与市场基地址分离，以便同一客户端安全地支持现货和 USDⓈ-M 合约。

/// 现货交易规则与交易对元数据接口。
pub const API_V3_EXCHANGE_INFO: &str = "/api/v3/exchangeInfo";
/// USDⓈ-M 合约交易规则与合约元数据接口。
pub const FAPI_V1_EXCHANGE_INFO: &str = "/fapi/v1/exchangeInfo";

pub const API_V3_TIME: &str = "/api/v3/time";
pub const API_V3_ACCOUNT: &str = "/api/v3/account";
pub const API_V3_OPEN_ORDERS: &str = "/api/v3/openOrders";
pub const API_V3_ORDER: &str = "/api/v3/order";
pub const SAPI_ASSET_TRANSFER: &str = "/sapi/v1/asset/transfer";

pub const FAPI_V1_AGG_TRADES: &str = "/fapi/v1/aggTrades";
pub const FAPI_V1_HISTORICAL_TRADES: &str = "/fapi/v1/historicalTrades";
pub const API_V3_AGG_TRADES: &str = "/api/v3/aggTrades";
pub const API_V3_HISTORICAL_TRADES: &str = "/api/v3/historicalTrades";
