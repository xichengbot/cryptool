# 构建与部署脚本重构设计

## 目标

将构建和部署入口按产品划分为桌面应用与 Web 服务。桌面应用继续只支持 macOS 和 Windows；Web 服务提供本机构建、Ubuntu x86_64 交叉构建、发布包制作、服务器部署和进程管理。

脚本需要满足以下约束：

- 保留现有 CLI、Web 和桌面功能。
- 保留现有顶层脚本作为兼容入口，避免已有文档和自动化立即失效。
- 构建输出统一写入 `dist/`，不把临时文件写入源码目录。
- 配置文件更新时不覆盖部署环境中的现有 `config.toml`。
- macOS、Windows 和 Ubuntu 的产物分别构建和验证，禁止跨平台混用。
- 所有脚本使用非零退出码报告失败，并在执行外部命令前检查依赖。

## 目录结构

```text
scripts/
├── app/
│   ├── macos/
│   │   ├── build.sh
│   │   └── install.sh
│   └── windows/
│       ├── build.ps1
│       └── install.ps1
├── web/
│   ├── build-local.sh
│   ├── build-ubuntu.sh
│   ├── package-ubuntu.sh
│   ├── deploy-ubuntu.sh
│   └── server.sh
├── lib/
│   └── common.sh
├── build-desktop.sh
├── build-desktop.ps1
├── build-web.sh
└── cryptool-web-server.sh
```

顶层四个旧脚本只负责参数透传和调用新入口，不再保存重复实现。

## 公共脚本约定

`scripts/lib/common.sh` 提供 Bash 脚本共用的只读能力：

- 解析仓库根目录。
- 检查必需命令。
- 输出统一的步骤、成功和错误信息。
- 安全清理限定在 `target/dx/`、`web-dist/` 和 `dist/` 下的已知生成目录。
- 计算 SHA-256，并兼容 macOS 的 `shasum` 与 Linux 的 `sha256sum`。

PowerShell 脚本保持自包含，使用相同的参数名、输出目录和失败语义，避免为了少量逻辑引入跨 Shell 依赖。

## 桌面应用

### macOS 构建

入口：

```sh
scripts/app/macos/build.sh
```

流程：

1. 要求运行环境为 macOS，并检查 `cargo`、`dx`、`codesign` 和 `plutil`。
2. 清理当前应用的 Dioxus macOS staging、bundle 和旧 `dist/macos/ExchangeData.app`。
3. 以 `desktop` feature 执行 release bundle。
4. 对最终 `.app` 执行 ad-hoc 签名。
5. 校验 Mach-O 架构、Info.plist、ICNS 引用和严格签名。
6. 输出绝对产物路径和大小。

环境变量 `DX` 可覆盖 Dioxus CLI。输出固定为：

```text
dist/macos/ExchangeData.app
```

### macOS 安装

入口：

```sh
scripts/app/macos/install.sh [--source APP] [--target DIR]
```

默认从 `dist/macos/ExchangeData.app` 安装到 `/Applications`。安装前验证 bundle，使用 `ditto` 复制并更新时间戳。脚本不主动结束用户进程或重启 Dock；如果应用仍在运行则明确报错，避免覆盖正在使用的 bundle。

### Windows 构建

入口：

```powershell
./scripts/app/windows/build.ps1
```

流程：

1. 要求 Windows PowerShell 7 或 Windows PowerShell，并检查 `cargo` 与 `dx`。
2. 清理当前应用的 Dioxus Windows staging、bundle 和旧输出。
3. 以 `desktop` feature 生成 release NSIS 安装程序。
4. 检查输出至少包含一个非空 `.exe`，并输出文件路径、大小和 SHA-256。

输出目录固定为：

```text
dist/windows/
```

### Windows 安装

入口：

```powershell
./scripts/app/windows/install.ps1 [-Installer PATH] [-Silent]
```

默认选择 `dist/windows/` 中最新的 NSIS `.exe`。普通模式启动安装界面并等待退出；`-Silent` 传递 NSIS 静默安装参数。安装程序非零退出时脚本失败。

## Web 服务

### 本机构建

入口：

```sh
scripts/web/build-local.sh
```

流程：

1. 清理 Web staging 和 `web-dist/public`。
2. 使用 Dioxus 构建 release Web/WASM 资源。
3. 使用 `embed-web` feature 构建当前系统、当前架构的 `cryptool`。
4. 验证 `index.html`、WASM、JavaScript、CSS 和 favicon 均存在。
5. 将当前平台二进制复制到 `dist/web-local/cryptool`，并打印 `file` 结果。

该产物仅用于本机运行，不命名为通用 Web 二进制。

### Ubuntu 交叉构建

入口：

```sh
scripts/web/build-ubuntu.sh [--target x86_64-unknown-linux-gnu.2.17]
```

默认目标为 `x86_64-unknown-linux-gnu.2.17`。脚本检查 `cargo zigbuild` 和 Zig，先确保 Web 资源已生成，再使用 `embed-web` feature 构建 Linux ELF。验证内容包括：

- 文件格式为 ELF x86-64。
- ELF interpreter 为 `/lib64/ld-linux-x86-64.so.2`。
- 引用的最高 GLIBC 符号版本不超过 2.17。
- 二进制非空且可执行。

