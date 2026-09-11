# 构建与部署脚本重构实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 将脚本按桌面应用和 Web 服务重组，桌面应用再按 macOS 和 Windows 划分，并提供构建、安装、Ubuntu 打包部署、兼容入口及 README 快速索引。

**架构：** 新实现位于 `scripts/app/<platform>/` 和 `scripts/web/`，Bash 公共能力位于 `scripts/lib/common.sh`。现有四个顶层脚本变为无业务逻辑的兼容转发器；构建产物统一进入 `dist/`，服务器运行脚本在仓库和同目录发布包中均可工作。

**技术栈：** Bash、PowerShell、Cargo、cargo-zigbuild、Dioxus CLI、NSIS、codesign、plutil、tar、SHA-256。

---

## 文件结构

- 创建 `scripts/lib/common.sh`：路径、日志、依赖检查、安全清理和 SHA-256 公共函数。
- 创建 `scripts/app/macos/{build,install}.sh`：macOS 构建、签名、验收和安装。
- 创建 `scripts/app/windows/{build,install}.ps1`：Windows NSIS 构建、校验和安装。
- 创建 `scripts/web/{build-local,build-ubuntu,package-ubuntu,deploy-ubuntu,server}.sh`：Web 构建、Ubuntu 发布和服务管理。
- 修改四个现有顶层脚本：保留为参数透传兼容入口。
- 创建 `tests/scripts/build-deploy-layout.sh`：布局、权限、帮助和兼容入口契约。
- 创建 `docs/build-deploy/{README,app-macos,app-windows,web-ubuntu}.md`：分目标文档。
- 修改 `README.md` 和 `docs/exchange-data-runtime.md`：快速索引与交叉链接。

### 任务 1：建立脚本布局契约测试

**文件：**
- 创建：`tests/scripts/build-deploy-layout.sh`

- [ ] **步骤 1：编写失败测试**

```bash
#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
required=(
  scripts/lib/common.sh
  scripts/app/macos/build.sh scripts/app/macos/install.sh
  scripts/app/windows/build.ps1 scripts/app/windows/install.ps1
  scripts/web/build-local.sh scripts/web/build-ubuntu.sh
  scripts/web/package-ubuntu.sh scripts/web/deploy-ubuntu.sh scripts/web/server.sh
)
for path in "${required[@]}"; do
  test -f "$root/$path" || { echo "missing: $path" >&2; exit 1; }
done
for path in scripts/app/macos/*.sh scripts/web/*.sh; do
  bash -n "$root/$path"
done
for path in scripts/app/macos/build.sh scripts/app/macos/install.sh scripts/web/*.sh; do
  "$root/$path" --help >/dev/null
done
```

- [ ] **步骤 2：运行并确认红灯**

运行：`bash tests/scripts/build-deploy-layout.sh`

预期：FAIL，报告 `missing: scripts/lib/common.sh`。

- [ ] **步骤 3：提交测试**

```bash
git add tests/scripts/build-deploy-layout.sh
git commit -m "test: define build and deployment script layout"
```

### 任务 2：公共函数和 macOS 应用脚本

**文件：**
- 创建：`scripts/lib/common.sh`
- 创建：`scripts/app/macos/build.sh`
- 创建：`scripts/app/macos/install.sh`
- 测试：`tests/scripts/build-deploy-layout.sh`

- [ ] **步骤 1：实现公共接口**

```bash
PROJECT_ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
fail() { printf 'error: %s\n' "$1" >&2; exit 1; }
step() { printf '==> %s\n' "$1"; }
require_command() { command -v "$1" >/dev/null 2>&1 || fail "required command not found: $1"; }
clean_generated_path() {
  case "$1" in
    "$PROJECT_ROOT/target/dx/"*|"$PROJECT_ROOT/web-dist/"*|"$PROJECT_ROOT/dist/"*) ;;
    *) fail "refusing to clean path outside generated directories: $1" ;;
  esac
  [[ ! -e "$1" ]] || find "$1" -depth -delete
}
sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1"; else shasum -a 256 "$1"; fi
}
```

- [ ] **步骤 2：实现 macOS build**

脚本支持 `--help`，验证 Darwin、`cargo`、`${DX:-dx}`、`codesign`、`plutil` 和 `file`；清理当前应用的 Dioxus release、bundle 和旧 `.app`；运行 Dioxus macOS release bundle；执行 ad-hoc 签名、plist、ICNS、Mach-O 和严格签名检查。

核心构建命令：

```bash
(cd "$PROJECT_ROOT/apps/exchange-ui" && "${DX:-dx}" bundle --macos --release \
  --no-default-features --features desktop --cargo-args=--locked \
  --package-types macos --out-dir "$PROJECT_ROOT/dist/macos")
codesign --force --deep --sign - "$PROJECT_ROOT/dist/macos/ExchangeData.app"
codesign --verify --deep --strict --verbose=2 "$PROJECT_ROOT/dist/macos/ExchangeData.app"
```

- [ ] **步骤 3：实现 macOS install**

