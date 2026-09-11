# Web 局域网启动脚本实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 提供一个可直接在 macOS/Linux 服务器管理 Web 服务的脚本，让局域网设备通过服务器 IPv4 访问，并把服务器构建、托管、更新和回滚步骤写入运行手册。

**架构：** `scripts/cryptool-web-server.sh` 负责解析环境、发现 Host、校验文件及管理进程。默认后台启动并维护 PID/日志；`run` 使用 `exec` 前台运行。脚本契约测试用假二进制验证参数和进程生命周期；systemd 使用 `run` 模式。

**技术栈：** Bash 3.2 兼容语法、现有 Axum CLI、shell 契约测试、Markdown。

---

## 文件结构

- 创建 `scripts/cryptool-web-server.sh`：局域网 Host 发现、启动参数装配、后台管理和前台执行。
- 创建 `tests/fixtures/capture-args.sh`：记录脚本传给假 `cryptool` 的参数。
- 创建 `tests/cryptool-web-server-script.sh`：覆盖显式配置、自动发现和失败边界。
- 修改 `docs/exchange-data-runtime.md`：整理现有使用说明，增加 Linux 服务器部署完整流程。
- 修改 `README.md`：增加局域网启动脚本入口。

### 任务 1：固定启动脚本契约

**文件：**
- 创建：`tests/fixtures/capture-args.sh`
- 创建：`tests/cryptool-web-server-script.sh`

- [x] **步骤 1：创建参数捕获程序**

捕获程序要求 `CRYPTOOL_TEST_ARGS_FILE`，把每个参数逐行写入该文件：

```bash
#!/usr/bin/env bash
set -euo pipefail
: "${CRYPTOOL_TEST_ARGS_FILE:?CRYPTOOL_TEST_ARGS_FILE is required}"
printf '%s\n' "$@" > "$CRYPTOOL_TEST_ARGS_FILE"
```

- [x] **步骤 2：编写失败的脚本契约测试**

测试创建临时目录，并依次断言：

```text
显式 config/listen/两个 Host → 参数顺序准确
CRYPTOOL_CONFIG 为空 → 不传 --config
假 hostname -I → 自动加入有效非回环 IPv4，并去重
非回环监听且无法发现 Host → 非零退出并提示 CRYPTOOL_ALLOW_HOSTS
二进制不存在 → 非零退出
显式配置不存在 → 非零退出
```

测试通过 `/bin/bash scripts/cryptool-web-server.sh` 调用被测脚本，并用绝对路径假二进制记录参数。

- [x] **步骤 3：验证红灯**

运行：

```bash
bash tests/cryptool-web-server-script.sh
```

预期：FAIL，原因是 `scripts/cryptool-web-server.sh` 尚不存在。

- [x] **步骤 4：提交测试**

```bash
git add tests/fixtures/capture-args.sh tests/cryptool-web-server-script.sh
git commit -m "test: define LAN web launcher contract"
```

### 任务 2：实现局域网启动脚本

**文件：**
- 创建：`scripts/cryptool-web-server.sh`
- 测试：`tests/cryptool-web-server-script.sh`

- [x] **步骤 1：实现配置和文件校验**

使用以下默认值和覆盖规则：

```bash
script_dir=$(cd -- "${BASH_SOURCE[0]%/*}" && pwd)
project_root=$(cd -- "$script_dir/.." && pwd)
# CRYPTOOL_BIN 未设置时优先使用 "$script_dir/cryptool"，
# 不存在时回退 "$project_root/target/release/cryptool"。
listen_address=${CRYPTOOL_LISTEN:-0.0.0.0:8080}
```

`CRYPTOOL_CONFIG` 已设置时使用其值；未设置时依次查找脚本同目录和仓库根目录的 `config.toml`。非空配置必须是普通可读文件，二进制必须是普通可执行文件。Linux 会提前拒绝明显的 Mach-O/PE 二进制。

- [x] **步骤 2：实现 Host 解析和自动发现**

将 `CRYPTOOL_ALLOW_HOSTS` 中的逗号替换为空格并去重；未显式设置时按 `hostname -I`、`ip -o -4 addr show scope global`、`ipconfig getifaddr en0/en1` 的顺序发现 IPv4。IPv4 必须有四个 0–255 的十进制段，且不能以 `127.` 开头。

- [x] **步骤 3：实现安全失败和前台启动**

监听地址不是 `127.*`、`localhost:*` 或 `[::1]:*` 且 Host 列表为空时退出。成功时打印监听地址和局域网 URL，并执行：

```bash
exec "$binary_path" "${config_args[@]}" serve \
  --listen "$listen_address" "${host_args[@]}"
```

- [x] **步骤 4：验证绿灯与语法**

运行：

```bash
bash tests/cryptool-web-server-script.sh
bash -n scripts/cryptool-web-server.sh tests/cryptool-web-server-script.sh tests/fixtures/capture-args.sh
```

预期：全部通过，测试输出显示所有契约场景通过。

- [x] **步骤 5：提交实现**

```bash
git add scripts/cryptool-web-server.sh
git commit -m "feat: add LAN web server launcher"
```

### 任务 3：整合服务器使用和部署文档

**文件：**
- 修改：`docs/exchange-data-runtime.md`
- 修改：`README.md`

- [x] **步骤 1：整理运行手册现有命令**

把脱离代码块的 macOS 配置和启动命令纳入对应章节，保持已有语义和用户内容。

- [x] **步骤 2：新增 Linux 服务器部署章节**

写明：

```text
Linux 构建产物必须与服务器 CPU/ABI 匹配
安装 Rust 1.98.0、wasm32 target、Dioxus CLI 0.7.10
运行 scripts/build-web.sh
使用 scripts/cryptool-web-server.sh 在局域网后台启动或管理进程
通过 CRYPTOOL_LISTEN/CONFIG/BIN/ALLOW_HOSTS 覆盖默认值
验证监听端口、防火墙和其他设备访问
使用 systemd 前台托管
使用 Nginx 提供 HTTPS、认证并传递匹配 Host
更新前备份，失败时回滚
```

- [x] **步骤 3：更新 README 入口**

增加最短可执行示例：

```bash
scripts/build-web.sh
./scripts/cryptool-web-server.sh
```

并链接到运行手册服务器章节。

- [x] **步骤 4：核对文档和脚本**

运行：

```bash
git diff --check
rg -n 'run-web-server|CRYPTOOL_LISTEN|CRYPTOOL_ALLOW_HOSTS|systemd|Nginx' \
  README.md docs/exchange-data-runtime.md
```

- [x] **步骤 5：提交文档**

```bash
git add README.md docs/exchange-data-runtime.md
git commit -m "docs: add LAN web deployment guide"
```

### 任务 4：最终验证

**文件：**
- 测试：`tests/cryptool-web-server-script.sh`
- 测试：`tests/server_contract.rs`

- [x] **步骤 1：运行脚本测试和格式检查**

```bash
bash tests/cryptool-web-server-script.sh
bash -n scripts/cryptool-web-server.sh tests/cryptool-web-server-script.sh tests/fixtures/capture-args.sh
git diff --check
```

- [x] **步骤 2：运行服务端回归测试**

```bash
cargo test -p cryptool --features server --locked
```

预期：CLI、查询、代理和 Axum 服务测试全部通过。

- [x] **步骤 3：核对工作区**

```bash
git status --short
git log --oneline -5
```

确认仅保留任务开始前已有的用户文档修改，新增脚本、测试、README 和运行手册均已进入明确提交。
