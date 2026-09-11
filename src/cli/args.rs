use super::commands::Command;
use clap::Parser;
use cryptool::config::OutputFormat;
use std::path::PathBuf;

/// 顶层 CLI 参数。
///
/// 全局选项可出现在任意子命令之前；`command` 决定后续由哪个命令
/// 处理器执行。
#[derive(Debug, Parser)]
#[command(name = "cryptool")]
#[command(version = "0.1.0")]
#[command(about = "Binance CLI quant tool")]
pub struct Cli {
    /// TOML 配置文件路径；未指定时使用内置默认值。
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    /// 输出格式；指定时覆盖配置文件中的 `output.format`。
    #[arg(long, global = true)]
    pub output: Option<OutputFormat>,

    /// 使用永续合约模式的开关，供需要区分市场类型的命令使用。
    #[arg(short, long)]
    pub future: bool,

    /// 启用详细日志；具体日志粒度由后续命令处理器决定。
    #[arg(long, global = true)]
    pub verbose: bool,

    /// 覆写现货 Binance API 基地址，常用于测试环境或区域端点。
    #[arg(long, global = true)]
    pub base_url: Option<String>,

    /// 要执行的一级业务命令。
    #[command(subcommand)]
    pub command: Command,
}