支持 `--source`、`--target` 和 `--help`，默认安装 `dist/macos/ExchangeData.app` 到 `/Applications`。验证源 bundle 和签名；检测 `exchange-data` 正在运行时退出；用 `ditto` 安装并 `touch` 目标 bundle，不主动重启 Dock。

- [ ] **步骤 4：验证并提交**

运行 `bash -n`、帮助命令和 `scripts/app/macos/build.sh`，预期 `.app` 构建且签名通过。提交：

```bash
git add scripts/lib/common.sh scripts/app/macos tests/scripts/build-deploy-layout.sh
git commit -m "feat: organize macOS application scripts"
```

### 任务 3：Windows 应用脚本

**文件：**
- 创建：`scripts/app/windows/build.ps1`
- 创建：`scripts/app/windows/install.ps1`

- [ ] **步骤 1：实现 Windows build**

支持 `-Help`；验证 Windows、Cargo 和 Dioxus CLI；清理当前应用的 Windows staging、bundle 和旧输出；执行：

```powershell
& $DxTool bundle --windows --release --no-default-features --features desktop `
  --cargo-args=--locked --package-types nsis --out-dir (Join-Path $ProjectRoot 'dist/windows')
```

递归查找非空 `.exe`，输出路径、大小及 `Get-FileHash -Algorithm SHA256`。

- [ ] **步骤 2：实现 Windows install**

支持 `-Installer PATH`、`-Silent` 和 `-Help`。默认选择 `dist/windows` 最新 `.exe`；普通模式启动安装界面，静默模式传 `/S`；等待退出并拒绝非零退出码。

- [ ] **步骤 3：解析校验并提交**

如果存在 `pwsh`，使用 `[scriptblock]::Create((Get-Content -Raw ...))` 解析两个脚本；否则记录 Windows 实机验证要求。提交：

```bash
git add scripts/app/windows
git commit -m "feat: organize Windows application scripts"
```

### 任务 4：Web 本机构建、Ubuntu 构建和打包

**文件：**
- 创建：`scripts/web/build-local.sh`
- 创建：`scripts/web/build-ubuntu.sh`
- 创建：`scripts/web/package-ubuntu.sh`

- [ ] **步骤 1：实现 build-local**

支持 `--assets-only` 和 `--help`。清理 Web staging 与 `web-dist/public`，执行 Dioxus Web release bundle；检查 HTML、WASM、JS、CSS 和 favicon。普通模式再运行 Cargo `embed-web` release 构建，并复制到 `dist/web-local/cryptool`。

- [ ] **步骤 2：实现 build-ubuntu**

支持 `--target`、`--no-package` 和 `--help`，默认 `x86_64-unknown-linux-gnu.2.17`。调用 `build-local.sh --assets-only` 后执行：

```bash
cargo zigbuild -p cryptool --release --locked --features embed-web --target "$target"
```

验证 ELF x86-64、interpreter、可执行位和最高 GLIBC 版本不超过 2.17；默认调用打包脚本。

- [ ] **步骤 3：实现 package-ubuntu**

支持 `--binary` 和 `--help`。生成 `dist/linux-x86_64`，安装二进制、`server.sh`（发布名 `cryptool-web-server.sh`）、配置模板、运行文档和 README；生成 tar.gz 与 `.sha256`，运行 `gzip -t`、SHA-256、清单和 ELF 校验。

- [ ] **步骤 4：实际构建并提交**

运行：

```bash
scripts/web/build-local.sh
scripts/web/build-ubuntu.sh
```

预期生成本机 `dist/web-local/cryptool` 和 Ubuntu x86_64/glibc 2.17 发布包。提交：

```bash
git add scripts/web/build-local.sh scripts/web/build-ubuntu.sh scripts/web/package-ubuntu.sh
git commit -m "feat: organize Web build and Ubuntu packaging"
```

### 任务 5：Web 服务管理和 Ubuntu 部署

**文件：**
- 创建：`scripts/web/server.sh`
- 创建：`scripts/web/deploy-ubuntu.sh`
+- 测试：`tests/scripts/build-deploy-layout.sh`
+
+- [ ] **步骤 1：扩展失败测试**
+
+验证 `server.sh --help` 包含 `start|stop|restart|status|run`；验证部署帮助包含 `--source`、`--target`、`--user` 和 `--start`。
+
+- [ ] **步骤 2：迁移 server 核心**
+
+保留当前同目录二进制/配置发现、环境变量、Host 白名单、格式检查、后台 PID、日志和 TERM 停止逻辑。仓库模式沿用 `scripts/cryptool.pid` 与 `scripts/cryptool.log`；发布包模式使用脚本同目录。
+
+- [ ] **步骤 3：实现 deploy-ubuntu**
+
+支持 `--source`、`--target`、`--user`、`--start` 和 `--help`。要求 Linux x86_64 和 root；验证源 ELF；创建服务用户；正常停止旧服务；安装文件；仅在缺少 `config.toml` 时创建；设置 PID/日志权限；仅当指定 `--start` 时启动并检查状态。
+
+- [ ] **步骤 4：进程测试并提交**
+
+用回环地址、独立 PID/日志和临时端口执行 start、status、restart、stop，确认停止后 PID 文件为空。提交：
+
+```bash
+git add scripts/web/server.sh scripts/web/deploy-ubuntu.sh tests/scripts/build-deploy-layout.sh
+git commit -m "feat: organize Web runtime and Ubuntu deployment"
+```
+
+### 任务 6：兼容入口
+
+**文件：**
+- 修改：`scripts/build-desktop.sh`
+- 修改：`scripts/build-desktop.ps1`
+- 修改：`scripts/build-web.sh`
+- 修改：`scripts/cryptool-web-server.sh`
+
+- [ ] **步骤 1：扩展兼容测试**
+
+验证旧入口存在、Bash 语法通过、`--help` 与新入口一致，并确认旧文件不再包含构建或服务业务实现。
+
+- [ ] **步骤 2：实现转发器**
+
+Bash 入口使用以下形式并替换目标路径：
+
+```bash
+#!/usr/bin/env bash
+set -euo pipefail
+script_dir=$(cd -- "${BASH_SOURCE[0]%/*}" && pwd)
+exec "$script_dir/web/build-local.sh" "$@"
+```
+
+PowerShell 入口调用 `app/windows/build.ps1`，透传 `@args` 和退出码。
+
+- [ ] **步骤 3：验证并提交**
+
+运行完整布局测试后提交：
+
+```bash
+git add scripts/build-desktop.sh scripts/build-desktop.ps1 scripts/build-web.sh scripts/cryptool-web-server.sh tests/scripts/build-deploy-layout.sh
+git commit -m "refactor: keep compatible build script entrypoints"
+```
+
+### 任务 7：平台文档和 README 索引
+
+**文件：**
+- 创建：`docs/build-deploy/README.md`
+- 创建：`docs/build-deploy/app-macos.md`
+- 创建：`docs/build-deploy/app-windows.md`
+- 创建：`docs/build-deploy/web-ubuntu.md`
+- 修改：`README.md`
+- 修改：`docs/exchange-data-runtime.md`
+
+- [ ] **步骤 1：编写分目标文档**
+
+总览用矩阵列出目标、构建入口、安装/部署入口和产物。三个平台文档分别包含依赖、首次构建、重新构建、安装/部署、配置、验证、升级和常见错误。
+
+- [ ] **步骤 2：增加根 README 快速索引**
+
+```markdown
+## 构建与部署索引
+
+| 目标 | 构建 | 安装/部署 | 文档 |
+| --- | --- | --- | --- |
+| macOS 应用 | `scripts/app/macos/build.sh` | `scripts/app/macos/install.sh` | `docs/build-deploy/app-macos.md` |
+| Windows 应用 | `scripts/app/windows/build.ps1` | `scripts/app/windows/install.ps1` | `docs/build-deploy/app-windows.md` |
+| Web / Ubuntu | `scripts/web/build-ubuntu.sh` | `scripts/web/deploy-ubuntu.sh` | `docs/build-deploy/web-ubuntu.md` |
+```
+
+- [ ] **步骤 3：收敛运行文档并检查链接**
+
+保留 `docs/exchange-data-runtime.md` 的配置、代理、API 和运行语义，在构建部署章节链接新文档，并将旧命令标为兼容入口。扫描 Markdown 相对链接确认文件存在。
+
+- [ ] **步骤 4：提交文档**
+
+```bash
+git add README.md docs/build-deploy docs/exchange-data-runtime.md
+git commit -m "docs: index application and Web deployment workflows"
+```
+
+### 任务 8：最终验收
+
+- [ ] **步骤 1：静态检查**
+
+```bash
+bash tests/scripts/build-deploy-layout.sh
+find scripts -name '*.sh' -type f -exec bash -n {} \;
+cargo fmt --all --check
+git diff --check
+```
+
+- [ ] **步骤 2：连续构建两次 macOS 应用**
+
+运行两次 `scripts/app/macos/build.sh`，确认无旧哈希资源、ICNS 存在、plist 正常且严格签名通过。
+
+- [ ] **步骤 3：构建 Web 和 Ubuntu 包**
+
+运行 `scripts/web/build-local.sh` 与 `scripts/web/build-ubuntu.sh`，确认本机二进制格式正确、Ubuntu ELF 最高 GLIBC 为 2.17、tar.gz 清单和 SHA-256 通过。
+
+- [ ] **步骤 4：验证 Git 边界**
+
+`git status --short` 不得新增 `dist/`、`target/`、PID 或日志；用户原有四个未提交分析文档保持未暂存且内容不被本任务覆盖。
+
+- [ ] **步骤 5：处理验收修正**
+
+只有验收发现并修正文档或脚本时才创建最终提交；否则记录已有提交和验证结果，不创建空提交。
+EOF
rg -n 'TODO|TBD|待定|PLACEHOLDER|类似任务|添加适当|添加验证|处理边界' docs/superpowers/plans/2026-09-11-build-deploy-scripts.md || true
git diff --check -- docs/superpowers/plans/2026-09-11-build-deploy-scripts.md
git status --short