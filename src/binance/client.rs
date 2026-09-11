use super::response::BinanceErrorResponse;
use crate::binance::account::BinanceAccountApi;
use crate::binance::futures_market::BinanceFuturesMarketApi;
use crate::binance::market::BinanceMarketApi;
use crate::binance::signer::sign_query;
use crate::{config::BinanceSettings, error::AppError};
use reqwest::Method;
use serde::{Serialize, de::DeserializeOwned};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const API_KEY_HEADER: &str = "X-MBX-APIKEY";

/// Binance HTTP 客户端与市场 API adapter 的创建入口。
///
/// 客户端在 `AppContext` 中复用，因此连接池、代理环境和超时策略会在
/// 同一进程的所有命令之间保持一致。
#[derive(Clone)]
pub struct BinanceClient {
    pub(super) settings: BinanceSettings,
    pub(super) http: reqwest::Client,
}

impl BinanceClient {
    /// 使用配置中的总请求超时构造 `reqwest` 客户端。
    ///
    /// 未显式配置代理时，`reqwest` 会遵循进程的 HTTP(S) 代理环境变量。
    pub fn new(settings: BinanceSettings) -> Result<Self, AppError> {
        let builder = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_millis(settings.timeout_ms));
        let http = settings
            .proxy
            .apply(builder)?
            .build()
            .map_err(|err| AppError::Http(format!("failed to build http client: {}", err)))?;
        Ok(Self { settings, http })
    }

    pub fn market(&self) -> BinanceMarketApi<'_> {
        BinanceMarketApi::new(self)
    }

    pub fn account(&self) -> BinanceAccountApi<'_> {
        BinanceAccountApi::new(self)
    }

    pub fn futures_market(&self) -> BinanceFuturesMarketApi<'_> {
        BinanceFuturesMarketApi::new(self)
    }

    /// 执行无需签名的 Binance GET 请求。
    ///
    /// 调用方明确提供市场基地址，防止现货路径意外发送到 USDⓈ-M 域名，
    /// 并由本方法统一处理 URL 拼接、query 编码、非 2xx 错误和 JSON 解码。
    pub(crate) async fn get<Q, R>(
        &self,
        base_url: &str,
        endpoint: &str,
        query: &Q,
    ) -> Result<R, AppError>
    where
        Q: Serialize + ?Sized,
        R: DeserializeOwned,
    {
        let response = self
            .http
            .get(Self::url(base_url, endpoint))
            .query(query)
            .send()
            .await
            .map_err(|err| AppError::Http(format!("GET {endpoint} failed: {err}")))?;

        Self::decode_response(endpoint, response).await
    }

    /// 返回配置的现货 REST API 基地址。
    pub(crate) fn spot_base_url(&self) -> &str {
        &self.settings.base_url
    }

    /// 通过现货基地址执行需要 `X-MBX-APIKEY` 的 GET 请求。
    ///
    /// 历史成交 endpoint 虽是公开市场数据，但 Binance 要求 API Key；该请求
    /// 不需要私钥签名。
    pub(crate) async fn api_key_get<Q, R>(&self, endpoint: &str, query: &Q) -> Result<R, AppError>
    where
        Q: Serialize + ?Sized,
        R: DeserializeOwned,
    {
        self.api_key_get_at(&self.settings.base_url, endpoint, query)
            .await
    }

    /// 通过指定市场基地址执行 API Key 认证 GET 请求。
    ///
    /// 使现货与 USDⓈ-M 合约历史成交能共享相同的认证和响应解码逻辑。
    pub(crate) async fn api_key_get_at<Q, R>(
        &self,
        base_url: &str,
        endpoint: &str,
        query: &Q,
    ) -> Result<R, AppError>
    where
        Q: Serialize + ?Sized,
        R: DeserializeOwned,
    {
        let response = self
            .http
            .get(Self::url(base_url, endpoint))
            .query(query)
            .header(
                API_KEY_HEADER,
                self.api_key("BINANCE_API_KEY is required for historical trades")?,
            )
            .send()
            .await
            .map_err(|err| AppError::Http(format!("GET {endpoint} failed: {err}")))?;

        Self::decode_response(endpoint, response).await
    }

    /// 返回配置的 USDⓈ-M 合约 REST API 基地址。
    pub(crate) fn usdm_futures_base_url(&self) -> &str {
        &self.settings.usdm_futures_base_url
    }

    /// 发起需要 Ed25519 私钥签名的私有 Binance 请求。
    ///
    /// 签名输入严格覆盖编码后的 query、`recvWindow` 和 `timestamp`；更改
    /// 任一字段顺序或编码形式都会导致 Binance 拒绝请求。
    #[allow(dead_code)]
    pub(crate) async fn signed_request<Q, R>(
        &self,
        method: Method,
        endpoint: &str,
        query: &Q,
        timestamp_ms: u64,
    ) -> Result<R, AppError>
    where
        Q: Serialize + ?Sized,
        R: DeserializeOwned,
    {
        let api_key = self.api_key("BINANCE_API_KEY is required")?;
        let private_key = self
            .settings
            .private_key
            .as_deref()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| AppError::Config("BINANCE_PRIVATE_KEY is required".to_string()))?;

        let encoded_query = serde_urlencoded::to_string(query)
            .map_err(|err| AppError::Serialization(format!("failed to encode query: {err}")))?;
        let mut canonical_query = encoded_query;
        if !canonical_query.is_empty() {
            canonical_query.push('&');
        }
        canonical_query.push_str(&format!(
            "recvWindow={}&timestamp={timestamp_ms}",
            self.settings.recv_window
        ));
        let signature = sign_query(private_key, &canonical_query)?;
        let signature_query = serde_urlencoded::to_string([("signature", signature)])
            .map_err(|err| AppError::Serialization(format!("failed to encode signature: {err}")))?;
        let url = format!(
            "{}?{canonical_query}&{signature_query}",
            Self::url(self.spot_base_url(), endpoint)
        );

        let response = self
            .http
            .request(method, url)
            .header(API_KEY_HEADER, api_key)
            .send()
            .await
            .map_err(|err| AppError::Http(format!("request {endpoint} failed: {err}")))?;

        Self::decode_response(endpoint, response).await
    }

    #[allow(dead_code)]
    pub(crate) fn current_timestamp_ms() -> Result<u64, AppError> {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| AppError::Http(format!("system clock error: {err}")))?;
        u64::try_from(duration.as_millis())
            .map_err(|_| AppError::Http("system timestamp exceeds u64".to_string()))
    }

    /// 验证 API Key 存在且非空，避免将认证失败延后到远端响应。
    fn api_key(&self, error_message: &'static str) -> Result<&str, AppError> {
        self.settings
            .api_key
            .as_deref()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| AppError::Config(error_message.to_string()))
    }

    /// 将 Binance 错误 JSON 优先转换为结构化 `AppError::Binance`。
    ///
    /// 非 Binance 格式的非成功响应保留 HTTP 状态和正文，便于诊断代理、
    /// 网关或上游服务错误。
    async fn decode_response<R>(endpoint: &str, response: reqwest::Response) -> Result<R, AppError>
    where
        R: DeserializeOwned,
    {
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|err| AppError::Http(format!("failed to read {endpoint} response: {err}")))?;

        if !status.is_success() {
            if let Ok(error) = serde_json::from_str::<BinanceErrorResponse>(&body) {
                return Err(AppError::Binance {
                    code: error.code,
                    message: error.msg,
                });
            }
            return Err(AppError::Http(format!(
                "{endpoint} returned HTTP {status}: {body}"
            )));
        }

        serde_json::from_str(&body).map_err(|err| {
            AppError::Serialization(format!("failed to decode {endpoint} response: {err}"))
        })
    }

    /// 规范化基地址末尾的 `/` 后再拼接固定 endpoint 路径。
    fn url(base_url: &str, endpoint: &str) -> String {
        format!("{}{}", base_url.trim_end_matches('/'), endpoint)
    }
}
