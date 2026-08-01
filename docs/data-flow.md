# cryptool 数据流

## 1. 通用命令执行流

```text
用户命令行
  │
  ▼
clap::Cli::parse
  │  解析全局选项、一级命令和二级子命令
  ▼
Settings::load
  │  默认值 / TOML / 环境变量 / CLI 覆写
  ▼
AppContext::new
  │  构造并复用 BinanceClient
  ▼
Command::{Market|FuturesMarket|Account}::execute
  │
  ▼
app 服务
  │
  ▼
Binance adapter
  │  URL、query、认证、HTTP、JSON 解码
  ▼
领域模型 或 AppError
  │
  ▼
Output::{Json|Text|Empty}
  │
  ▼
output::render
  │
  ▼
stdout（JSON 或表格）
```

## 2. 现货交易规则流

```text
cryptool market symbol BTCUSDT
  │
  ▼
MarketAction::Symbol { symbol }
  │
  ▼
app::market::handle_symbol(ctx, "BTCUSDT")
  │
  ▼
BinanceMarketApi::exchange_info_symbol
  │
  ▼
BinanceClient::get(spot_base_url, "/api/v3/exchangeInfo", symbol query)
  │
  ▼
GET https://api.binance.com/api/v3/exchangeInfo?symbol=BTCUSDT
  │
  ▼
ExchangeInfoResponse → ExchangeSymbol → ExchangeInfo
  │
  ▼
Output::json → JSON/table renderer
```

转换重点：

```text
PRICE_FILTER.tickSize          → ExchangeInfo.tick_size
LOT_SIZE.minQty / stepSize     → min_quantity / step_size
MIN_NOTIONAL 或 NOTIONAL       → min_notional
baseAsset / quoteAsset         → base_asset / quote_asset
```

所有数值字段都从字符串直接解析为 `Decimal`。

## 3. 现货全量规则与状态筛选流

```text
cryptool market all --status trading
  │
  ▼
SymbolStatusFilter::Status(Trading)
  │
  ▼
SymbolStatus::as_binance_param() → "TRADING"
  │
  ▼
GET /api/v3/exchangeInfo?symbolStatus=TRADING
  │
  ▼
将 symbols 逐条映射为 ExchangeInfo
```

```text
cryptool market all --status all
  │
  ▼
SymbolStatusFilter::All
  │
  ▼
GET /api/v3/exchangeInfo（无 query）
```

现货的状态筛选由 Binance 服务端完成。

## 4. USDⓈ-M 合约规则流

```text
cryptool futures-market all --status trading
  │
  ▼
FuturesMarketAction::All
  │
  ▼
app::futures_market::handle_all
  │
  ▼
BinanceFuturesMarketApi::exchange_info
  │
  ▼
GET https://fapi.binance.com/fapi/v1/exchangeInfo
  │
  ▼
客户端按 SymbolStatusFilter 过滤 symbols
  │
  ▼
FuturesExchangeSymbol → FuturesExchangeInfo
```

合约 endpoint 不支持现货的 `symbolStatus` 服务端参数，因此必须先取全量再本地过滤。

## 5. 历史公开成交流

### 以 trade ID 为锚点

```text
cryptool market trades BTCUSDT --trade-id 1000 --before 20 --after 20
  │
  ▼
TradeWindowArgs::to_query
  │  校验 before + after + 1 ∈ 1..=500
  ▼
TradeWindowQuery {
  anchor: TradeId(1000),
  start_id: 980,
  limit: 41,
}
  │
  ▼
BinanceMarketApi::historical_trades
  │
  ▼
API Key GET /api/v3/historicalTrades?symbol=BTCUSDT&fromId=980&limit=41
  │
  ▼
SpotHistoricalTradeResponse → PublicTrade[]
```

### 以 UTC 时间为锚点

```text
cryptool market trades BTCUSDT --time 2026-07-26T12:34:56Z
  │
  ▼
parse_utc_time
  │  只接受以 Z 结尾的 RFC3339 UTC 时间
  ▼
TradeAnchor::Time
  │
  ▼
GET /api/v3/aggTrades?symbol=BTCUSDT&startTime=T&endTime=T+1000&limit=1
  │
  ▼
取得最早匹配 aggregate trade ID
  │
  ▼
TradeWindowQuery::with_trade_id
  │
  ▼
历史成交 API Key GET（同上）
```

USDⓈ-M 合约历史成交遵循同一两阶段模式，但使用 `/fapi/v1/aggTrades` 和 `/fapi/v1/historicalTrades`，并以 USDⓈ-M base URL 发起请求。

## 6. 认证与签名数据流

### API Key 请求

```text
BINANCE_API_KEY（TOML 或环境变量）
  │
  ▼
BinanceClient::api_key
  │  空值时立即返回 Config 错误
  ▼
X-MBX-APIKEY header
  │
  ▼
历史成交 endpoint
```

历史成交是公开市场数据，但 Binance 的 historical trades endpoint 仍要求 API Key。该请求不使用私钥签名。

### 预留的私有签名请求

```text
业务 query
  │
  ▼
serde_urlencoded
  │
  ▼
canonical_query + recvWindow + timestamp
  │
  ▼
Ed25519 PKCS#8 PEM 私钥签名
  │
  ▼
Base64 signature
  │
  ▼
X-MBX-APIKEY + URL signature query
```

任何 query 编码、字段顺序、时间戳或 `recvWindow` 的变化都必须发生在签名之前；签名完成后不得改变 URL 参数。

## 7. 错误流

```text
配置/输入/网络/Binance/JSON/Decimal 错误
  │
  ▼
AppError
  │
  ├── Config
  ├── InvalidInput
  ├── Http
  ├── Binance { code, message }
  ├── Serialization
  ├── NotFound
  └── NotImplemented
  │
  ▼
? 传播至 main
  │
  ▼
anyhow::Result<()> 输出错误链并以非零码退出
```

`decode_response` 优先识别 Binance 标准错误 JSON；如果响应不是 Binance 错误格式，则保留 HTTP 状态码与原始正文，便于分析代理、网关或上游异常。
