use super::context::AppContext;
use crate::domain::account::AccountType;
use crate::error::AppError;

/// 查询指定账户类型的余额。
///
/// 账户身份认证和 Binance 响应处理由 adapter 层负责。
pub async fn handle(ctx: &AppContext, account: AccountType) -> Result<(), AppError> {
    ctx.binance().account().balance(account).await
}
