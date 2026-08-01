use crate::{
    app::{self, context::AppContext},
    domain::account::AccountType,
    error::AppError,
    output::Output,
};
use clap::{Args, Subcommand};

/// 账户命令的参数集合。
#[derive(Debug, Args)]
pub struct AccountCommand {
    /// 要执行的账户查询。
    #[command(subcommand)]
    pub action: AccountAction,
}

/// 账户支持的查询操作。
#[derive(Debug, Subcommand)]
pub enum AccountAction {
    /// 查询指定账户类型的余额。
    Balance {
        /// 账户类型；默认查询现货账户。
        #[arg(long, value_enum, default_value = "spot")]
        account: AccountType,
    },
}

impl AccountCommand {
    /// 将账户 CLI 参数编排为应用服务调用。
    pub async fn execute(self, ctx: &AppContext) -> Result<Output, AppError> {
        match self.action {
            AccountAction::Balance { account } => {
                app::account::handle(ctx, account).await?;
                Ok(Output::Empty)
            }
        }
    }
}
