//! 命令行解析层。
//!
//! 本模块仅将用户输入映射为命令对象并协调应用服务；HTTP 调用和领域
//! 数据转换分别由 `app` 与 `binance` 模块负责。

pub mod account;
pub mod args;
pub mod commands;
mod futures_market;
pub mod market;
#[cfg(feature = "server")]
pub mod serve;
mod trades;

// 重新导出
pub use args::Cli;
pub use commands::Command;
