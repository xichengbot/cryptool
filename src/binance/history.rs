//! Strict, bounded market-data decoding for the graphical clients.
use super::BinanceClient;
use exchange_types::{ErrorCode, Market, QueryError, TimeUnit, Timestamp, TradeRow};
use rust_decimal::Decimal;
use serde::{Deserialize, de::DeserializeOwned};
use std::time::SystemTime;

const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug)]
pub(crate) struct Record {
    pub id: u64,
    pub row: TradeRow,
}
#[derive(Deserialize)]
struct HistoricalTrade {
    id: u64,
    price: String,
    qty: String,
    #[serde(rename = "quoteQty")]
    quote_qty: String,
    time: i64,
    #[serde(rename = "isBuyerMaker")]
    maker: bool,
    #[serde(rename = "isBestMatch")]
    best_match: Option<bool>,
    #[serde(rename = "isRPITrade")]
    is_rpi_trade: Option<bool>,
}
#[derive(Deserialize)]
struct AggregateTrade {
    f: u64,
}

impl BinanceClient {
    pub(crate) fn has_history_key(&self) -> bool {
        self.settings
            .api_key
            .as_deref()
            .is_some_and(|s| !s.trim().is_empty())
    }
    pub(crate) async fn aggregate_start(
        &self,
        market: Market,
        symbol: &str,
        start: i64,
        end: i64,
    ) -> Result<Option<u64>, QueryError> {
        let rows: Vec<AggregateTrade> = self
            .history_get(
                market,
                "aggTrades",
                &[
                    ("symbol", symbol.to_owned()),
                    ("startTime", start.to_string()),
                    ("endTime", end.to_string()),
                    ("limit", "1".into()),
                ],
                false,
            )
            .await?;
        if rows.len() > 1 {
            return Err(malformed());
        }
        Ok(rows.first().map(|r| r.f))
    }
    pub(crate) async fn history_page(
        &self,
        market: Market,
        symbol: &str,
        from: u64,
        limit: usize,
    ) -> Result<Vec<Record>, QueryError> {
        let rows: Vec<HistoricalTrade> = self
            .history_get(
                market,
                "historicalTrades",
                &[
                    ("symbol", symbol.to_owned()),
                    ("fromId", from.to_string()),
                    ("limit", limit.to_string()),
                ],
                market == Market::Usdm,
            )
            .await?;
        if rows.len() > limit {
            return Err(malformed());
        }
        rows.into_iter()
            .map(|raw| {
                for value in [&raw.price, &raw.qty, &raw.quote_qty] {
                    let decimal = value.parse::<Decimal>().map_err(|_| malformed())?;
                    if decimal.is_sign_negative() {
                        return Err(malformed());
                    }
                }
                let timestamp = Timestamp {
                    value: raw.time,
                    unit: match market {
                        Market::Spot => TimeUnit::Micros,
                        Market::Usdm => TimeUnit::Millis,
                    },
                };
                if timestamp.to_micros().map_err(|_| malformed())? < 0
                    || (market == Market::Spot && raw.best_match.is_none())
                {
                    return Err(malformed());
                }
                Ok(Record {
                    id: raw.id,
                    row: TradeRow {
                        id: raw.id.to_string(),
                        price: raw.price,
                        qty: raw.qty,
                        quote_qty: raw.quote_qty,
                        timestamp,
                        maker: raw.maker,
                        best_match: if market == Market::Spot {
                            raw.best_match
                        } else {
                            None
                        },
                        is_rpi_trade: if market == Market::Usdm {
                            raw.is_rpi_trade
                        } else {
                            None
                        },
                    },
                })
            })
            .collect()
    }
    async fn history_get<R: DeserializeOwned>(
        &self,
        market: Market,
        endpoint: &str,
        params: &[(&str, String)],
        key: bool,
    ) -> Result<R, QueryError> {
        let (base, prefix) = match market {
            Market::Spot => (self.spot_base_url(), "/api/v3/"),
            Market::Usdm => (self.usdm_futures_base_url(), "/fapi/v1/"),
        };
        let mut request = self
            .http
            .get(format!("{}{prefix}{endpoint}", base.trim_end_matches('/')))
            .query(params);
        if market == Market::Spot {
            request = request.header("X-MBX-TIME-UNIT", "MICROSECOND");
        }
        if key {
            request = request.header(
                "X-MBX-APIKEY",
                self.settings.api_key.as_deref().unwrap_or_default(),
            );
        }
        let mut response = request.send().await.map_err(|_| {
            QueryError::new(ErrorCode::UpstreamError, "无法连接交易所，请检查网络或代理")
        })?;
        let status = response.status();
        if status.as_u16() == 429 || status.as_u16() == 418 {
            let seconds = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|value| {
                    value.parse::<u64>().ok().or_else(|| {
                        httpdate::parse_http_date(value).ok().map(|date| {
                            date.duration_since(SystemTime::now())
                                .map(|d| {
                                    d.as_secs().saturating_add(u64::from(d.subsec_nanos() > 0))
                                })
                                .unwrap_or(1)
                        })
                    })
                })
                .unwrap_or(60)
                .clamp(1, 7 * 24 * 3600);
            let mut error =
                QueryError::new(ErrorCode::RateLimited, "交易所限流，请在冷却结束后重试");
            error.retry_after_secs = Some(seconds);
            return Err(error);
        }
        if response
            .content_length()
            .is_some_and(|len| len > MAX_BODY_BYTES as u64)
        {
            return Err(malformed());
        }
        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| QueryError::new(ErrorCode::UpstreamError, "读取交易所响应失败"))?
        {
            if body.len().saturating_add(chunk.len()) > MAX_BODY_BYTES {
                return Err(malformed());
            }
            body.extend_from_slice(&chunk);
        }
        if !status.is_success() {
            let upstream =
                serde_json::from_slice::<super::response::BinanceErrorResponse>(&body).ok();
            let mut error = match upstream.as_ref().map(|response| response.code) {
                Some(-1121) => QueryError::new(ErrorCode::SymbolNotFound, "交易所未找到该交易对"),
                Some(-4166)
                    if market == Market::Usdm
                        && endpoint == "aggTrades"
                        && status.as_u16() == 400 =>
                {
                    QueryError::new(
                        ErrorCode::InvalidInput,
                        "USD-M 合约的时间锚点仅支持最近 2 天，请改用近期时间或成交 ID",
                    )
                }
                Some(
                    -1100 | -1101 | -1102 | -1103 | -1104 | -1105 | -1106 | -1111 | -1115 | -1116
                    | -1128 | -1130,
                ) => QueryError::new(
                    ErrorCode::InvalidInput,
                    "交易所拒绝了查询参数，请检查成交 ID、时间和数量",
                ),
                Some(-2014 | -2015) => QueryError::new(
                    ErrorCode::CredentialsNotConfigured,
                    "API Key 无效或没有此接口权限，请检查原生配置",
                ),
                _ => QueryError::new(
                    ErrorCode::UpstreamError,
                    format!("交易所返回 HTTP {}", status.as_u16()),
                ),
            };
            error.retryable = error.code == ErrorCode::UpstreamError && status.is_server_error();
            return Err(error);
        }
        serde_json::from_slice(&body).map_err(|_| malformed())
    }
}
fn malformed() -> QueryError {
    let mut error = QueryError::new(
        ErrorCode::UpstreamError,
        "交易所返回的数据格式、精度或大小不符合接口约定",
    );
    error.retryable = false;
    error
}
