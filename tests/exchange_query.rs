mod support;

use cryptool::app::exchange_data::ExchangeDataService;
use exchange_types::{ErrorCode, Market, TimeUnit, Timestamp, TradeAnchor, TradeQuery};
use serde_json::json;
use support::{Fixture, Response, history};

fn query(market: Market, before: u16, after: u16) -> TradeQuery {
    TradeQuery::new(market, "BTCUSDT", TradeAnchor::Id(1000), before, after).unwrap()
}

#[tokio::test]
async fn futures_preserve_rpi_true_false_and_unavailable_flags() {
    let mock = Fixture::start(|_, req| {
        let mut response = history(req, false);
        let rows = response.body.as_array_mut().unwrap();
        rows[0]["isRPITrade"] = json!(true);
        rows[1]["isRPITrade"] = json!(false);
        rows[3]["isRPITrade"] = json!(null);
        response
    })
    .await;
    let result = ExchangeDataService::new(mock.settings())
        .unwrap()
        .query(query(Market::Usdm, 0, 3))
        .await
        .unwrap();
    let json = serde_json::to_value(result).unwrap();
    let flags = json["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row.get("is_rpi_trade").cloned())
        .collect::<Vec<_>>();
    assert_eq!(
        flags,
        vec![
            Some(json!(true)),
            Some(json!(false)),
            Some(json!(null)),
            Some(json!(null))
        ]
    );
}

#[tokio::test]
async fn futures_thousand_records_use_two_bounded_pages() {
    let mock = Fixture::start(|_, req| history(req, false)).await;
    let result = ExchangeDataService::new(mock.settings())
        .unwrap()
        .query(query(Market::Usdm, 499, 500))
        .await
        .unwrap();
    assert_eq!(result.items.len(), 1000);
    assert_eq!(result.items.first().unwrap().id, "501");
    assert_eq!(result.items.last().unwrap().id, "1500");
    assert_eq!(result.meta.actual_before, 499);
    assert_eq!(result.meta.actual_after, 500);
    assert!(!result.meta.partial);
    assert_eq!(result.meta.upstream_calls, 2);
    let calls = mock.requests();
    assert_eq!(
        calls
            .iter()
            .map(|r| r.query["limit"].as_str())
            .collect::<Vec<_>>(),
        ["500", "500"]
    );
    assert_eq!(calls[1].query["fromId"], "1001");
    assert_eq!(calls[0].headers["x-mbx-apikey"], "fixture-key");
    assert_eq!(result.items[0].timestamp.unit, TimeUnit::Millis);
}

#[tokio::test]
async fn window_at_id_lower_bound_reports_actual_counts_and_partial_boundary() {
    let mock = Fixture::start(|_, req| history(req, true)).await;
    let request = TradeQuery::new(Market::Spot, "BTCUSDT", TradeAnchor::Id(2), 5, 2).unwrap();
    let result = ExchangeDataService::new(mock.settings())
        .unwrap()
        .query(request)
        .await
        .unwrap();

    assert_eq!(mock.requests()[0].query["fromId"], "0");
    assert_eq!(result.items.first().unwrap().id, "0");
    assert_eq!(result.items.last().unwrap().id, "7");
    assert_eq!(result.meta.anchor_id.as_deref(), Some("2"));
    assert_eq!(result.meta.requested, 8);
    assert_eq!(result.meta.received, 8);
    assert_eq!(result.meta.actual_before, 2);
    assert_eq!(result.meta.actual_after, 5);
    assert!(result.meta.partial);
    assert!(result.meta.boundary.as_deref().unwrap().contains("下界"));
    assert_eq!(result.meta.upstream_calls, 1);
}

#[tokio::test]
async fn spot_uses_microseconds_without_requiring_credentials() {
    let mock = Fixture::start(|_, req| history(req, true)).await;
    let mut settings = mock.settings();
    settings.binance.api_key = None;
    let result = ExchangeDataService::new(settings)
        .unwrap()
        .query(query(Market::Spot, 0, 0))
        .await
        .unwrap();
    assert_eq!(result.items[0].timestamp.value, 1_700_000_000_123_456);
    assert_eq!(result.items[0].timestamp.unit, TimeUnit::Micros);
    let calls = mock.requests();
    assert_eq!(calls[0].headers["x-mbx-time-unit"], "MICROSECOND");
    assert!(!calls[0].headers.contains_key("x-mbx-apikey"));
}

#[tokio::test]
async fn time_anchor_uses_f_and_checks_the_original_timestamp() {
    let mock = Fixture::start(|_, req| {
        if req.path.ends_with("aggTrades") {
            Response::ok(json!([{"a":7,"f":1000}]))
        } else {
            history(req, true)
        }
    })
    .await;
    let mut request = query(Market::Spot, 2, 2);
    request.anchor = TradeAnchor::Time(Timestamp {
        value: 1_700_000_000_123_458,
        unit: TimeUnit::Micros,
    });
    let result = ExchangeDataService::new(mock.settings())
        .unwrap()
        .query(request)
        .await
        .unwrap();
    assert_eq!(result.meta.anchor_id.as_deref(), Some("1002"));
    assert_eq!(result.items.first().unwrap().id, "1000");
    assert_eq!(result.items.last().unwrap().id, "1004");
    let calls = mock.requests();
    assert_eq!(calls[0].query["startTime"], "1700000000123458");
    assert_eq!(calls[0].query["endTime"], "1700000001123458");
    assert_eq!(calls[1].query["fromId"], "998");
    assert!(calls.len() <= 6);
}

#[tokio::test]
async fn no_aggregate_trades_returns_an_empty_window() {
    let mock = Fixture::start(|_, _| Response::ok(json!([]))).await;
    let mut request = query(Market::Spot, 50, 50);
    request.anchor = TradeAnchor::Time(Timestamp {
        value: 1_700_000_000_123_456,
        unit: TimeUnit::Micros,
    });
    let result = ExchangeDataService::new(mock.settings())
        .unwrap()
        .query(request)
        .await
        .unwrap();
    assert!(result.items.is_empty());
    assert!(result.meta.anchor_id.is_none());
    assert!(result.meta.boundary.is_some());
    assert_eq!(mock.requests().len(), 1);
}

#[tokio::test]
async fn futures_time_outside_binance_window_reports_the_two_day_limit() {
    let mock = Fixture::start(|_, req| {
        assert!(req.path.ends_with("aggTrades"));
        Response {
            status: 400,
            body: json!({
                "code": -4166,
                "msg": "Search window is restricted to recent 2 days only."
            }),
            ..Response::ok(json!([]))
        }
    })
    .await;
    let mut request = query(Market::Usdm, 50, 50);
    request.anchor = TradeAnchor::Time(Timestamp {
        value: 1_700_000_000_000,
        unit: TimeUnit::Millis,
    });

    let error = ExchangeDataService::new(mock.settings())
        .unwrap()
        .query(request)
        .await
        .unwrap_err();

    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert_eq!(
        error.message,
        "USD-M 合约的时间锚点仅支持最近 2 天，请改用近期时间或成交 ID"
    );
    assert!(!error.retryable);
    assert_eq!(mock.requests().len(), 1);
}

#[tokio::test]
async fn binance_rejected_parameters_are_reported_as_invalid_input() {
    let mock = Fixture::start(|_, _| Response {
        status: 400,
        body: json!({
            "code": -1102,
            "msg": "Mandatory parameter 'fromid' was not sent, was empty/null, or malformed."
        }),
        ..Response::ok(json!([]))
    })
    .await;

    let error = ExchangeDataService::new(mock.settings())
        .unwrap()
        .query(query(Market::Usdm, 0, 0))
        .await
        .unwrap_err();

    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert_eq!(
        error.message,
        "交易所拒绝了查询参数，请检查成交 ID、时间和数量"
    );
    assert!(!error.retryable);
}

#[tokio::test]
async fn invalid_input_never_becomes_retryable_from_the_http_status() {
    let mock = Fixture::start(|_, _| Response {
        status: 500,
        body: json!({"code": -1102, "msg": "malformed parameter"}),
        ..Response::ok(json!([]))
    })
    .await;

    let error = ExchangeDataService::new(mock.settings())
        .unwrap()
        .query(query(Market::Usdm, 0, 0))
        .await
        .unwrap_err();

    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert!(!error.retryable);
    assert_eq!(mock.requests().len(), 1);
}

#[tokio::test]
async fn binance_two_day_code_is_scoped_to_futures_time_lookup() {
    let mock = Fixture::start(|_, _| Response {
        status: 400,
        body: json!({"code": -4166, "msg": "different endpoint context"}),
        ..Response::ok(json!([]))
    })
    .await;

    let error = ExchangeDataService::new(mock.settings())
        .unwrap()
        .query(query(Market::Spot, 0, 0))
        .await
        .unwrap_err();

    assert_eq!(error.code, ErrorCode::UpstreamError);
    assert_eq!(error.message, "交易所返回 HTTP 400");
    assert!(!error.retryable);
}

#[tokio::test]
async fn missing_futures_key_is_reported_before_network_io() {
    let mock = Fixture::start(|_, req| history(req, false)).await;
    let mut settings = mock.settings();
    settings.binance.api_key = None;
    let error = ExchangeDataService::new(settings)
        .unwrap()
        .query(query(Market::Usdm, 0, 0))
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::CredentialsNotConfigured);
    assert!(mock.requests().is_empty());
}

