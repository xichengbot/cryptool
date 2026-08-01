# cryptool 架构概览

## 项目定位

`cryptool` 是一个 Rust 命令行工具，用于查询 Binance 现货与 USDⓈ-M 合约市场规则、公开历史成交，以及预留的账户与私有交易能力。它以统一领域模型和输出协议隔离 CLI、HTTP adapter 与终端呈现。

## 技术栈

- Rust 2024
- Tokio：异步运行时
- clap：命令行解析与帮助文本
- reqwest + rustls：HTTP 客户端，默认继承进程 HTTP(S) 代理环境
- serde / serde_json / toml：配置与 API JSON 序列化
- rust_decimal：交易数量、价格和规则的精确十进制表达
- chrono：UTC 成交时间锚点
- ed25519-dalek：私有 Binance 请求签名
- comfy-table：表格输出

## 顶层结构

```text
src/
├── main.rs              # 启动、配置覆写、命令分发、输出渲染
├── app/                 # 应用服务编排
├── binance/             # Binance REST adapter、签名与响应映射
├── cli/                 # clap 命令树与输入校验
├── config/              # 默认值、TOML、环境变量、CLI 覆写
├── domain/              # 市场、账户、交易领域模型
├── output/              # JSON / table 统一输出
└── error.rs             # 跨层 AppError
```

## 模块边界

```text
CLI 输入
  │
  ▼
cli ───────────────► app ───────────────► binance ───────────────► Binance REST
  │                   │                    │
  │                   └──────── domain ────┘
  │
  └────────────────────────────► output ─────────────────────────► stdout

config ────────────────────────► main / AppContext / BinanceClient
error  ◄──────────────────────── 所有边界层
```

- `cli`：把 clap 解析结果转换成 `MarketCommand`、`FuturesMarketCommand`、`AccountCommand`；不直接处理 HTTP 响应。
- `app`：应用服务层，只编排查询流程，例如时间锚点先解析为 trade ID 再查询历史成交窗口。
- `binance`：负责 endpoint、请求认证、HTTP、Binance 错误响应和外部 JSON 到领域模型的转换。
- `domain`：与 Binance JSON、clap 或终端输出无关的稳定业务类型。
- `output`：将 `Output` 协议渲染成 JSON 或表格，不参与业务判断。

## 入口与命令树

`main` 使用 `#[tokio::main]` 启动：

1. 初始化 `tracing_subscriber`。
2. 解析 `Cli`。
3. 通过 `Settings::load` 合并默认值 / TOML / 环境变量 / CLI 基地址覆写。
4. 应用 `--output` 覆写。
5. 创建持有共享 `BinanceClient` 的 `AppContext`。
6. 分发顶层 `Command`，各分支返回统一 `Output`。
7. 按 `OutputFormat` 渲染 stdout。

```text
cryptool
├── market
│   ├── symbol <SYMBOL>
│   ├── all [--status trading|halt|break|all]
│   └── trades <SYMBOL> (--trade-id <ID> | --time <RFC3339Z>) [--before N] [--after N]
├── futures-market
│   ├── symbol <SYMBOL>
│   ├── all [--status trading|halt|break|all]
│   └── trades <SYMBOL> (--trade-id <ID> | --time <RFC3339Z>) [--before N] [--after N]
└── account
    └── balance [--account spot|funding|futures]
```

## 配置优先级

`Settings::load` 的语义如下：

```text
内置默认值 或 --config TOML
            │
            ▼
BINANCE_API_KEY / BINANCE_PRIVATE_KEY 环境变量覆盖凭据
            │
            ▼
--base-url 覆盖现货 REST 基地址
            │
            ▼
main 中 --output 覆盖 output.format
```

现货与 USDⓈ-M 合约具有独立 base URL。HTTP 客户端使用 `timeout_ms` 创建，并会遵循进程设置的 HTTP(S) 代理环境变量。

## 并发模型

- 程序运行在单个 Tokio runtime 上。
- 命令处理路径是顺序 `await`，没有当前命令内的并发 fan-out。
- 网络 I/O 由 `reqwest` 异步执行；`BinanceClient` 在 `AppContext` 生命周期内复用，因而复用连接池、超时和代理策略。
- 当前没有显式锁、channel、spawn 任务或共享可变状态；主要风险来自外部 API 延迟、代理可用性和 Binance 限流。

## 关键约束

- 价格、数量、最小名义金额使用 `Decimal`，避免浮点舍入影响交易规则。
- 历史成交窗口满足 `before + after + 1 ∈ 1..=500`。
- 历史成交 endpoint 需要 `X-MBX-APIKEY`，但不需要私钥签名。
- 私有签名请求使用 Ed25519 PKCS#8 PEM；签名输入必须保持 canonical query 的编码与字段顺序。