use serde::Deserialize;

/// Binance 非成功响应的标准 JSON 结构。
///
/// 客户端先尝试将错误响应解码为该类型，再退化为保留原始 HTTP 正文。
#[derive(Debug, Deserialize)]
pub struct BinanceErrorResponse {
    pub code: i64,
    pub msg: String,
}
