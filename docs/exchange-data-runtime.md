# Exchange Data 运行与接口

构建和部署入口已按平台拆分，先阅读[构建与部署索引](build-deploy/README.md)。macOS、Windows
应用和 Ubuntu Web 的完整步骤分别见对应平台文档；本文保留配置、接口和运行机制说明。

## 运行边界

Web 使用 Dioxus WASM 页面与同源 Axum API；macOS / Windows Desktop 使用同一组页面，直接调用 Rust 查询服务，不启动本地 HTTP 服务。CLI 的规则、账户预留命令和签名能力维持命令行入口，没有新增对应 GUI/API。账户查询仍为原项目的未实现状态。

时间转换完全在客户端 Rust 中执行，可离线使用。桌面需要系统 WebView：macOS 使用 WKWebView，Windows 使用 WebView2。项目不构建 Linux 桌面程序。

## 构建工具

项目用 `rust-toolchain.toml` 固定已验证的 Rust 1.98.0；CI 使用同一版本。安装 WASM target 和与项目一致的 Dioxus CLI：

```sh
rustup target add wasm32-unknown-unknown
cargo install dioxus-cli --version 0.7.10 --locked
```

无需 Node、npm、Vue 或 Tauri。Rust/WASM 与桌面渲染器的 feature 分别启用，不能使用 `--all-features`。

## 配置与凭据

先在仓库根目录创建本地配置：

```sh
cp config.example.toml config.toml
```

Windows PowerShell 使用：

```powershell
Copy-Item .\config.example.toml .\config.toml
```

现货 GUI 查询不要求 Key；USD-M 历史成交需要 Binance API Key。GUI 不使用私钥，不增加签名或账户入口。按需编辑：

```toml
[binance]
api_key = "your-binance-api-key"
private_key = ""

usdm_futures_base_url = "https://fapi.binance.com"
base_url = "https://api.binance.com"
recv_window = 5000
timeout_ms = 10000
```

也可通过 `BINANCE_API_KEY` 和 `BINANCE_PRIVATE_KEY` 环境变量覆盖配置。Web 的配置和凭据只由 Axum 服务端读取；Desktop 只由本机 Rust 进程读取。凭据不会进入 API 响应、DOM、localStorage 或页面状态。`config.toml` 已被 Git 忽略，不应将真实凭据提交到仓库。

Desktop 未指定 `--config` 时会查找以下文件；界面会显示实际使用的路径：

| 平台 | 默认配置文件 |
|---|---|
| macOS | `~/Library/Application Support/com.cryptool.exchange-data/config.toml` |
| Windows | `%APPDATA%\cryptool\exchange-data\config\config.toml` |

将配置安装到默认位置后，Finder、Launchpad、Spotlight 或 Windows 开始菜单启动的应用也能自动读取。macOS：

```sh
mkdir -p "$HOME/Library/Application Support/com.cryptool.exchange-data"
cp config.toml "$HOME/Library/Application Support/com.cryptool.exchange-data/config.toml"
```

Windows PowerShell：

```powershell
$ExchangeDataConfigDir = Join-Path $env:APPDATA 'cryptool\exchange-data\config'
New-Item -ItemType Directory -Force $ExchangeDataConfigDir | Out-Null
Copy-Item .\config.toml (Join-Path $ExchangeDataConfigDir 'config.toml')
```

文件不存在时使用无凭据的默认配置，现货查询和时间转换仍可使用。显式传入但无法读取或解析的配置会在界面中显示错误，时间转换仍可离线使用。主题等界面偏好保存于独立文件，不与凭据混用。

### 代理

代理只用于 Rust 进程访问 Binance，不代理浏览器到 Web 服务的连接。四种模式如下：

```toml
# 读取 HTTP_PROXY、HTTPS_PROXY、ALL_PROXY、NO_PROXY 等进程环境变量
[binance.proxy]
mode = "auto"
```

```toml
# 不使用系统或进程代理
[binance.proxy]
mode = "direct"
```

```toml
# 显式 HTTP(S) 代理
[binance.proxy]
mode = "http"
url = "http://127.0.0.1:7890"
```

```toml
# 显式 SOCKS5 代理；socks5h 让代理端解析域名
[binance.proxy]
mode = "socks5"
url = "socks5h://127.0.0.1:1080"
# username = "local-user"
# password = "local-password"
```

`auto` 和 `direct` 不能配置 URL 或认证；`http` 和 `socks5` 必须配置匹配模式的 URL。用户名和密码必须同时提供，并放在独立字段中，不能嵌入 URL。修改配置后重启对应进程。

