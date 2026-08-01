use crate::error::AppError;
use crate::{binance::BinanceClient, config::Settings};

/// 应用生命周期内共享的依赖集合。
///
/// 当前持有一个已按最终配置创建的 Binance HTTP 客户端，避免每个命令
/// 重复构建连接池和超时策略。
pub struct AppContext {
    pub binance: BinanceClient,
}

impl AppContext {
    /// 依据最终配置构造共享依赖。
    pub fn new(settings: Settings) -> Result<Self, AppError> {
        let binance = BinanceClient::new(settings.binance)?;
        Ok(Self { binance })
    }

    pub fn binance(&self) -> &BinanceClient {
        &self.binance
    }
}
