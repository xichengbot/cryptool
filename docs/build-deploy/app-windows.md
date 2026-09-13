# Windows 应用构建与安装

在 Windows PowerShell 中安装项目固定 Rust 工具链、Dioxus CLI 0.7.10、Visual Studio C++
Build Tools、WebView2 Runtime 和 NSIS，然后在仓库根目录执行：

```powershell
rustup target add wasm32-unknown-unknown
cargo install dioxus-cli --version 0.7.10 --locked
.\scripts\app\windows\build.ps1
```

脚本清理本应用的旧 Windows staging，生成 NSIS `.exe`，并输出文件大小和 SHA-256。产物位于
`dist\windows`。在同一源码版本上重新执行该命令即可重新构建。

Dioxus CLI 0.7.10 生成的 NSIS 脚本为无 BOM 的 UTF-8，在中文 Windows 上可能因默认按 ACP
读取而报 `Bad text encoding`。构建脚本会识别该错误，自动使用 `/INPUTCHARSET UTF8` 重新
编译生成的 `.nsi`，保留中文应用描述，并将安装包复制到 `dist\windows`。首次打包的错误日志
可能仍会显示，以脚本最后输出的安装包路径、大小和 SHA-256 为准；其他构建错误仍会终止构建。
Dioxus 构建日志保存在 `target\windows-bundle.log`（设置 `CARGO_TARGET_DIR` 时位于该目录）。

安装最新产物：

```powershell
.\scripts\app\windows\install.ps1
```

指定安装包或静默安装：

```powershell
.\scripts\app\windows\install.ps1 -Installer .\dist\windows\ExchangeData.exe
.\scripts\app\windows\install.ps1 -Silent
```

默认配置位于 `%APPDATA%\cryptool\exchange-data\config\config.toml`。实际安装包文件名由
Dioxus/NSIS 生成，以构建脚本输出为准。Windows 产物必须在 Windows 实机完成构建与启动验收。