## Web 使用方法

### 使用已有发布二进制

仓库已提供 `dist/web/cryptool` 时，从仓库根目录启动：

```sh
./dist/web/cryptool --config ./config.toml serve
```

默认地址为 `http://127.0.0.1:8080`。自定义端口：

```sh
./dist/web/cryptool --config ./config.toml serve --listen 127.0.0.1:9090
```

允许局域网或反向代理访问时，非回环监听必须显式声明可接受的 Host，可重复传入：

```sh
./dist/web/cryptool \
  --config ./config.toml \
  serve \
  --listen 0.0.0.0:9090 \
  --allow-host 192.168.1.100 \
  --allow-host exchange.example.com
```

公开部署应由反向代理提供 HTTPS 和访问认证，并仅配置实际使用的 Host。服务默认不接受任意 Host，不启用跨源 CORS，也不提供多租户用户管理。

### 从源码构建内嵌版本

```sh
scripts/build-web.sh
./target/release/cryptool --config ./config.toml serve
```

脚本先构建 Dioxus 页面至 `web-dist/public/`，再使用 `embed-web` 构建 Axum 程序。最终 `target/release/cryptool` 已内嵌页面资源，可以复制到任意启动目录，部署时不需要同时复制 `web-dist/`。

开发时也可分开构建和托管静态文件；`--web-dir` 必须指向包含 `index.html` 的目录：

```sh
cd apps/exchange-ui
dx bundle --web --release --debug-symbols=false --no-default-features --features web --out-dir ../../web-dist
cd ../..
cargo run -p cryptool --features server -- serve --web-dir ./web-dist/public
```

直接由 Axum 托管时使用发布页面，避免调试页面尝试连接 Dioxus 热更新服务。按 `Ctrl+C` 可优雅停止服务。

## Linux 服务器部署与局域网访问

Linux 服务器只运行 Web/Axum 服务，不涉及 Linux Desktop。`scripts/build-web.sh` 生成的是当前构建主机和 CPU 架构的原生二进制：macOS 构建的文件不能在 Linux 运行，`x86_64` 和 `aarch64` 产物也不能混用。生产环境应在相同架构、相同或更旧的 Linux 发行版上构建，或使用与生产环境兼容的 Linux CI 构建。

### 1. 安装构建环境

Ubuntu/Debian 可以先安装基础工具：

```sh
sudo apt update
sudo apt install -y build-essential pkg-config curl unzip
```

安装 Rust 后，使用项目固定版本：

```sh
rustup toolchain install 1.98.0
rustup target add --toolchain 1.98.0 wasm32-unknown-unknown
cargo +1.98.0 install dioxus-cli --version 0.7.10 --locked
```

检查服务器架构和工具：

```sh
uname -m
rustc +1.98.0 --version
dx --version
```

### 2. 构建并在局域网启动

从仓库根目录执行：

```sh
./scripts/build-web.sh
./scripts/cryptool-web-server.sh
```

启动脚本默认：

- 优先使用脚本同目录的 `cryptool` 和 `config.toml`，适合把三个文件一起复制到服务器。
- 同目录没有 `cryptool` 时，回退到仓库的 `target/release/cryptool`，方便源码目录直接运行。
- 同目录没有 `config.toml` 时，回退到仓库根目录配置；两处都没有时使用程序内置默认配置。
- 监听 `0.0.0.0:8080`，接受服务器全部 IPv4 网卡的连接。
- 自动发现非回环局域网 IPv4，将其作为 Axum 的 `--allow-host`，并打印可访问 URL。
- 不带参数或使用 `start` 时在后台运行，PID 和日志默认写入脚本同目录。
- 使用 `run` 时以前台方式运行，适合 systemd、容器和临时调试。

输出类似：

```text
Starting Exchange Data Web on 0.0.0.0:8080
LAN URL: http://192.168.1.20:8080/
Exchange Data Web started in background (PID 12345)
Log: /data/cryptool/cryptool.log
```

进程管理命令：

```sh
./scripts/cryptool-web-server.sh start    # 后台启动；start 可省略
./scripts/cryptool-web-server.sh status   # 查看状态、PID 和日志路径
./scripts/cryptool-web-server.sh restart  # 重启
./scripts/cryptool-web-server.sh stop     # 正常发送 TERM 并等待退出
./scripts/cryptool-web-server.sh run      # 前台运行，按 Ctrl+C 停止
```

