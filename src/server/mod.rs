//! HTTP adapter for the shared exchange-data service.

use std::{collections::HashSet, net::Ipv6Addr, path::PathBuf, sync::Arc, time::Instant};

#[cfg(feature = "embed-web")]
use axum::http::Method;
#[cfg(feature = "embed-web")]
use axum::routing::get;
use axum::{
    Json, Router,
    body::HttpBody as _,
    extract::{Path, RawQuery, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{MethodFilter, any, get_service, on},
};
use exchange_types::{
    ErrorCode, Market, QueryError, TimeUnit, TradeAnchor, TradeQuery, parse_time,
};
use tower_http::services::{ServeDir, ServeFile};

use crate::app::exchange_data::ExchangeDataService;

const MAX_URI_BYTES: usize = 8 * 1024;
const SPA_PATHS: [&str; 5] = [
    "/dashboard",
    "/binance/spot-history-trade",
    "/binance/future-history-trade",
    "/tools/time-convert",
    "/",
];

#[derive(Clone, Debug)]
pub struct ServerOptions {
    pub allowed_hosts: Vec<String>,
    pub static_dir: Option<PathBuf>,
}

impl Default for ServerOptions {
    fn default() -> Self {
        Self {
            allowed_hosts: vec!["localhost".into(), "127.0.0.1".into(), "::1".into()],
            static_dir: None,
        }
    }
}

#[derive(Clone)]
struct HostPolicy {
    allowed_hosts: Arc<HashSet<String>>,
}

pub fn router(service: ExchangeDataService, options: ServerOptions) -> Result<Router, QueryError> {
    let allowed_hosts = options
        .allowed_hosts
        .iter()
        .map(|host| normalize_allowed_host(host))
        .collect::<Result<HashSet<_>, _>>()?;
    if allowed_hosts.is_empty() {
        return Err(configuration("at least one allowed host is required"));
    }

    let mut app = Router::new()
        .route(
            "/health",
            on(MethodFilter::GET, health).on(MethodFilter::HEAD, api_method_not_allowed),
        )
        .route(
            "/api/v1/binance/{market}/historical-trades",
            on(MethodFilter::GET, historical_trades).on(MethodFilter::HEAD, api_method_not_allowed),
        )
        .route("/api", any(api_not_found))
        .route("/api/{*path}", any(api_not_found))
        .method_not_allowed_fallback(api_method_not_allowed)
        .with_state(service);

    if let Some(root) = options.static_dir {
        let root = root.canonicalize().map_err(|error| {
            configuration(format!(
                "cannot resolve web directory '{}': {error}",
                root.display()
            ))
        })?;
        let index = root.join("index.html");
        if !index.is_file() {
            return Err(configuration(format!(
                "web directory '{}' does not contain index.html",
                root.display()
            )));
        }
        for path in SPA_PATHS {
            app = app.route(path, get_service(ServeFile::new(index.clone())));
        }
        app = app.fallback_service(ServeDir::new(root));
    } else {
        app = configure_packaged_assets(app);
    }

    let policy = HostPolicy {
        allowed_hosts: Arc::new(allowed_hosts),
    };
    let app = app.layer(middleware::from_fn_with_state(policy, request_guard));
    Ok(app.layer(middleware::from_fn(trace_request)))
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "healthy"}))
}

async fn historical_trades(
    State(service): State<ExchangeDataService>,
    Path(market): Path<String>,
    RawQuery(raw_query): RawQuery,
) -> Result<Json<exchange_types::TradeWindow>, ApiError> {
    let query = parse_trade_query(&market, raw_query.as_deref().unwrap_or_default())?;
    service.query(query).await.map(Json).map_err(ApiError)
}

fn parse_trade_query(market: &str, raw: &str) -> Result<TradeQuery, QueryError> {
    let market = market.parse::<Market>()?;
    let pairs = serde_urlencoded::from_str::<Vec<(String, String)>>(raw)
        .map_err(|_| invalid("query string is not valid URL-encoded data"))?;
    let mut values = std::collections::BTreeMap::new();
    for (key, value) in pairs {
        if !matches!(
            key.as_str(),
            "symbol" | "mode" | "trade_id" | "time" | "time_unit" | "before" | "after"
        ) {
            return Err(invalid(format!("unknown query parameter '{key}'")));
        }
        if values.insert(key.clone(), value).is_some() {
            return Err(invalid(format!("duplicate query parameter '{key}'")));
        }
    }

    let symbol = required(&values, "symbol")?;
    let mode = required(&values, "mode")?;
    let before = parse_count(values.get("before"), "before")?.unwrap_or(50);
    let after = parse_count(values.get("after"), "after")?.unwrap_or(50);
    let anchor = match mode {
        "id" => {
            reject_present(&values, &["time", "time_unit"], "mode=id")?;
            let id = required(&values, "trade_id")?;
            if id.is_empty() || !id.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(invalid("trade_id must be an unsigned decimal integer"));
            }
            TradeAnchor::Id(
                id.parse::<u64>()
                    .map_err(|_| invalid("trade_id is outside the u64 range"))?,
            )
        }
        "time" => {
            reject_present(&values, &["trade_id"], "mode=time")?;
            let time = required(&values, "time")?;
            let unit = required(&values, "time_unit")?.parse::<TimeUnit>()?;
            TradeAnchor::Time(parse_time(time, Some(unit))?)
        }
        _ => return Err(invalid("mode must be 'id' or 'time'")),
    };
    TradeQuery::new(market, symbol, anchor, before, after)
}

