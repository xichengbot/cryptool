use crate::config::{OutputFormat, ProxySettings, QuerySettings};
use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::{env, fs};

/// 应用运行时的完整配置。
///
/// 加载顺序为：内置默认值或指定 TOML 文件，然后以环境变量注入凭据，
/// 最后应用 CLI 基地址覆写。
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub binance: BinanceSettings,
    pub output: OutputSettings,
    pub exchange_data: QuerySettings,
}

/// Binance 客户端连接与认证配置。
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BinanceSettings {
    /// 私有端点使用的 API Key；可由 `BINANCE_API_KEY` 覆写。
    pub api_key: Option<String>,
    /// 私有请求签名使用的私钥；可由 `BINANCE_PRIVATE_KEY` 覆写。
    pub private_key: Option<String>,
    /// 现货 REST API 基地址。
    pub base_url: String,
    /// USDⓈ-M 合约 REST API 基地址。
    pub usdm_futures_base_url: String,
    /// Binance 私有请求允许的最大时间窗口，单位为毫秒。
    pub recv_window: u64,
    /// 单次 HTTP 请求的总超时，单位为毫秒。
    pub timeout_ms: u64,
    pub proxy: ProxySettings,
}

impl Settings {
    /// 加载并按既定优先级合并配置。
    ///
    /// `BINANCE_API_KEY` 与 `BINANCE_PRIVATE_KEY` 仅覆盖对应的认证字段；
    /// `base_url_override` 来自 CLI，优先级高于配置文件。
    pub fn load(
        config_path: Option<&Path>,
        base_url_override: Option<String>,
    ) -> Result<Self, AppError> {
        let mut settings = if let Some(path) = config_path {
            // 读取配置文件
            let content = fs::read_to_string(path)
                .map_err(|err| AppError::Config(format!("Unable to read config file: {}", err)))?;
            // 解析配置文件
            toml::from_str::<Self>(&content).map_err(|_| {
                AppError::Config(format!("invalid TOML configuration: {}", path.display()))
            })?
        } else {
            Self::default()
        };

        if let Ok(api_key) = env::var("BINANCE_API_KEY") {
            settings.binance.api_key = Some(api_key);
        }

        if let Ok(private_key) = env::var("BINANCE_PRIVATE_KEY") {
            settings.binance.private_key = Some(private_key);
        }

        if let Some(base_url) = base_url_override {
            settings.binance.base_url = base_url;
        }

        Ok(settings)
    }
}

impl std::fmt::Debug for BinanceSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BinanceSettings")
            .field("api_key_configured", &self.api_key.is_some())
            .field("private_key_configured", &self.private_key.is_some())
            .field("timeout_ms", &self.timeout_ms)
            .field("proxy", &self.proxy)
            .finish_non_exhaustive()
    }
}

impl Default for BinanceSettings {
    fn default() -> Self {
        Self {
            api_key: None,
            private_key: None,
            base_url: "https://api.binance.com".into(),
            usdm_futures_base_url: "https://fapi.binance.com".into(),
            recv_window: 50_000,
            timeout_ms: 100_000,
            proxy: ProxySettings::default(),
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            binance: BinanceSettings::default(),
            output: OutputSettings {
                format: OutputFormat::Table,
            },
            exchange_data: QuerySettings::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutputSettings {
    pub format: OutputFormat,
}