重复启动会返回错误，不会创建第二个进程。`status` 在服务运行时返回 0，停止时返回 3，便于监控脚本判断。所有管理命令应使用同一个系统用户执行；若使用 `sudo` 启动，后续 `status`、`restart` 和 `stop` 也应使用 `sudo`。

局域网其他用户应访问脚本打印的 IP，不能在浏览器中访问 `0.0.0.0`。可以从另一台机器验证：

```sh
curl --fail http://192.168.1.20:8080/
```

脚本支持以下覆盖变量：

| 环境变量 | 说明 | 默认值 |
|---|---|---|
| `CRYPTOOL_BIN` | Web/Axum 二进制 | 同目录 `cryptool`，不存在时回退 `target/release/cryptool` |
| `CRYPTOOL_CONFIG` | 配置文件；显式空值表示不传 `--config` | 同目录 `config.toml`，其次仓库根目录配置 |
| `CRYPTOOL_LISTEN` | 完整监听地址 | `0.0.0.0:8080` |
| `CRYPTOOL_ALLOW_HOSTS` | 逗号或空格分隔的 IP/域名 | 自动发现局域网 IPv4 |
| `CRYPTOOL_PID_FILE` | 后台服务 PID 文件 | 脚本同目录 `cryptool.pid` |
| `CRYPTOOL_LOG_FILE` | 后台服务日志 | 脚本同目录 `cryptool.log` |

自定义端口、配置和 Host：

```sh
CRYPTOOL_LISTEN="0.0.0.0:9090" \
CRYPTOOL_CONFIG="/etc/cryptool/config.toml" \
CRYPTOOL_ALLOW_HOSTS="192.168.1.20,exchange.local" \
./scripts/cryptool-web-server.sh
```

使用安装到其他位置的二进制：

```sh
CRYPTOOL_BIN="/opt/cryptool/bin/cryptool" \
CRYPTOOL_CONFIG="/etc/cryptool/config.toml" \
CRYPTOOL_ALLOW_HOSTS="192.168.1.20" \
./scripts/cryptool-web-server.sh
```

如果自动发现失败，脚本不会放开任意 Host，而是退出并提示设置 `CRYPTOOL_ALLOW_HOSTS`。检查监听和本机地址：

```sh
ip -4 addr
ss -lntp | grep ':8080'
curl --fail http://127.0.0.1:8080/
```

若出现 `cannot execute binary file: Exec format error`，说明二进制操作系统或 CPU 架构与服务器不匹配。先检查：

```sh
uname -s
uname -m
file ./cryptool
```

Ubuntu 上的正确产物应显示 `ELF`，而不是 macOS 的 `Mach-O` 或 Windows 的 `PE32`；其 `x86-64` / `aarch64` 架构也必须与 `uname -m` 对应。不能把 macOS 的 `dist/web/cryptool` 直接复制到 Ubuntu，应在兼容的 Linux 环境重新执行：

```sh
./scripts/build-web.sh
```

新版启动脚本会在 Linux 上提前拒绝明显的 Mach-O/PE 二进制，并给出重编译提示。

### 3. 防火墙

局域网直连时只向实际网段开放应用端口。以下 UFW 示例假设局域网为 `192.168.1.0/24`：

```sh
sudo ufw allow from 192.168.1.0/24 to any port 8080 proto tcp
sudo ufw status
```

云服务器还需要在安全组中使用相同来源网段和端口。通过本机 Nginx 反向代理时，Axum 应监听 `127.0.0.1`，无需向网络开放 8080，只开放 Nginx 的 80/443。

应用没有多用户登录和权限隔离。局域网直连意味着该网段内能访问端口的用户都可以发起查询；跨不可信网络使用时必须增加反向代理认证和 TLS。

### 4. 同目录安装程序和配置

以下以 `/data/cryptool` 为部署目录，将二进制、配置和启动脚本放在一起：

```sh
sudo useradd --system --no-create-home --shell /usr/sbin/nologin cryptool
sudo install -d -o root -g cryptool -m 0750 /data/cryptool
sudo install -o root -g cryptool -m 0750 target/release/cryptool /data/cryptool/cryptool
sudo install -o root -g cryptool -m 0750 scripts/cryptool-web-server.sh /data/cryptool/cryptool-web-server.sh
sudo install -o root -g cryptool -m 0640 config.toml /data/cryptool/config.toml
sudo install -o cryptool -g cryptool -m 0640 /dev/null /data/cryptool/cryptool.pid
sudo install -o cryptool -g cryptool -m 0640 /dev/null /data/cryptool/cryptool.log
```

