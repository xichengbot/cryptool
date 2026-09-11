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
