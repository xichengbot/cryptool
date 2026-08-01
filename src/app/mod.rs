//! 应用服务层。
//!
//! 服务函数编排 CLI 命令与 Binance adapter，但不解析 HTTP 响应或承担
//! 终端输出格式化职责。

pub mod account;
pub mod context;
pub mod futures_market;
pub mod market;
