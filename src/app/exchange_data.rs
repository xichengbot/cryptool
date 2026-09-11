//! Shared native query service used by Axum and Dioxus Desktop.
use crate::{
    binance::{BinanceClient, history::Record},
    config::{QuerySettings, Settings},
};
use exchange_types::{
    ErrorCode, Market, QueryError, TradeAnchor, TradeQuery, TradeRow, TradeWindow, TradeWindowMeta,
};
use std::{
    collections::BTreeMap,
    future::Future,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::Semaphore;

#[derive(Clone)]
pub struct ExchangeDataService {
    inner: Arc<Inner>,
}
struct Inner {
    client: BinanceClient,
    budgets: QuerySettings,
    permits: Semaphore,
    cooldown: Mutex<Option<Instant>>,
}
impl ExchangeDataService {
    pub fn new(mut settings: Settings) -> Result<Self, QueryError> {
        let invalid = || {
            QueryError::new(
                ErrorCode::Configuration,
                "查询配置无效，请检查网络地址、代理和请求预算",
            )
        };
        settings.exchange_data.validate().map_err(|_| invalid())?;
        for base in [
            &settings.binance.base_url,
            &settings.binance.usdm_futures_base_url,
        ] {
            let url = reqwest::Url::parse(base).map_err(|_| invalid())?;
            if !matches!(url.scheme(), "https" | "http")
                || url.host_str().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
                || url.query().is_some()
                || url.fragment().is_some()
            {
                return Err(invalid());
            }
        }
        if settings.binance.timeout_ms == 0 {
            return Err(invalid());
        }
        if let Some(key) = &settings.binance.api_key {
            reqwest::header::HeaderValue::from_str(key).map_err(|_| invalid())?;
        }
        settings.binance.private_key = None;
        settings.binance.timeout_ms = settings
            .binance
            .timeout_ms
            .min(settings.exchange_data.request_timeout_ms);
        let client = BinanceClient::new(settings.binance).map_err(|_| invalid())?;
        Ok(Self {
            inner: Arc::new(Inner {
                client,
                permits: Semaphore::new(settings.exchange_data.max_concurrent),
                budgets: settings.exchange_data,
                cooldown: Mutex::new(None),
            }),
        })
    }
    /// Dropping this future cancels its HTTP requests and releases the permit.
    pub async fn query(&self, query: TradeQuery) -> Result<TradeWindow, QueryError> {
        query.validate()?;
        if query.market == Market::Usdm && !self.inner.client.has_history_key() {
            return Err(QueryError::new(
                ErrorCode::CredentialsNotConfigured,
                "合约历史成交需要 API Key，请在原生配置或 BINANCE_API_KEY 环境变量中设置",
            ));
        }
        self.check_cooldown()?;
        let _permit = self.inner.permits.try_acquire().map_err(|_| {
            let mut error = QueryError::new(ErrorCode::RateLimited, "同时查询过多，请稍后重试");
            error.retry_after_secs = Some(1);
            error
        })?;
        tokio::time::timeout(
            Duration::from_millis(self.inner.budgets.total_timeout_ms),
            self.window(query),
        )
        .await
        .map_err(|_| budget("查询超出总时间预算，请缩小窗口后重试"))?
    }
    async fn window(&self, query: TradeQuery) -> Result<TradeWindow, QueryError> {
        let requested = query.limit()?;
        let mut calls = 0;
        let mut anchor = match query.anchor {
            TradeAnchor::Id(id) => Some(id),
            TradeAnchor::Time(_) => None,
        };
        let time_range = match query.anchor {
            TradeAnchor::Time(timestamp) => {
                let start = timestamp.to_micros()?;
                let end = start
                    .checked_add(1_000_000)
                    .ok_or_else(|| QueryError::new(ErrorCode::InvalidInput, "时间超出支持范围"))?;
                Some((start, end))
            }
            _ => None,
        };
        let candidate = if let Some((start, end)) = time_range {
            let divisor = if query.market == Market::Usdm {
                1000
            } else {
                1
            };
            self.call(&mut calls, || {
                self.inner.client.aggregate_start(
                    query.market,
                    &query.symbol,
                    start / divisor,
                    end / divisor,
                )
            })
            .await?
        } else {
            anchor
        };
        let Some(candidate) = candidate else {
            return Ok(finish(
                query,
                BTreeMap::new(),
                None,
                calls,
                Some("指定时间之后 1 秒内没有成交".into()),
                false,
            ));
        };
        let mut cursor = candidate.saturating_sub(u64::from(query.before));
        let mut rows: BTreeMap<u64, TradeRow> = BTreeMap::new();
        let mut boundary = None;
        let page_max = if query.market == Market::Usdm {
            500
        } else {
            1000
        };
        loop {
            let remaining = if anchor.is_some() {
                usize::from(requested).saturating_sub(rows.len())
            } else {
                usize::from(requested)
            };
            if remaining == 0 {
                break;
            }
            let limit = remaining.min(page_max);
            let page: Vec<Record> = match self
                .call(&mut calls, || {
                    self.inner
                        .client
                        .history_page(query.market, &query.symbol, cursor, limit)
                })
                .await
            {
                Ok(page) => page,
                Err(error) if anchor.is_some() && !rows.is_empty() => {
                    boundary = Some(error.message);
                    break;
                }
                Err(error) => return Err(error),
            };
            let page_len = page.len();
            let last_id = page.iter().map(|row| row.id).max();
            for record in page {
                if record.id >= cursor {
                    rows.entry(record.id).or_insert(record.row);
                }
            }
            if let Some((start, end)) = time_range
                && anchor.is_none()
            {
                anchor = rows.iter().find_map(|(id, row)| {
                    row.timestamp
                        .to_micros()
                        .ok()
                        .filter(|&t| t >= start && t <= end)
                        .map(|_| *id)
                });
            }
            if let Some(id) = anchor {
                let start = id.saturating_sub(u64::from(query.before));
                rows.retain(|id, _| *id >= start);
                if rows.len() >= usize::from(requested) {
                    break;
                }
            } else {
                // Before the original trade is resolved, retain only a bounded look-behind.
                while rows.len() > usize::from(query.before) + page_max {
                    rows.pop_first();
                }
            }
            if page_len == 0 {
                boundary = Some("已到达可查询数据边界".into());
                break;
            }
            let Some(next) = last_id
                .and_then(|id| id.checked_add(1))
                .filter(|next| *next > cursor)
            else {
                boundary = Some("上游分页未前进或成交 ID 已到上界".into());
                break;
            };
            if page_len < limit {
                boundary = Some("交易所返回的可用成交不足请求窗口".into());
                break;
            }
            cursor = next;
        }
        if anchor.is_none() {
            if time_range.is_some() && !rows.is_empty() {
                return Err(budget("未能在查询预算内确认原始成交时间锚点"));
            }
            rows.clear();
        }
        while rows.len() > usize::from(requested) {
            rows.pop_last();
        }
        if let Some(id) = anchor {
            if id < u64::from(query.before) {
                boundary.get_or_insert("窗口前沿已到成交 ID 下界".into());
            }
            if !rows.contains_key(&id) {
                boundary.get_or_insert("可用数据中缺少请求的锚点成交".into());
                anchor = None;
            }
        }
        Ok(finish(query, rows, anchor, calls, boundary, true))
    }
    async fn call<R, F, Fut>(&self, calls: &mut usize, operation: F) -> Result<R, QueryError>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<R, QueryError>>,
    {
        for attempt in 0..2 {
            self.check_cooldown()?;
            if *calls >= self.inner.budgets.max_upstream_requests {
                return Err(budget("已达到上游请求次数上限"));
            }
            *calls += 1;
            match operation().await {
                Ok(value) => return Ok(value),
                Err(error) => {
                    if error.code == ErrorCode::RateLimited {
                        let seconds = error.retry_after_secs.unwrap_or(60).min(7 * 24 * 3600);
                        let until = Instant::now() + Duration::from_secs(seconds);
                        let mut cooldown = self
                            .inner
                            .cooldown
                            .lock()
                            .unwrap_or_else(|e| e.into_inner());
                        *cooldown = Some(cooldown.map_or(until, |old| old.max(until)));
                    }
                    if error.code != ErrorCode::UpstreamError || !error.retryable || attempt == 1 {
                        return Err(error);
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
        unreachable!("bounded retry loop always returns")
    }
    fn check_cooldown(&self) -> Result<(), QueryError> {
        let cooldown = self
            .inner
            .cooldown
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(remaining) =
            cooldown.and_then(|until| until.checked_duration_since(Instant::now()))
        {
            let mut error = QueryError::new(ErrorCode::RateLimited, "交易所限流冷却尚未结束");
            error.retry_after_secs = Some(
                remaining
                    .as_secs()
                    .saturating_add(u64::from(remaining.subsec_nanos() > 0)),
            );
            return Err(error);
        }
        Ok(())
    }
}
fn budget(message: &str) -> QueryError {
    QueryError::new(ErrorCode::QueryBudgetExceeded, message)
}
fn finish(
    query: TradeQuery,
    rows: BTreeMap<u64, TradeRow>,
    anchor: Option<u64>,
    calls: usize,
    boundary: Option<String>,
    check_partial: bool,
) -> TradeWindow {
    let requested = query.before + query.after + 1; // validated by query() before entering window()
    let actual_before = anchor.map_or(0, |anchor| rows.keys().filter(|&&id| id < anchor).count());
    let actual_after = anchor.map_or(0, |anchor| rows.keys().filter(|&&id| id > anchor).count());
    let partial = check_partial
        && (rows.len() != usize::from(requested)
            || anchor.is_none()
            || actual_before < usize::from(query.before)
            || actual_after < usize::from(query.after)
            || boundary.is_some());
    let meta = TradeWindowMeta {
        query,
        anchor_id: anchor.map(|id| id.to_string()),
        requested,
        received: rows.len(),
        actual_before,
        actual_after,
        boundary,
        upstream_calls: calls,
        partial,
    };
    TradeWindow {
        items: rows.into_values().collect(),
        meta,
    }
}
