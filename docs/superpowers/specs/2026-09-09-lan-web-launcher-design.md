# Web 局域网启动脚本设计

日期：2026-09-09

## 目标

新增 `scripts/cryptool-web-server.sh`，用于在 macOS 或 Linux 服务器上管理已经构建的 Web/Axum 二进制，并允许同一局域网内的其他设备访问。同步扩充 `docs/exchange-data-runtime.md`，形成从 Linux 构建、后台进程管理、systemd 托管、Nginx 反向代理到更新回滚的完整操作说明。

Linux 只承载 Web/Axum 服务；项目仍不支持 Linux Desktop。

## 启动接口

脚本默认行为：

- 二进制：优先读取脚本同目录的 `cryptool`；不存在时回退到仓库内 `target/release/cryptool`。
- 配置：优先读取脚本同目录的 `config.toml`，其次读取仓库根配置，两处都不存在时使用程序内置默认配置。
- 监听：`0.0.0.0:8080`。
- Host：优先使用显式配置；未配置时自动发现本机非回环局域网 IPv4。
- 进程：默认 `start` 后台运行；提供 `stop`、`restart`、`status`，`run` 通过 `exec` 前台运行。
- 状态：PID 和日志默认保存在脚本同目录，可通过环境变量覆盖。

支持以下环境变量：

| 变量 | 含义 | 默认值 |
|---|---|---|
| `CRYPTOOL_BIN` | Web/Axum 可执行文件 | 同目录 `cryptool`，不存在时回退仓库发布产物 |
| `CRYPTOOL_CONFIG` | TOML 配置文件；空值表示不传 `--config` | 同目录 `config.toml`，其次仓库根配置 |
| `CRYPTOOL_LISTEN` | 完整监听地址 | `0.0.0.0:8080` |
| `CRYPTOOL_ALLOW_HOSTS` | 逗号或空格分隔的域名/IP | 自动发现局域网 IPv4 |
| `CRYPTOOL_PID_FILE` | 后台进程 PID 文件 | 同目录 `cryptool.pid` |
| `CRYPTOOL_LOG_FILE` | 后台进程日志 | 同目录 `cryptool.log` |

显式 Host 示例：

```sh
CRYPTOOL_ALLOW_HOSTS="192.168.1.20,exchange.local" \
CRYPTOOL_LISTEN="0.0.0.0:9090" \
./scripts/cryptool-web-server.sh
```

## 局域网地址发现

脚本按顺序尝试：

1. Linux `hostname -I`。
2. Linux `ip -o -4 addr show scope global`。
3. macOS `ipconfig getifaddr en0` 和 `ipconfig getifaddr en1`。

仅接受格式有效且不是 `127.*` 的 IPv4，并去重。监听非回环地址且没有任何有效 Host 时，脚本返回非零退出码并提示设置 `CRYPTOOL_ALLOW_HOSTS`，不会使用通配 Host。

脚本启动前检查二进制是否存在且可执行；显式配置路径必须是普通可读文件。错误信息不打印配置内容、Key 或代理认证信息。

## 文档结构

运行手册增加“Linux 服务器部署与局域网访问”章节，包含：

1. Linux 必须使用 Linux 构建产物，且 CPU 架构需要匹配。
2. Rust 1.98.0、WASM target 和 Dioxus CLI 0.7.10 的安装方式。
3. `scripts/build-web.sh` 构建及内嵌资源说明。
4. 启动脚本的默认用法、环境变量、局域网访问地址及故障排查。
5. 防火墙仅在确有需要时开放应用端口；反向代理模式保持 Axum 监听回环地址。
6. systemd 使用脚本的 `run` 前台模式，配置和凭据置于程序目录之外。
7. Nginx 传递真实 Host，并与 `CRYPTOOL_ALLOW_HOSTS` 对齐。
8. 替换二进制前备份、健康检查失败时回滚。

现有文档中的用户未提交内容原样保留；整理明显脱离 Markdown 代码块的命令时，只调整排版，不改变命令语义。

## 测试

先新增脚本契约测试并验证红灯，再实现脚本。测试使用临时假二进制记录收到的参数，不启动真实网络服务，覆盖：

- 显式监听、配置和多个 Host 被正确转换为 `cryptool ... serve` 参数。
- 空显式配置不产生 `--config`。
- 缺少可执行文件时失败。
- 显式配置路径不存在时失败。
- 自动发现不可用且监听非回环地址时失败，并给出可操作提示。
- 后台启动生成 PID，重复启动被拒绝，状态可查询，停止后清理 PID。

完成后运行脚本契约测试、`bash -n`、`git diff --check`，再运行受影响的 Rust 服务测试。文档示例与脚本的变量名和默认值逐项核对。

## 安全边界

局域网访问不等于公网发布。脚本不修改防火墙、不创建系统用户、不安装 systemd unit，也不请求 root 权限。公网部署仍需由操作者单独配置 HTTPS、认证和访问控制。应用没有多租户权限系统，不应把裸 Axum 端口直接暴露到公网。
