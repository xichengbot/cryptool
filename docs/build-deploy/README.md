# 构建与部署索引

脚本按交付目标划分。桌面应用只支持 macOS 和 Windows；Linux 只部署 Web 服务。

| 目标 | 构建 | 安装/部署 | 文档 |
|---|---|---|---|
| macOS 应用 | `scripts/app/macos/build.sh` | `scripts/app/macos/install.sh` | [macOS 应用](app-macos.md) |
| Windows 应用 | `scripts/app/windows/build.ps1` | `scripts/app/windows/install.ps1` | [Windows 应用](app-windows.md) |
| 本机 Web | `scripts/web/build-local.sh` | `scripts/web/server.sh` | [Ubuntu Web](web-ubuntu.md) |
| Ubuntu Web | `scripts/web/build-ubuntu.sh` | `scripts/web/deploy-ubuntu.sh` | [Ubuntu Web](web-ubuntu.md) |

旧入口 `scripts/build-desktop.sh`、`scripts/build-desktop.ps1`、`scripts/build-web.sh` 和
`scripts/cryptool-web-server.sh` 保留，内部转发到上表的新入口。

发布产物统一写入 `dist/`：

- `dist/macos/ExchangeData.app`
- `dist/windows/**/*.exe`
- `dist/web-local/cryptool`
- `dist/linux-x86_64/`
- `dist/cryptool-web-linux-x86_64.tar.gz` 及 `.sha256`
