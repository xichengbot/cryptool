use crate::binance::BinanceClient;
use crate::domain::account::AccountType;
use crate::error::AppError;

/// Binance 账户 REST adapter。
///
/// 该 adapter 将在账户私有端点实现后统一承载认证与账户类型分发。
pub struct BinanceAccountApi<'a> {
    client: &'a BinanceClient,
}

impl<'a> BinanceAccountApi<'a> {
    pub fn new(client: &'a BinanceClient) -> Self {
        Self { client }
    }

    /// 查询账户余额。
    ///
    /// 当前私有端点尚未实现；保留显式错误以避免命令产生看似成功的空结果。
    pub async fn balance(&self, _account: AccountType) -> Result<(), AppError> {
        let _ = self.client;
        Err(AppError::NotImplemented("account balance".to_string()))
    }
}