#[tokio::test]
async fn direct_service_validates_a_manually_constructed_query() {
    let mock = Fixture::start(|_, req| history(req, false)).await;
    let mut request = query(Market::Usdm, 0, 0);
    request.before = 1000;
    let error = ExchangeDataService::new(mock.settings())
        .unwrap()
        .query(request)
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert!(mock.requests().is_empty());
}

#[tokio::test]
async fn rate_limit_preserves_cooldown_and_does_not_busy_retry() {
    let mock = Fixture::start(|_, _| Response {
        status: 429,
        body: json!({"code":-1003,"msg":"fixture limited"}),
        headers: vec![("Retry-After".into(), "60".into())],
        delay: std::time::Duration::ZERO,
    })
    .await;
    let service = ExchangeDataService::new(mock.settings()).unwrap();
    let error = service.query(query(Market::Spot, 0, 0)).await.unwrap_err();
    assert_eq!(error.code, ErrorCode::RateLimited);
    assert_eq!(error.retry_after_secs, Some(60));
    let second = service.query(query(Market::Spot, 0, 0)).await.unwrap_err();
    assert_eq!(second.code, ErrorCode::RateLimited);
    assert_eq!(mock.requests().len(), 1);
}

#[tokio::test]
async fn second_page_failure_is_explicitly_partial() {
    let mock = Fixture::start(|_, req| {
        if req.query["fromId"] == "1001" {
            Response {
                status: 502,
                body: json!({"error":"fixture unavailable"}),
                ..Response::ok(json!([]))
            }
        } else {
            history(req, false)
        }
    })
    .await;
    let result = ExchangeDataService::new(mock.settings())
        .unwrap()
        .query(query(Market::Usdm, 499, 500))
        .await
        .unwrap();
    assert_eq!(result.items.len(), 500);
    assert!(result.meta.partial);
    assert!(result.meta.boundary.is_some());
    assert!(result.meta.upstream_calls <= 6);
}

