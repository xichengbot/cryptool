# cryptool / Exchange Data

Rust 交易所数据工具，保留原 `cryptool` CLI，增加共享 Dioxus 界面的 Web 与 macOS / Windows 桌面应用。GUI 提供现货历史成交、USD-M 合约历史成交和时间转换；不增加规则、账户或签名界面。原账户命令仍为预留实现。

```text
cryptool CLI ─────────────── 原命令服务
Dioxus Web ── Axum API ─┐
Dioxus Desktop ─────────┴── ExchangeDataService ── Binance
       共享页面、DTO、整数时间转换与窗口校验
```

## Web 与桌面

### 快速索引

| 目标 | 文档 | 构建脚本 | 安装/部署脚本 |
|---|---|---|---|
| macOS 应用 | [构建与安装](docs/build-deploy/app-macos.md) | `scripts/app/macos/build.sh` | `scripts/app/macos/install.sh` |
| Windows 应用 | [构建与安装](docs/build-deploy/app-windows.md) | `scripts/app/windows/build.ps1` | `scripts/app/windows/install.ps1` |
| Ubuntu Web | [构建与部署](docs/build-deploy/web-ubuntu.md) | `scripts/web/build-ubuntu.sh` | `scripts/web/deploy-ubuntu.sh` |
| 全部入口 | [构建与部署总索引](docs/build-deploy/README.md) | — | — |

旧脚本入口继续可用，新的自动构建和重新部署请优先使用索引中的平台脚本。

```sh
rustup target add wasm32-unknown-unknown
cargo install dioxus-cli --version 0.7.10 --locked

# Web：构建页面并嵌入 Axum 程序
scripts/build-web.sh
./target/release/cryptool serve

# Web：监听全部 IPv4 网卡并在后台启动
./scripts/cryptool-web-server.sh
./scripts/cryptool-web-server.sh status
./scripts/cryptool-web-server.sh stop

# macOS：生成独立 .app
scripts/build-desktop.sh
```

Windows 在 PowerShell 中执行 `./scripts/build-desktop.ps1`。不提供 Linux Desktop。构建只需要 Rust/Dioxus 工具链，无需 Node、Vue 或 Tauri。

直接运行 `cryptool serve` 时默认 Web 地址为 `http://127.0.0.1:8080`；局域网启动脚本默认监听 `0.0.0.0:8080` 并在后台运行，支持 `start`、`stop`、`restart`、`status` 和 `run`，优先读取脚本同目录的 `cryptool` 和 `config.toml`。现货 GUI 无需 Key；合约历史成交在本地配置或服务端环境中设置 `BINANCE_API_KEY`。时间转换可离线使用。

详见 [使用、配置、服务器部署与回滚手册](docs/exchange-data-runtime.md) 和 [设计文档](docs/superpowers/specs/2026-09-09-exchange-data-web-integration-design.md)。运行手册包含 Web 自定义端口与 Host、Linux 服务器和局域网访问、systemd / Nginx、macOS / Windows Desktop、原 CLI，以及带验证和回滚的后续部署流程。

## 原 CLI

```sh
cargo run -p cryptool -- --output json market symbol BTCUSDT
cargo run -p cryptool -- --output json market all
cargo run -p cryptool -- --output json market all --status trading
cargo run -p cryptool -- --output json futures-market symbol BTCUSDT
cargo run -p cryptool -- --output json futures-market all --status trading

cargo run -p cryptool -- --config ./config.toml market trades BTCUSDT --trade-id 6533409332 --before 20 --after 20
cargo run -p cryptool -- --config ./config.toml futures-market trades BTCUSDT --trade-id 8068267977 --before 20 --after 20
```

`--status` 支持 `trading`、`halt`、`break`、`all`。配置从 `config.example.toml` 复制，真实 Key / 私钥仅保存在本地或环境变量中。

CLI 保持默认前后各 20、总上限 500 和原有输出字段。GUI 默认各 50、总上限 1,000，合约按每页 500 条分页。GUI/API 的 ID 和时间不经浮点转换，JSON 使用字符串保留大整数；现货使用微秒，合约使用毫秒。

USD-M 按时间定位使用 Binance `aggTrades`，仅支持最近 2 天；已知成交 ID 时，`historicalTrades` 可查询最近 1 个月。更早的逐笔需要使用 [Binance Public Data](https://github.com/binance/binance-public-data) 的 USD-M `daily/monthly/trades` 文件，当前程序尚未集成该归档数据源。

## 开发验证

```sh
cargo test -p exchange-types
cargo test -p cryptool --features server
cargo test -p exchange-ui --lib --no-default-features
cargo check -p exchange-ui --target wasm32-unknown-unknown --no-default-features --features web
cargo check -p exchange-ui --no-default-features --features desktop
```

Web / Desktop features 必须分别构建，不使用 `--all-features`。静态资源嵌入构建需先执行 Web 构建脚本。CI 的桌面矩阵仅 macOS 和 Windows；本地验证结果与未验证项记录在 [验收记录](docs/exchange-data-acceptance.md)。
