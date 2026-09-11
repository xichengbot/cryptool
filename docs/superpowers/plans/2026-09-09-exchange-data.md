# Exchange Data Web / Desktop 实现计划

> 使用 subagent-driven-development 按独立任务实现，任务审查包含规格与代码质量；根代理负责原生查询、API、集成和最终验证。用户已确认方案，不在任务之间请求继续许可。

**目标：** 保留原 cryptool CLI，完整复现 exchange-data 的三个业务页，同时交付 Dioxus Web 与 macOS / Windows Desktop。

**架构：** Web 通过 Axum，Desktop 直接调用原生 Rust 服务；共享 DTO、时间函数与 Dioxus 页面。根 package 提供 CLI / library / 可选 server，另有 exchange-types 和 exchange-ui 两个 package。

**技术栈：** Rust 2024、Dioxus 0.7、Axum 0.8、Tokio、reqwest、serde、chrono / chrono-tz、Decimal。

## 全局约束

- 桌面仅 macOS / Windows，不提供 Linux 桌面目标。
- 图形界面只有首页、现货成交、合约成交、时间转换及已有导航交互，不增加 CLI 规则 / 账户页面。
- CLI 默认 before / after 各 20，上限 500；图形界面各 50，上限 1,000。
- 时间及 ID 不经过浮点；JSON 的 ID / 时间值为字符串；现货微秒、合约毫秒。
- 原 CLI 命令、配置和数据字段保持；修正合约配置覆写、JSON 诊断混入 stdout。
- 新功能优先用测试定义正确结果，确认失败后实现；可逆配置和纯样式不创建镜像测试。
- 不读取或迁入真实凭据。模拟服务只用虚构 Key；不改参考仓库。
- 工作目录 `.worktrees/exchange-data`；使用 `CARGO_TARGET_DIR=/Users/xichengbot/rustProjects/cryptool/target` 复用本地缓存。

## 文件责任

- `crates/exchange-types/src/{lib,time,trade,error}.rs`：可用于 native / WASM 的数据与校验。
- `src/lib.rs`、`src/config/format.rs`：原生复用入口与中立输出格式。
- `src/app/exchange_data.rs`、`src/binance/history.rs`：窗口策略、分页、预算、认证与精度。
- `src/server/{mod,assets}.rs`：HTTP 提取、错误映射、Host / 路由边界、静态资源。
- `apps/exchange-ui/src/`：共享 Dioxus 页面、双查询客户端和平台适配。
- `tests/`：CLI 二进制回归、真实 HTTP fixture 与 API 契约测试。
- `.github/workflows/`、`scripts/`、`docs/`：macOS / Windows 构建、发布与运行验收。

## 任务 1：共享类型与 CLI 原生 library

- [x] 先创建 `tests/cli_regression.rs`，用本地 exchangeInfo fixture 和隔离子进程验证完整 stdout 能被 JSON 解析；用 config fixture 验证合约 base URL 保留。先运行，记录预期失败。
- [x] 在 `crates/exchange-types/tests/contracts.rs` 定义微秒不丢失、UTC / 北京时间、显式单位、负时间、整数溢出、非法窗口和 JSON 大整数契约，先执行缺失实现的测试。
- [x] 创建 exchange-types；根 workspace 增加该成员与依赖。抽出 `src/lib.rs`；OutputFormat 移至中立配置模块，clap 解析使用 FromStr。原模块类型和输出字段不改。
- [x] 删除 stdout 诊断，日志进入 stderr；移除合约 base URL 的无条件赋值及两处 needless_borrow。
- [x] 运行 `cargo test -p exchange-types`、`cargo test -p cryptool`、`cargo clippy -p cryptool --all-targets -- -D warnings`；针对更改做任务审查。

精确接口由任务简报中的契约定义；基本断言：