fn required<'a>(
    values: &'a std::collections::BTreeMap<String, String>,
    key: &str,
) -> Result<&'a str, QueryError> {
    values
        .get(key)
        .filter(|value| !value.is_empty())
        .map(String::as_str)
        .ok_or_else(|| invalid(format!("query parameter '{key}' is required")))
}

fn reject_present(
    values: &std::collections::BTreeMap<String, String>,
    keys: &[&str],
    mode: &str,
) -> Result<(), QueryError> {
    if let Some(key) = keys.iter().find(|key| values.contains_key(**key)) {
        return Err(invalid(format!(
            "query parameter '{key}' is incompatible with {mode}"
        )));
    }
    Ok(())
}

fn parse_count(value: Option<&String>, name: &str) -> Result<Option<u16>, QueryError> {
    value
        .map(|value| {
            if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(invalid(format!(
                    "{name} must be an unsigned decimal integer"
                )));
            }
            value
                .parse::<u16>()
                .map_err(|_| invalid(format!("{name} is outside the u16 range")))
        })
        .transpose()
}

async fn api_not_found() -> Response {
    error_response(StatusCode::NOT_FOUND, invalid("API 不存在"))
}

async fn api_method_not_allowed() -> Response {
    error_response(StatusCode::METHOD_NOT_ALLOWED, invalid("请求方法不受支持"))
}

async fn request_guard(policy: State<HostPolicy>, request: Request, next: Next) -> Response {
    if request.uri().to_string().len() > MAX_URI_BYTES {
        return secured(error_response(
            StatusCode::BAD_REQUEST,
            invalid("request URI exceeds 8 KiB"),
        ));
    }
    if request.method() == axum::http::Method::GET && get_request_has_body(&request) {
        return secured(error_response(
            StatusCode::BAD_REQUEST,
            invalid("GET requests must not contain a body"),
        ));
    }
    let mut host_values = request.headers().get_all(header::HOST).iter();
    let Some(host_value) = host_values.next() else {
        return secured(error_response(
            StatusCode::FORBIDDEN,
            invalid("invalid Host header"),
        ));
    };
    if host_values.next().is_some() {
        return secured(error_response(
            StatusCode::FORBIDDEN,
            invalid("multiple Host headers are not allowed"),
        ));
    }
    let Some(authority) = host_value
        .to_str()
        .ok()
        .filter(|value| valid_host_authority(value))
        .and_then(|value| value.parse::<axum::http::uri::Authority>().ok())
    else {
        return secured(error_response(
            StatusCode::FORBIDDEN,
            invalid("invalid Host header"),
        ));
    };
    let host = authority.host().to_ascii_lowercase();
    if !policy.allowed_hosts.contains(&host) {
        return secured(error_response(
            StatusCode::FORBIDDEN,
            invalid("Host is not allowed"),
        ));
    }
    if !same_origin(request.headers(), &authority) {
        return secured(error_response(
            StatusCode::FORBIDDEN,
            invalid("cross-site Origin is not allowed"),
        ));
    }
    secured(next.run(request).await)
}

async fn trace_request(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let started = Instant::now();
    let response = next.run(request).await;
    tracing::info!(
        %method,
        %path,
        status = response.status().as_u16(),
        elapsed_ms = started.elapsed().as_millis(),
        "HTTP request completed"
    );
    response
}

fn same_origin(headers: &HeaderMap, request_authority: &axum::http::uri::Authority) -> bool {
    let mut origins = headers.get_all(header::ORIGIN).iter();
    let Some(origin) = origins.next() else {
        return true;
    };
    if origins.next().is_some() {
        return false;
    }
    let Ok(origin) = origin.to_str() else {
        return false;
    };
    let Ok(uri) = origin.parse::<axum::http::Uri>() else {
        return false;
    };
    matches!(uri.scheme_str(), Some("http" | "https"))
        && uri
            .path_and_query()
            .is_some_and(|value| value.as_str() == "/")
        && uri.authority().is_some_and(|authority| {
            valid_host_authority(authority.as_str())
                && authority
                    .as_str()
                    .eq_ignore_ascii_case(request_authority.as_str())
        })
}

