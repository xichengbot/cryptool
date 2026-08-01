//! 统一终端输出协议与渲染器。
//!
//! 命令处理器返回 `Output`，入口根据配置选择 JSON 或表格渲染，从而不让
//! 业务层依赖终端格式。

pub mod format;
pub mod json;
pub mod table;

use crate::error::AppError;
use crate::output::format::OutputFormat;
use serde::Serialize;
use serde_json::Value;

/// 命令处理器交给终端渲染器的中间输出。
#[derive(Debug)]
pub enum Output {
    /// 以纯文本呈现的消息。
    Text(String),
    /// 可由 JSON 或表格渲染器消费的结构化数据。
    Json(Value),
    /// 成功但没有内容的命令结果。
    Empty,
}

impl Output {
    /// 将可序列化领域对象转换为统一 JSON 输出；序列化错误保持为应用错误。
    pub fn json<T: Serialize>(value: T) -> Result<Self, AppError> {
        serde_json::to_value(value)
            .map(Self::Json)
            .map_err(|err| AppError::Serialization(err.to_string()))
    }
}

/// 使用配置的渲染格式输出命令结果。
pub fn render(output: Output, format: OutputFormat) -> Result<(), AppError> {
    match format {
        OutputFormat::Table => table::render(output),
        OutputFormat::Json => json::render(output),
    }
}