#[tokio::test]
async fn id_at_u64_max_does_not_wrap_pagination_cursor() {
    let mock = Fixture::start(|_, _| {
        Response::ok(json!([{
            "id":u64::MAX,"price":"1","qty":"2","quoteQty":"2",
            "time":1_700_000_000_123_456_i64,"isBuyerMaker":true,"isBestMatch":true
        }]))
    })
    .await;
    let request =
        TradeQuery::new(Market::Spot, "BTCUSDT", TradeAnchor::Id(u64::MAX), 0, 1).unwrap();
    let result = ExchangeDataService::new(mock.settings())
        .unwrap()
        .query(request)
        .await
        .unwrap();
    assert_eq!(result.items[0].id, u64::MAX.to_string());
    assert!(result.meta.partial);
    assert_eq!(mock.requests().len(), 1);
}

#[tokio::test]
async fn total_deadline_releases_the_concurrency_permit() {
    let mock = Fixture::start(|_, req| {
        let mut response = history(req, true);
        response.delay = std::time::Duration::from_millis(100);
        response
    })
    .await;
    let mut settings = mock.settings();
    settings.exchange_data.total_timeout_ms = 20;
    settings.exchange_data.max_concurrent = 1;
    let service = ExchangeDataService::new(settings).unwrap();
    for _ in 0..2 {
        let error = service.query(query(Market::Spot, 0, 0)).await.unwrap_err();
        assert_eq!(error.code, ErrorCode::QueryBudgetExceeded);
    }
}

#[tokio::test]
async fn malformed_decimal_is_not_presented_as_valid_market_data() {
    let mock = Fixture::start(|_, _| {
        Response::ok(json!([{
            "id":1000,"price":"NaN","qty":"1","quoteQty":"1",
            "time":1_700_000_000_123_456_i64,"isBuyerMaker":true,"isBestMatch":true
        }]))
    })
    .await;
    let error = ExchangeDataService::new(mock.settings())
        .unwrap()
        .query(query(Market::Spot, 0, 0))
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::UpstreamError);
    assert!(!error.retryable);
    assert_eq!(mock.requests().len(), 1);
}