预先创建的 PID 和日志文件允许非 root 服务用户使用后台管理命令。进入目录即可启动，不需要设置二进制和配置路径：

```sh
cd /data/cryptool
sudo -u cryptool ./cryptool-web-server.sh
```

也可把 API Key 放入 `/data/cryptool/cryptool.env`，避免写进 TOML：

```text
BINANCE_API_KEY=your-binance-api-key
```

权限设置：

```sh
sudo chown root:cryptool /data/cryptool/cryptool.env
sudo chmod 0640 /data/cryptool/cryptool.env
```

### 5. systemd 托管

创建 `/etc/systemd/system/cryptool.service`：

```ini
[Unit]
Description=Cryptool Exchange Data Web
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=cryptool
Group=cryptool
WorkingDirectory=/data/cryptool
EnvironmentFile=-/data/cryptool/cryptool.env
Environment=CRYPTOOL_LISTEN=0.0.0.0:8080
Environment=CRYPTOOL_ALLOW_HOSTS=192.168.1.20,exchange.local
ExecStart=/data/cryptool/cryptool-web-server.sh run
Restart=on-failure
RestartSec=3
NoNewPrivileges=true
PrivateTmp=true
ProtectHome=true
ProtectSystem=strict

[Install]
WantedBy=multi-user.target
```

