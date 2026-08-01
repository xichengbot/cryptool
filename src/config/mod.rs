//! 配置加载与配置模型。
//!
//! `Settings` 负责将内置默认值、可选 TOML 文件、环境变量和 CLI 覆写
//! 合并为创建 Binance 客户端所需的最终配置。

pub mod settings;

pub use settings::{BinanceSettings, Settings};
