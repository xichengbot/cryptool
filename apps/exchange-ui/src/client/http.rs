use exchange_types::{ErrorCode, QueryError, TradeAnchor, TradeQuery, TradeWindow};
use gloo_net::http::Request;
use web_sys::AbortController;

use super::{QueryClient, QueryFuture};

pub struct HttpQueryClient;

struct AbortOnDrop(AbortController);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

impl QueryClient for HttpQueryClient {
    fn query_trades(&self, query: TradeQuery) -> QueryFuture {
        Box::pin(async move {
            let market = query.market.as_str();
            let mut params = vec![
                ("symbol", query.symbol.clone()),
                ("before", query.before.to_string()),
                ("after", query.after.to_string()),
            ];
            match query.anchor {
                TradeAnchor::Id(value) => {
                    params.push(("mode", "id".into()));
                    params.push(("trade_id", value.to_string()));
                }
                TradeAnchor::Time(timestamp) => {
                    params.push(("mode", "time".into()));
                    params.push(("time", timestamp.value.to_string()));
                    params.push(("time_unit", timestamp.unit.to_string()));
                }
            }
            let encoded = serde_urlencoded::to_string(params)
                .map_err(|_| QueryError::new(ErrorCode::InvalidInput, "无法编码查询参数"))?;
            let url = format!("/api/v1/binance/{market}/historical-trades?{encoded}");
            let controller = AbortController::new().map_err(|_| {
                QueryError::new(ErrorCode::UpstreamError, "浏览器无法创建可取消请求")
            })?;
            let abort_on_drop = AbortOnDrop(controller);
            let response = Request::get(&url)
                .abort_signal(Some(&abort_on_drop.0.signal()))
                .send()
                .await
                .map_err(|_| QueryError::new(ErrorCode::UpstreamError, "无法连接查询服务"))?;
            if response.ok() {
                return response.json::<TradeWindow>().await.map_err(|_| {
                    QueryError::new(ErrorCode::UpstreamError, "查询服务返回了无效数据")
                });
            }
            let status = response.status();
            match response.json::<QueryError>().await {
                Ok(error) => Err(error),
                Err(_) => Err(QueryError::new(
                    ErrorCode::UpstreamError,
                    format!("查询服务返回 HTTP {status}"),
                )),
            }
        })
    }
}