将示例 IP/域名改成服务器实际值，然后启动：

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now cryptool
sudo systemctl status cryptool --no-pager
sudo journalctl -u cryptool -n 100 --no-pager
```

### 6. Nginx、HTTPS 和认证

若使用 Nginx，把 systemd 中监听地址改为 `127.0.0.1:8080`，并将公开域名加入 `CRYPTOOL_ALLOW_HOSTS`。Nginx 示例：

```nginx
server {
    listen 80;
    server_name exchange.example.com;

    auth_basic "Exchange Data";
    auth_basic_user_file /etc/nginx/.htpasswd;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

Axum 校验原始 Host，因此 `server_name`、浏览器访问域名和 `CRYPTOOL_ALLOW_HOSTS` 必须一致。检查后重新加载 Nginx：

```sh
sudo nginx -t
sudo systemctl reload nginx
```

正式公网入口还应配置 TLS 证书和限流。默认不启用跨源 CORS，页面和 API 应保持同源。

## macOS Desktop 使用方法

已有应用可以从终端打开：

```sh
open dist/macos/ExchangeData.app
```

也可以在 Finder 中进入 `dist/macos/` 后双击 `ExchangeData.app`，或使用绝对路径：

```sh
open /absolute/path/to/dist/macos/ExchangeData.app
```

直接运行内部程序可以在终端查看日志，并显式指定配置：

```sh
dist/macos/ExchangeData.app/Contents/MacOS/exchange-data --config ./config.toml
```

通过 `open` 传递配置参数：

```sh
open dist/macos/ExchangeData.app --args --config /absolute/path/to/config.toml
```

安装到系统应用目录后，可通过 Finder、Launchpad 或 Spotlight 启动：

```sh
ditto dist/macos/ExchangeData.app /Applications/ExchangeData.app
open -a ExchangeData
```

从源码重新构建：

```sh
scripts/build-desktop.sh
```

产物位于 `dist/macos/ExchangeData.app`。当前应用未包含签名和公证；首次被 Gatekeeper 拦截时，在确认是自己构建的产物后，可在 Finder 中右键应用并选择“打开”。

## Windows Desktop 使用方法

Windows 构建机需要 Rust 1.98.0、Dioxus CLI 0.7.10、Microsoft Edge WebView2 Runtime，以及带 MSVC 和 Windows SDK 的 Visual Studio 2022 Build Tools“使用 C++ 的桌面开发”工作负载。

在 PowerShell 中安装 Rust 侧工具并构建：

```powershell
rustup toolchain install 1.98.0
cargo install dioxus-cli --version 0.7.10 --locked
./scripts/build-desktop.ps1
```

NSIS 安装程序生成在 `dist\windows\`。可用下面的命令定位后双击安装：

```powershell
Get-ChildItem .\dist\windows -Recurse -Filter *.exe
```

当前安装程序未签名，Windows SmartScreen 可能显示未知发布者。确认产物由当前源码构建后，再选择“更多信息 → 仍要运行”。

开发或验收时可以跳过安装，直接从源码运行并指定配置：

```powershell
cargo run -p exchange-ui `
  --no-default-features `
  --features desktop `
  -- `
  --config .\config.toml
```

Windows Desktop 直接调用 Rust 查询服务，不启动 Axum；`serve`、`--listen` 和 `--allow-host` 只适用于 Web 服务。

## 原 CLI 使用方法

原有命令保持不变，例如：

```sh
cargo run -p cryptool -- --output json market symbol BTCUSDT
cargo run -p cryptool -- --output json market all --status trading
cargo run -p cryptool -- --output json futures-market symbol BTCUSDT

cargo run -p cryptool -- \
  --config ./config.toml \
  market trades BTCUSDT \
  --trade-id 6533409332 \
  --before 20 \
  --after 20

cargo run -p cryptool -- \
  --config ./config.toml \
  futures-market trades BTCUSDT \
  --time 2026-07-26T12:34:56Z \
  --before 20 \
  --after 20
```

## 下次重新构建与部署

### 1. 更新与验证源码

先确认当前分支和本地修改，再更新目标分支。不要在有未提交修改时直接覆盖文件：

```sh
git status --short
git branch --show-current
git pull --ff-only
```

运行与 CI 一致的核心检查：

```sh
cargo fmt --all -- --check
cargo test -p cryptool -p exchange-types -p exchange-ui \
  --no-default-features \
  --features cryptool/server,exchange-ui/desktop \
  --locked
cargo clippy -p cryptool -p exchange-types -p exchange-ui \
  --all-targets \
  --no-default-features \
  --features cryptool/server,exchange-ui/desktop \
  --locked -- -D warnings
```

Web 和 Desktop feature 必须分别构建，不能使用 `--all-features`。Windows 应在 Windows 上构建；macOS 应在 macOS 上构建。

### 2. 重新部署 Web

构建新的独立二进制：

```sh
scripts/build-web.sh
```

在替换正式服务前，使用临时端口检查新程序能够独立启动并返回页面：

```sh
cp target/release/cryptool /tmp/cryptool.new
/tmp/cryptool.new \
  --config /absolute/path/to/config.toml \
  serve --listen 127.0.0.1:18080 \
  >/tmp/cryptool-smoke.log 2>&1 &
smoke_pid=$!
curl --fail http://127.0.0.1:18080/
kill "$smoke_pid"
```

生产目录和服务名由部署环境决定。若按前述服务器章节使用 `/data/cryptool/` 和 systemd 服务 `cryptool`，替换前保留上一版：

```sh
sudo systemctl stop cryptool
sudo cp /data/cryptool/cryptool /data/cryptool/cryptool.previous
sudo install -o root -g cryptool -m 0750 target/release/cryptool /data/cryptool/cryptool
sudo install -o root -g cryptool -m 0750 scripts/cryptool-web-server.sh /data/cryptool/cryptool-web-server.sh
sudo systemctl start cryptool
sudo systemctl status cryptool --no-pager
curl --fail http://127.0.0.1:8080/
```

`config.toml` 应放在二进制之外并由服务启动参数显式指定，重新部署时不要覆盖。反向代理、TLS 和认证配置也不需要随二进制重复部署。

如果启动或健康检查失败，恢复上一版：

```sh
sudo systemctl stop cryptool
sudo cp /data/cryptool/cryptool.previous /data/cryptool/cryptool
sudo systemctl start cryptool
sudo systemctl status cryptool --no-pager
```

若部署环境不使用 systemd，保持相同顺序：停止旧进程、备份旧二进制、替换、启动、检查首页、失败时回滚。

### 3. 重新部署 macOS Desktop

```sh
scripts/build-desktop.sh
```

先退出正在运行的 ExchangeData，再备份并替换应用：

```sh
osascript -e 'quit app "ExchangeData"'
backup_stamp=$(date +%Y%m%d-%H%M%S)
if [ -d /Applications/ExchangeData.app ]; then
  mv /Applications/ExchangeData.app "/Applications/ExchangeData-$backup_stamp.app"
fi
ditto dist/macos/ExchangeData.app /Applications/ExchangeData.app
open -a ExchangeData
```

配置存放在应用包之外，替换 `.app` 不会覆盖配置。验收现货查询、需要 Key 的 USD-M 查询、时间转换、主题和全屏。失败时退出新应用，将带时间戳的上一版改回 `ExchangeData.app`。

### 4. 重新部署 Windows Desktop

在 Windows PowerShell 中重新构建安装程序：

```powershell
./scripts/build-desktop.ps1
Get-ChildItem .\dist\windows -Recurse -Filter *.exe
```

保留上一版安装程序，运行新 NSIS 安装程序完成升级，然后从开始菜单启动。配置位于安装目录之外，升级时不要删除 `%APPDATA%\cryptool\exchange-data\config\config.toml`。验收项目与 macOS 相同；失败时卸载当前版本并运行保留的上一版安装程序。

### 5. CI 产物

`.github/workflows/verify.yml` 会在 macOS 和 Windows 上执行检查并分别构建桌面产物，随后上传 `exchange-data-macOS` 或 `exchange-data-Windows` artifact。当前工作流只构建和保存可测试产物，不会自动签名、公证、发布或替换线上服务；正式部署仍按以上步骤执行。

## 查询 API

```text
GET /api/v1/binance/spot/historical-trades?symbol=BTCUSDT&mode=id&trade_id=1000&before=50&after=50
GET /api/v1/binance/usdm/historical-trades?symbol=BTCUSDT&mode=time&time=1700000000123&time_unit=ms&before=50&after=50
```

`market` 只接受 `spot` / `usdm`；`mode` 只接受 `id` / `time`。两种锚点互斥；TIME 模式必须给出 `time_unit=s|ms|us`。未知、重复或非法参数返回结构化错误。

成功响应是 `TradeWindow { items, meta }`。ID、价格、数量和 `timestamp.value` 都通过字符串传输；时间对象另带显式 `unit`。现货原始成交精度为微秒，合约为毫秒。客户端不得用浮点转换 ID、时间或 Decimal 字段。

`meta` 包括原始查询、确认后的 `anchor_id`、请求和收到的数量、实际前后行数、上游调用次数，以及 `partial` / `boundary`。必须展示部分结果状态，不能将分页失败或交易所保留边界当作完整查询。

| 错误码 | HTTP | 含义 |
|---|---:|---|
| INVALID_INPUT | 400 | 参数、窗口或时间不合法 |
| SYMBOL_NOT_FOUND | 404 | 交易对不存在 |
| RATE_LIMITED | 429 | 本地并发额度或交易所冷却；带 Retry-After |
| CREDENTIALS_NOT_CONFIGURED | 503 | 合约 Key 缺失、无效或权限不足 |
| CONFIGURATION | 503 | 原生服务配置不可用 |
| UPSTREAM_ERROR | 502 | 上游网络或数据异常 |
| QUERY_BUDGET_EXCEEDED | 504 | 超出时间或请求预算 |
| CANCELLED | 408 | 查询取消 |

## 预算与窗口语义

| 限制 | CLI | GUI / API |
|---|---:|---:|
| 默认前 / 后数量 | 20 / 20 | 50 / 50 |
| 总窗口上限（含锚点） | 500 | 1,000 |
| USD-M 单页上限 | 原 CLI 行为 | 500，按 ID 顺序分页 |
| 上游调用总预算 | 原 CLI 行为 | 默认及硬上限 6 次，含重试 |
| 单请求超时 | 原配置 | 默认 10 秒，与原配置取较小值 |
| 一次查询总超时 | 原 CLI 行为 | 默认 30 秒 |
| 同时查询 | 原 CLI 行为 | 默认 4 个，超额立即返回限流 |

TIME 查询先在 `[T, T+1秒]` 请求聚合成交，使用 `f`（原始成交 ID）定位；随后检查原始成交时间，避免将聚合 ID 当作成交 ID，或将聚合内较早成交当作时间锚点。通常需要一次定位和至少一次成交请求；纠正与分页都计入预算。

Binance USD-M `aggTrades` 的时间查询窗口仅覆盖最近 2 天；超出范围时 API 返回 `INVALID_INPUT` 和明确提示。已知成交 ID 时，`historicalTrades` 可读取最近 1 个月的数据。更早的逐笔需要下载 [Binance Public Data](https://github.com/binance/binance-public-data) 的 `data/futures/um/{daily|monthly}/trades` 文件；当前服务未接入该归档，因此不会把 Kline 当作逐笔数据返回。

返回窗口从锚点 ID 减去 before 开始；下界饱和到 0，实际行数由上游可用数据决定。短页、无进展、缺锚点、分页失败和边界均显式标记。查询取消会丢弃在途 HTTP future 并释放并发名额。429/418 按 Retry-After 冷却，不循环忙重试；瞬时网络错误最多重试一次。

单个上游响应最多接收 2 MiB，窗口最多 1,000 行。此限制用于控制内存和上游开销，不代表实测吞吐或交易所允许的最大请求频率。