```rust
assert_eq!(parse_time("1700000000123456", Some(TimeUnit::Micros)).unwrap().to_micros().unwrap(), 1700000000123456);
assert!(TradeQuery::new(Market::Spot, "BTCUSDT", TradeAnchor::Id(1), 500, 500).is_err());
assert!(serde_json::from_slice::<serde_json::Value>(&output.stdout).is_ok());
```
## 任务 2：原生查询服务与 Axum

- [x] 先写 `tests/exchange_query.rs`：合约 1,000 条分 500 / 500、取 f 而不是 a、现货微秒 header、窗口下界、缺 Key、429 / 冷却、空结果、分页失败与预算。
- [x] 实现 `ExchangeDataService::new(Settings)` 与 `query(TradeQuery)`；类型以 exchange-types 为唯一来源。最大 6 次上游调用、10 秒单请求、30 秒总预算、每进程并发 4；使用请求上下文计数和 checked 算术。
- [x] 增加 HTTP / SOCKS5 / direct 配置及 URL 校验；查询层价格 / 数量解析 Decimal，异常响应限长并分类。
- [x] 先写 `tests/server_contract.rs`：HTTP 和原生结果等价、非法参数 400、未知 API JSON 404、无规则接口、Host 边界。
- [x] 增加可选 `server` / `embed-web` feature、serve 命令和唯一成交 API；UI 只调用 `/api/v1/binance/{market}/historical-trades`。
- [x] 运行 `cargo test -p cryptool --features server`，检查请求数量、目的地和完整输出，再完成审查。

```text
GET /api/v1/binance/usdm/historical-trades?symbol=BTCUSDT&mode=id&trade_id=1000&before=499&after=500
预期：items 长度 1000，上游两次 historicalTrades 请求；meta.upstream_calls = 2。
```

## 任务 3：共享 Dioxus UI 与 Desktop bootstrap

- [x] 添加 exchange-ui package，web / desktop 互斥 feature，原生核心依赖仅 Desktop 启用。只允许 macOS / Windows desktop。
- [x] 先写 UI 状态 / 表单测试：+50 使用整数、非法输入阻止查询、generation 抑制过期结果、时间历史新记录置顶。
- [x] 实现同一个 TradePage、TimeConvertPage 和 AppShell；保留三个业务路由、首页、侧栏、面包屑、页签、菜单搜索、主题和全屏。
- [x] 实现 TradeTable 全部列、maker 颜色、斑马纹、拖动列宽和 100 / 全部显示。
- [x] HttpQueryClient 解析共享 DTO；NativeQueryClient 调用 `ExchangeDataService::query`，不用 localhost 服务。配置失败显示路径与可恢复提示，时间工具仍可使用。
- [x] 集中实现平台偏好与窗口操作；异步查询可取消，关闭窗口释放任务。
- [x] 分别运行 Web WASM check、Desktop check 与 UI 逻辑测试；真实浏览器按功能矩阵验收，macOS 已构建和启动，Desktop 实机视觉与 Windows 验收限制见 `docs/exchange-data-acceptance.md`。

## 任务 4：打包、平台 CI 与最终验收

- [x] 配置 dx Web 构建和 rust-embed staging；验证无需开发服务器、任意目录运行。
- [x] 配置 macOS / Windows Desktop bundle；CI 只包含这两个桌面 OS，校验 WebView2 依赖说明；不加入 Linux Desktop 工作流。
- [x] 更新 README、配置示例、API / 运行说明和图形界面验收清单。
- [x] 运行 native tests / fmt / Clippy、Web WASM 构建、macOS 桌面构建与启动；Windows 无本地执行条件时明确记录 CI / 实机尚待验证，不能声称已通过。
- [x] 完成全分支规格 / 质量审查，修复重要问题并重跑受影响检查；保留可审阅分支，不自动推送或发布。

## 验收账本

- 2026-09-09：用户批准设计，并将 Desktop 限定为 macOS / Windows。
- 工作区：`feat/exchange-data`，基线 `1d05196`。原项目已有 6 个测试；原严格 Clippy 两处问题随任务 1 修正。