### Ubuntu 发布包

入口：

```sh
scripts/web/package-ubuntu.sh
```

脚本默认消费交叉构建产物，生成：

```text
dist/linux-x86_64/
├── cryptool
├── cryptool-web-server.sh
├── config.example.toml
├── exchange-data-runtime.md
└── README.txt

dist/cryptool-ubuntu-x86_64-glibc217.tar.gz
dist/cryptool-ubuntu-x86_64-glibc217.tar.gz.sha256
```

发布包使用固定目录层级，关闭 macOS 扩展属性归档，并在完成后执行 gzip、SHA-256、文件清单和 ELF 校验。

`build-ubuntu.sh` 默认在成功编译后调用 `package-ubuntu.sh`；通过 `--no-package` 可以只生成 ELF。

### Ubuntu 部署

入口：

```sh
scripts/web/deploy-ubuntu.sh [--source DIR] [--target DIR] [--user USER]
```

默认从脚本所在发布目录或 `dist/linux-x86_64` 读取文件，部署到 `/data/cryptool`，服务用户默认为 `cryptool`。流程为：

1. 验证当前系统为 Linux、CPU 为 x86_64、源二进制为 ELF。
2. 如果服务正在运行，先通过服务管理脚本正常停止。
3. 安装二进制、服务管理脚本、配置模板和文档。
4. 仅在目标不存在 `config.toml` 时从模板创建。
5. 设置目录、二进制、配置、PID 和日志权限。
6. 默认完成文件部署但不自动启动；`--start` 明确要求启动并检查状态。

脚本不修改防火墙、Nginx 或 systemd，避免隐式改变服务器安全边界。相关配置继续由部署文档提供。

### Web 进程管理

入口：

```sh
scripts/web/server.sh [start|stop|restart|status|run]
```

保留当前行为：

- 默认后台启动。
- 二进制、`config.toml`、PID 和日志优先从脚本同目录读取。
- 支持 `CRYPTOOL_BIN`、`CRYPTOOL_CONFIG`、`CRYPTOOL_LISTEN`、`CRYPTOOL_ALLOW_HOSTS`、`CRYPTOOL_PID_FILE` 和 `CRYPTOOL_LOG_FILE`。
- 非回环监听时发现局域网地址，并拒绝在没有可用 Host 白名单时启动。
- 启动前检查操作系统与二进制格式是否匹配。
- 停止时发送 TERM 并等待，超时后报告失败，不强制 KILL。

## 兼容入口

旧命令继续可用：

```text
scripts/build-desktop.sh       -> scripts/app/macos/build.sh
scripts/build-desktop.ps1      -> scripts/app/windows/build.ps1
scripts/build-web.sh           -> scripts/web/build-local.sh
scripts/cryptool-web-server.sh -> scripts/web/server.sh
```

兼容脚本不输出弃用警告，避免污染自动化日志。README 和新文档统一使用新路径，旧路径只用于兼容。

## 文档结构

```text
docs/build-deploy/
├── README.md          # 产品矩阵、依赖和命令总览
├── app-macos.md       # 构建、安装、更新、签名和故障处理
├── app-windows.md     # 构建、安装、更新和 SmartScreen 说明
└── web-ubuntu.md      # 构建、打包、部署、服务管理和代理配置
```

现有 `docs/exchange-data-runtime.md` 保留接口、配置和运行语义，并链接到新的构建部署文档，避免继续扩张为混合手册。

## 测试与验收

### 静态检查

- Bash：`bash -n`，并在可用时执行 `shellcheck`。
- PowerShell：解析脚本语法；Windows 构建机执行实际构建。
- 所有兼容入口验证参数和退出码透传。
- Git 差异检查不得包含生成的 PID、日志或 `dist/` 文件。

### macOS

- 连续执行两次构建，确认无旧哈希资源残留。
- `plutil -lint` 通过。
- `CFBundleIconFile` 指向实际 ICNS。
- `codesign --verify --deep --strict` 通过。
- 产物为当前 macOS 主机架构的 Mach-O。

### Windows

- 在 Windows 主机执行构建。
- NSIS `.exe` 存在且非空。
- SHA-256 生成成功。
- 普通安装与静默安装返回成功退出码。

### Web/Ubuntu

- Web release bundle 和内嵌原生构建通过。
- Ubuntu 产物为 x86-64 ELF，最高 GLIBC 版本不超过 2.17。
- 压缩包结构和 SHA-256 校验通过。
- 在 Ubuntu 上验证 `start`、`status`、`restart`、`stop` 和 `run`。
- 用局域网地址访问首页，并执行一次现货查询。

## 风险与取舍

- macOS 使用 ad-hoc 签名，适合内部部署；面向外部发布仍需要 Developer ID 和公证。
- Windows 构建必须在 Windows 上完成，当前 macOS 环境只能验证 PowerShell 结构，不能验收最终安装程序。
- Ubuntu 固定到 x86_64/glibc 2.17，暂不增加 aarch64，以保持发布矩阵简单。
- 兼容入口会增加少量文件，但避免一次性破坏现有调用方；核心逻辑只保留在新目录中。
- 部署脚本默认不启动、不修改网络安全配置，使文件更新与服务启用保持显式边界。
