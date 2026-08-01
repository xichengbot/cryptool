use thiserror::Error;

/// 跨 CLI、应用服务、Binance adapter 与输出层传播的统一错误类型。
///
/// adapter 应保留 Binance 的业务错误码，而网络、配置、输入和 JSON 错误
/// 则保留其所属边界的诊断信息。
#[derive(Debug, Error)]
pub enum AppError {
    #[error("config error: {0}")]
    Config(String),

    #[error("http error: {0}")]
    Http(String),

    #[error("binance error: code={code}, message={message}")]
    Binance { code: i64, message: String },

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("not implemented: {0}")]
    NotImplemented(String),

    #[error("serialization error: {0}")]
    Serialization(String),
}