#[tokio::test]
async fn repeated_pages_stop_without_duplicate_rows_or_cursor_wrap() {
    let mock = Fixture::start(|_, req| {
        let row = json!({
            "id":1000,"price":"1","qty":"1","quoteQty":"1",
            "time":1_700_000_000_123_456_i64,"isBuyerMaker":true,"isBestMatch":true
        });
        Response::ok(json!(vec![
            row;
            req.query["limit"].parse::<usize>().unwrap()
        ]))
    })
    .await;
    let window = ExchangeDataService::new(mock.settings())
        .unwrap()
        .query(query(Market::Spot, 0, 3))
        .await
        .unwrap();
    assert_eq!(window.items.len(), 1);
    assert!(window.meta.partial);
    assert_eq!(window.meta.upstream_calls, 2);
}

#[test]
fn native_configuration_redacts_credentials_and_rejects_wrong_proxy_scheme() {
    use cryptool::config::{ProxyMode, ProxySettings, Settings};
    let mut settings = Settings::default();
    settings.binance.api_key = Some("test-key-must-not-leak".into());
    settings.binance.private_key = Some("private-key-must-not-leak".into());
    settings.binance.proxy = ProxySettings {
        mode: ProxyMode::Http,
        url: Some("socks5://127.0.0.1:8080".into()),
        username: Some("fixture-user".into()),
        password: Some("proxy-must-not-leak".into()),
    };
    let debug = format!("{settings:?}");
    assert!(!debug.contains("must-not-leak"));
    let error = ExchangeDataService::new(settings).err().unwrap();
    assert_eq!(error.code, ErrorCode::Configuration);
    assert!(!error.message.contains("must-not-leak"));
}

#[tokio::test]
async fn request_budget_limits_pagination_even_when_more_rows_exist() {
    let mock = Fixture::start(|_, req| history(req, false)).await;
    let mut settings = mock.settings();
    settings.exchange_data.max_upstream_requests = 1;
    let window = ExchangeDataService::new(settings)
        .unwrap()
        .query(query(Market::Usdm, 499, 500))
        .await
        .unwrap();
    assert_eq!(window.items.len(), 500);
    assert!(window.meta.partial);
    assert_eq!(window.meta.upstream_calls, 1);
    assert_eq!(mock.requests().len(), 1);
}

#[tokio::test]
async fn cancelling_a_query_releases_capacity_for_the_next_query() {
    let mock = Fixture::start(|_, req| {
        let mut response = history(req, true);
        response.delay = std::time::Duration::from_millis(50);
        response
    })
    .await;
    let mut settings = mock.settings();
    settings.exchange_data.max_concurrent = 1;
    let service = ExchangeDataService::new(settings).unwrap();
    let mut pending = Box::pin(service.query(query(Market::Spot, 0, 0)));
    tokio::select! {
        result=&mut pending=>panic!("expected query in flight: {result:?}"),
        _=async {while mock.requests().is_empty() {tokio::task::yield_now().await;}}=>{}
    }
    let busy = service.query(query(Market::Spot, 0, 0)).await.unwrap_err();
    assert_eq!(busy.code, ErrorCode::RateLimited);
    drop(pending);
    let window = service.query(query(Market::Spot, 0, 0)).await.unwrap();
    assert_eq!(window.items.len(), 1);
}

#[tokio::test]
async fn oversized_content_length_is_rejected_before_waiting_for_the_body() {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut input = Vec::new();
        let mut byte = [0];
        while !input.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).await.unwrap();
            input.push(byte[0]);
        }
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2097153\r\n\r\n")
            .await
            .unwrap();
        std::future::pending::<()>().await;
    });
    let mut settings = cryptool::config::Settings::default();
    settings.binance.base_url = format!("http://{address}");
    settings.binance.proxy.mode = cryptool::config::ProxyMode::Direct;
    settings.exchange_data.total_timeout_ms = 1000;
    let result = ExchangeDataService::new(settings)
        .unwrap()
        .query(query(Market::Spot, 0, 0))
        .await;
    server.abort();
    let error = result.unwrap_err();
    assert_eq!(error.code, ErrorCode::UpstreamError);
    assert!(!error.retryable);
}
