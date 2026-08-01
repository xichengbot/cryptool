# cryptool 模块设计

## 依赖方向

```text
main
 ├── cli
 ├── config
 ├── app
 │    └── binance
 ├── output
 └── error

binance ──► domain
app     ──► domain
cli     ──► domain
output  ──► error
```

`domain` 不依赖 `cli`、`app`、`binance` 或 `output`。这使外部 Binance 字段映射和用户输入格式可以演进，而不影响领域模型调用方。

## main

**职责**

- 初始化 Tokio 和日志。
- 组装最终配置、共享上下文与输出格式。
- 将顶层命令分发到各命令对象。
- 统一渲染 `Output`。

**边界**

`main` 不包含 endpoint、认证、JSON 解析或表格布局逻辑。

## cli

### 组成

- `args.rs`：顶层 `Cli` 及全局选项。
- `commands.rs`：一级 `Command` 枚举。
- `market.rs`：现货市场子命令。
- `futures_market.rs`：USDⓈ-M 合约市场子命令。
- `trades.rs`：现货与合约共用的历史成交窗口参数。
- `account.rs`：账户子命令。

### 职责

- 使用 clap 声明命令层级、默认值、参数互斥与枚举范围。
- 将 CLI 参数转换为领域查询，例如 `TradeWindowArgs::to_query`。
- 将应用服务的领域结果包装为 `Output`。

### 输入约束

`TradeWindowArgs` 强制 `--trade-id` 与 `--time` 恰好传入一个。

- `--time` 必须是以 `Z` 结束的 RFC3339 UTC 时间，避免本地时区偏移。
- `--before` 与 `--after` 决定以锚点为中心的成交窗口。
- 领域构造器限制总窗口不超过 Binance 单次历史接口的 500 条上限。

## app

### AppContext

`AppContext` 持有一个已初始化的 `BinanceClient`。

- 所有命令共享连接池、HTTP 超时与代理策略。
- 上下文只提供依赖，不存储命令级可变状态。

### 市场服务

`app::market` 与 `app::futures_market` 暴露：

- `handle_symbol`：查询单个交易规则。
- `handle_all`：按状态查询规则集合。
- `trades`：查询历史公开成交。

时间锚点查询的编排：

1. 通过聚合成交 endpoint 找到该时间附近的首个 trade ID。
2. 调用 `TradeWindowQuery::with_trade_id`，保留用户窗口大小。
3. 调用历史成交 endpoint，通过 `fromId` 和 `limit` 获取窗口。

## binance

### BinanceClient

职责：

- 创建 `reqwest::Client` 并应用总超时。
- 拼接 `base_url + endpoint`。
- 编码 query、注入 API Key、发送请求。
- 将非 2xx 响应优先转换为结构化 `AppError::Binance`。
- 将成功 JSON 反序列化为调用者指定类型。

请求分类：

| 方法 | 认证 | 市场选择 | 使用场景 |
|---|---|---|---|
| `get` | 无 | 调用方传入 base URL | `exchangeInfo`、聚合成交 |
| `api_key_get` | `X-MBX-APIKEY` | 现货 base URL | 现货历史成交 |
| `api_key_get_at` | `X-MBX-APIKEY` | 调用方传入 base URL | 现货/合约历史成交共享路径 |
| `signed_request` | API Key + Ed25519 签名 | 现货 base URL | 预留的私有请求 |

### 现货 adapter

`BinanceMarketApi` 访问：

- `/api/v3/exchangeInfo`
- `/api/v3/aggTrades`
- `/api/v3/historicalTrades`

现货 `exchangeInfo` 使用 `symbolStatus` 参数时由 Binance 服务端筛选；`all` 不传状态 query。

### USDⓈ-M 合约 adapter

`BinanceFuturesMarketApi` 访问：

- `/fapi/v1/exchangeInfo`
- `/fapi/v1/aggTrades`
- `/fapi/v1/historicalTrades`

合约 `exchangeInfo` 不支持服务端状态筛选，因此先拉取全量响应，再在客户端按 `SymbolStatusFilter` 过滤。

### 数据映射

adapter 内部 response struct 只声明领域模型需要的字段。映射规则：

- Binance 字符串数值经 `parse_decimal` 转换为 `Decimal`。
- `PRICE_FILTER` → `tick_size`。
- `LOT_SIZE` → `min_quantity` 与 `step_size`。
- `MIN_NOTIONAL` 或 `NOTIONAL` → `min_notional`。
- 未识别 filter 用 `Unknown` 忽略，允许 Binance 增加新 filter 而不使整个响应反序列化失败。
- 未知交易状态和合约类型保存原始字符串，保持前向兼容。

## domain

### market

- `ExchangeInfo`：现货交易规则。
- `FuturesExchangeInfo`：合约规则，附加保证金资产、合约类型、市场保护边界。
- `SymbolStatusFilter`：`all` 或已知交易状态；未知状态不能转换为 Binance 查询参数。

### trade

- `PublicTrade`：公开历史成交；可选字段区分上游未返回与 false。
- `TradeAnchor`：原始 trade ID 或 UTC 时间。
- `TradeWindowQuery`：交易对、锚点和窗口范围；负责验证最大 500 条。
- `Order`、`Position` 与 `PlaceOrderRequest`：预留的交易与持仓领域对象。

### account

`AccountType` 定义现货、资金和合约账户的稳定枚举。

## output

所有命令返回三态 `Output`：

- `Text`：纯文本消息。
- `Json`：可序列化结构化数据。
- `Empty`：成功但无内容。

渲染策略由 `OutputFormat` 决定：

- `Json`：保证 `Text`、`Json`、`Empty` 都输出合法 JSON。
- `Table`：对象或对象数组渲染为表格；非对象 JSON 降级为字符串。

## error

`AppError` 是跨层错误契约：

- `Config`：配置和凭据缺失。
- `Http`：客户端、网络、代理、网关或非 Binance 错误响应。
- `Binance`：保留 Binance `code` 与 `message`。
- `InvalidInput`：用户输入或领域约束失败。
- `NotFound`：查询目标不在响应中。
- `Serialization`：JSON 或 Decimal 映射失败。
- `NotImplemented`：已暴露但尚未实现的功能。
