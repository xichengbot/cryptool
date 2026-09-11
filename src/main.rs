#![allow(dead_code)]

//! `cryptool` 的二进制入口：解析 CLI、合并配置、构造共享上下文、
//! 分发子命令，并将统一输出渲染到标准输出。

use clap::Parser;

mod cli;
mod output;

use cli::{Cli, Command};
use cryptool::app::context::AppContext;
use cryptool::config::Settings;
pub(crate) use cryptool::{app, domain, error};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    tracing::debug!(future = cli.future, "parsed CLI market flag");
    let mut settings = Settings::load(cli.config.as_deref(), cli.base_url.clone())?;
    // CLI 覆写配置文件和默认配置中的输出格式。
    if let Some(output) = cli.output {
        settings.output.format = output;
    }

    let output_format = settings.output.format;

    #[cfg(feature = "server")]
    let command = match cli.command {
        Command::Serve(command) => {
            let service = cryptool::app::exchange_data::ExchangeDataService::new(settings)?;
            command.execute(service).await?;
            return Ok(());
        }
        command => command,
    };
    #[cfg(not(feature = "server"))]
    let command = cli.command;

    let ctx = AppContext::new(settings)?;

    // 各命令都返回 Output，使渲染策略与业务处理解耦。
    let output = match command {
        Command::Market(command) => command.execute(&ctx).await?,
        Command::FuturesMarket(command) => command.execute(&ctx).await?,
        Command::Account(command) => command.execute(&ctx).await?,
        #[cfg(feature = "server")]
        Command::Serve(_) => unreachable!("serve command returned before AppContext construction"),
    };

    output::render(output, output_format)?;

    Ok(())

    // // 1. 手动构建命令和参数
    // let matches = Command::new("myapp")
    //     .version("1.0.0")
    //     .about("这是一个很棒的 CLI 工具")
    //     .arg(
    //         Arg::new("config")
    //             .short('c')
    //             .long("config")
    //             .help("设置一个配置文件路径")
    //             // 告诉 clap 这个参数需要接收一个值
    //             .action(ArgAction::Set)
    //     )
    //     .arg(
    //         Arg::new("verbose")
    //             .short('v')
    //             .long("verbose")
    //             .help("开启啰嗦模式")
    //             // 告诉 clap 这是一个开关(Flag)，出现就是 true
    //             .action(ArgAction::SetTrue)
    //     )
    //     .arg(
    //         Arg::new("file_name")
    //             .help("要处理的目标文件名")
    //             // 位置参数，且标记为必填
    //             .required(true)
    //             .index(1)
    //     )
    //     .get_matches(); // 开始解析命令行输入！

    // // 2. 手动从 matches 中提取（解包）数据

    // // 提取必填的位置参数 (因为设置了 required(true)，所以 unwrap 是安全的)
    // let file_name = matches.get_one::<String>("file_name").unwrap();
    // println!("目标文件: {}", file_name);

    // // 提取可选的 config 参数
    // if let Some(config_path) = matches.get_one::<String>("config") {
    //     println!("使用的配置文件: {}", config_path);
    // }

    // // 提取 boolean 类型的开关
    // let verbose = matches.get_flag("verbose");
    // if verbose {
    //     println!("啰嗦模式已开启！");
    // }
}