fn get_request_has_body(request: &Request) -> bool {
    let headers = request.headers();
    if headers.contains_key(header::TRANSFER_ENCODING) {
        return true;
    }
    let mut lengths = headers.get_all(header::CONTENT_LENGTH).iter();
    if let Some(length) = lengths.next()
        && (lengths.next().is_some()
            || length
                .to_str()
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                != Some(0))
    {
        return true;
    }
    let body = request.body();
    !body.is_end_stream() || body.size_hint().exact() != Some(0)
}

fn normalize_allowed_host(input: &str) -> Result<String, QueryError> {
    let input = input.trim();
    if input.is_empty() || input.contains('/') || input.contains('@') {
        return Err(configuration(format!("invalid allowed host '{input}'")));
    }
    let authority = if valid_host_authority(input) {
        input.parse::<axum::http::uri::Authority>().ok()
    } else if input.parse::<Ipv6Addr>().is_ok() {
        format!("[{input}]")
            .parse::<axum::http::uri::Authority>()
            .ok()
    } else {
        None
    }
    .ok_or_else(|| configuration(format!("invalid allowed host '{input}'")))?;
    Ok(authority.host().to_ascii_lowercase())
}

fn valid_host_authority(input: &str) -> bool {
    if input.is_empty() || input.contains('@') {
        return false;
    }
    if let Some(rest) = input.strip_prefix('[') {
        let Some(close) = rest.find(']') else {
            return false;
        };
        let suffix = &rest[close + 1..];
        return suffix.is_empty() || suffix.strip_prefix(':').is_some_and(valid_port);
    }
    match input.matches(':').count() {
        0 => true,
        1 => input
            .rsplit_once(':')
            .is_some_and(|(host, port)| !host.is_empty() && valid_port(port)),
        _ => false,
    }
}

fn valid_port(port: &str) -> bool {
    !port.is_empty()
        && port.bytes().all(|byte| byte.is_ascii_digit())
        && port.parse::<u16>().is_ok()
}

fn secured(mut response: Response) -> Response {
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
        .headers_mut()
        .insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    response
}

fn error_response(status: StatusCode, error: QueryError) -> Response {
    let retry_after = error.retry_after_secs;
    let mut response = (status, Json(error)).into_response();
    if let Some(seconds) = retry_after
        && let Ok(value) = HeaderValue::from_str(&seconds.to_string())
    {
        response.headers_mut().insert(header::RETRY_AFTER, value);
    }
    response
}

struct ApiError(QueryError);

impl From<QueryError> for ApiError {
    fn from(error: QueryError) -> Self {
        Self(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.0.code {
            ErrorCode::InvalidInput => StatusCode::BAD_REQUEST,
            ErrorCode::SymbolNotFound => StatusCode::NOT_FOUND,
            ErrorCode::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            ErrorCode::CredentialsNotConfigured | ErrorCode::Configuration => {
                StatusCode::SERVICE_UNAVAILABLE
            }
            ErrorCode::UpstreamError => StatusCode::BAD_GATEWAY,
            ErrorCode::QueryBudgetExceeded => StatusCode::GATEWAY_TIMEOUT,
            ErrorCode::Cancelled => StatusCode::REQUEST_TIMEOUT,
        };
        error_response(status, self.0)
    }
}

fn invalid(message: impl Into<String>) -> QueryError {
    QueryError::new(ErrorCode::InvalidInput, message)
}

fn configuration(message: impl Into<String>) -> QueryError {
    QueryError::new(ErrorCode::Configuration, message)
}

#[cfg(not(feature = "embed-web"))]
fn configure_packaged_assets(app: Router) -> Router {
    app.fallback(static_not_found)
}

#[cfg(not(feature = "embed-web"))]
async fn static_not_found() -> StatusCode {
    StatusCode::NOT_FOUND
}

#[cfg(feature = "embed-web")]
#[derive(rust_embed::RustEmbed)]
#[folder = "web-dist/public/"]
struct WebAssets;

#[cfg(feature = "embed-web")]
fn configure_packaged_assets(mut app: Router) -> Router {
    for path in SPA_PATHS {
        app = app.route(path, get(embedded_index));
    }
    app.fallback(embedded_asset)
}

#[cfg(feature = "embed-web")]
async fn embedded_index() -> Response {
    embedded_response("index.html")
}

#[cfg(feature = "embed-web")]
async fn embedded_asset(method: Method, request: Request) -> Response {
    if method != Method::GET {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }
    embedded_response(request.uri().path().trim_start_matches('/'))
}

#[cfg(feature = "embed-web")]
fn embedded_response(path: &str) -> Response {
    let Some(asset) = WebAssets::get(path) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let content_type = mime_guess::from_path(path).first_or_octet_stream();
    (
        [(header::CONTENT_TYPE, content_type.as_ref())],
        asset.data.into_owned(),
    )
        .into_response()
}
