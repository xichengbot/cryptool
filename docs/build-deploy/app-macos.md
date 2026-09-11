# macOS 应用构建与安装

要求 macOS、项目固定 Rust 工具链、`wasm32-unknown-unknown` target 和 Dioxus CLI 0.7.10。

```sh
rustup target add wasm32-unknown-unknown
cargo install dioxus-cli --version 0.7.10 --locked
./scripts/app/macos/build.sh
```

产物为 `dist/macos/ExchangeData.app`。脚本会清理该应用的旧 staging，生成 `.app`，执行 ad-hoc
签名，并检查 Mach-O、Info.plist、ICNS 和签名。重复执行即可重新构建。

安装到 `/Applications`：

```sh
./scripts/app/macos/install.sh
open -a ExchangeData
```

安装到任意目录：

```sh
./scripts/app/macos/install.sh --target "$HOME/Applications"
open "$HOME/Applications/ExchangeData.app"
```

也可直接双击 `.app`、使用 Finder、Launchpad、Spotlight，或执行
`open dist/macos/ExchangeData.app`。默认配置位于
`~/Library/Application Support/com.cryptool.exchange-data/config.toml`；应用目录不影响配置位置。

未做 Apple Developer ID 签名和公证，向其他机器分发时需按组织证书流程补签和 notarize。
