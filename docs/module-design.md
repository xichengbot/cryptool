# 模块设计

本文按 2026-09-11 的 `main` 分支（`5093712`）核对，描述当前实现，不沿用集成前的单 crate 假设。

## exchange-types

Native 与 WASM 的唯一数据契约来源：

- `Market`：spot / usdm。
- `TimeUnit`、`Timestamp`：显式 s/ms/us，checked 整数换算，JSON value 为字符串。
- `TradeAnchor`：u64 原始成交 ID 或显式时间；ID 使用字符串 JSON。
- `TradeQuery`：交易对、锚点、前后数量。构造时规范化 symbol，服务入口再次 validate；总窗口不超过 1,000。
- `TradeRow`：保留 ID、价格/数量十进制文本、时间、maker 与可选 bestMatch。
- `TradeWindowMeta`：请求、确认锚点、实际行数、调用次数、partial/boundary。
- `QueryError`：稳定错误码、用户可读消息、重试属性和冷却时间。

时间解析接受明确单位的整数、可识别长度的自动整数、RFC3339 或默认上海时区的本地时间。无法判断单位、整数溢出、微秒以下精度均返回错误。UTC/北京时间格式化不依赖机器时区，保留历史秒级时区偏移。

## cryptool 原生核心

### 原 CLI 服务

`app::{market,futures_market,account}`、原 `domain`、`output` 保持既有调用路径。CLI 的历史窗口默认20/20、上限500；规则状态筛选、Decimal 映射与 JSON/table 字段保持。

账户 `balance` 仍返回 `NotImplemented`。Ed25519 签名仍为已有预留能力，不接入新 GUI。

### ExchangeDataService

```rust
let service = ExchangeDataService::new(settings)?;
let window = service.query(query).await?;
```

服务可 Clone，共享连接池、查询并发限制与限流冷却。职责为验证查询、按市场选择认证、确认时间锚点、顺序分页、去重、控制预算并生成真实结果元数据。

算法使用有序 BTreeMap 保留和筛选原始成交；页大小现货最多1000、合约最多500。游标使用 checked_add，前沿使用 saturating_sub。短页、空页、无进展、缺锚点、预算耗尽或后续分页异常均有明确结果路径；尚未确认时间锚点时不伪造完整结果。

### BinanceClient / history

`BinanceClient` 持有 native Settings 与 reqwest 连接池。原 CLI 请求方法保留；新的 history adapter 负责：

- 现货 `/api/v3/` 和合约 `/fapi/v1/` 地址选择。
- 现货 `X-MBX-TIME-UNIT: MICROSECOND`；合约历史成交才带 Key。
- 聚合成交读取 `f`，不使用 `a` 作为原始成交 ID。
- HTTP 状态/Retry-After 分类、流式限长读取、严格 Decimal 和时间检查。
- 不把原始上游正文或凭据放入 GUI 错误。

### config

Settings 支持默认值与部分 TOML。`BINANCE_API_KEY` / `BINANCE_PRIVATE_KEY`、CLI `--base-url` / `--output` 覆写方式保留；配置的合约基地址不再被无条件覆盖。

ProxySettings 表达 auto/direct/http/socks5，以及独立 URL/username/password。显式 URL 校验协议、主机、路径和凭据位置。QuerySettings 控制并发、单请求/总超时和调用预算。配置 Debug 隐藏敏感值。

## Axum adapter

通过 `server` feature 启用；`embed-web` 在此基础上嵌入已构建资源。

- `router(service, ServerOptions)` 创建可供测试/启动的 Router。
- `ServerOptions.allowed_hosts` 默认 localhost/127.0.0.1/::1；`static_dir` 可指定外部构建目录。
- 成交 API 只做 HTTP 参数校验、共享查询构造、服务调用和状态码映射。
- 未知 API 返回 JSON404，不落到 SPA 的 index.html。
- 业务深链接返回页面；未知静态资源返回404。
- serve 命令负责监听与 Ctrl-C 优雅关闭，网络配置错误在监听前暴露。

## exchange-ui

| 模块 | 职责 |
|---|---|
| routes / shell | 页面路由、侧栏、面包屑、页签、搜索、主题与全屏 |
| pages/trade | 同一表单按 Market 复用；校验、加载、取消、结果/错误/部分状态 |
| components/trade_table | 全字段展示、原始单位、锚点、列宽与本地分页 |
| pages/time_convert | 共享时间函数、转换输出和历史列表，无网络依赖 |
| state | 可测试表单、+50、时间历史、请求 generation |
| client/http | 浏览器同源 Axum GET，解析共享 DTO |
| client/native | 直接调用 ExchangeDataService |
| platform | Web Storage / 原生偏好文件、全屏与配置发现 |

原生配置发现使用 ProjectDirs；显式配置错误保留可恢复状态，不阻止打开窗口或使用时间工具。桌面渲染只启用 macOS / Windows；Web 和 Desktop features 分开编译。

## 构建与部署脚本

| 模块 | 职责 |
|---|---|
| `scripts/lib/common.sh` | 项目路径、依赖检查、受限生成目录清理、Cargo target 解析和 SHA-256 |
| `scripts/app/macos` | 清理 Dioxus staging、构建 `.app`、ad-hoc 签名、plist/ICNS/Mach-O 验证及安装 |
| `scripts/app/windows` | Windows staging 清理、NSIS 构建、SHA-256 输出及安装程序执行 |
| `scripts/web/build-local.sh` | 构建 Dioxus WASM，并生成内嵌资源的本机 Axum 二进制 |
| `scripts/web/build-ubuntu.sh` | 使用 cargo-zigbuild 交叉构建 ELF，限制最高 GLIBC 2.17 |
| `scripts/web/package-ubuntu.sh` | 生成自包含运行目录、tar.gz 和 SHA-256 |
| `scripts/web/deploy-ubuntu.sh` | root 安装、服务用户、配置保留、权限和符号链接边界 |
| `scripts/web/server.sh` | 前台/后台启动、PID、日志、状态、重启、停止及同目录发现 |

顶层 `build-desktop.*`、`build-web.sh` 和 `cryptool-web-server.sh` 只保留兼容转发逻辑。脚本布局和帮助接口由 `tests/scripts/build-deploy-layout.sh` 校验。

测试与运行步骤见 [验收记录](exchange-data-acceptance.md) 和 [运行文档](exchange-data-runtime.md)。
