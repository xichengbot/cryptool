#![cfg(feature = "server")]
mod support;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use cryptool::{
    app::exchange_data::ExchangeDataService,
    server::{ServerOptions, router},
};
use exchange_types::{Market, TradeAnchor, TradeQuery, TradeWindow};
use http_body_util::BodyExt;
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use support::{Fixture, history};
use tower::ServiceExt;

async fn app() -> (Fixture, axum::Router) {
    let fixture = Fixture::start(|_, req| history(req, true)).await;
    let service = ExchangeDataService::new(fixture.settings()).unwrap();
    let app = router(service, ServerOptions::default()).unwrap();
    (fixture, app)
}
fn request(path: &str, host: &str) -> Request<Body> {
    Request::builder()
        .uri(path)
        .header("host", host)
        .body(Body::empty())
        .unwrap()
}

fn static_fixture() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("cryptool-server-{nonce}"));
    fs::create_dir_all(root.join("assets")).unwrap();
    fs::write(root.join("index.html"), "<main>cryptool ui</main>").unwrap();
    fs::write(root.join("assets/app.js"), "console.log('cryptool')").unwrap();
    root
}
#[tokio::test]
async fn http_and_native_share_the_complete_contract() {
    let (fixture, app) = app().await;
    let native = ExchangeDataService::new(fixture.settings())
        .unwrap()
        .query(TradeQuery::new(Market::Spot, "BTCUSDT", TradeAnchor::Id(1000), 2, 2).unwrap())
        .await
        .unwrap();
    let response=app.oneshot(request("/api/v1/binance/spot/historical-trades?symbol=BTCUSDT&mode=id&trade_id=1000&before=2&after=2","localhost:8080")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let http: TradeWindow = serde_json::from_slice(&body).unwrap();
    assert_eq!(http, native);
    let raw: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(raw["items"][0]["timestamp"]["value"].is_string());
}
#[tokio::test]
async fn invalid_query_forms_are_json_errors_before_upstream_io() {
    let (fixture, app) = app().await;
    for params in [
        "symbol=BTCUSDT&mode=id&trade_id=1&time=1700000000&time_unit=s",
        "symbol=BTCUSDT&mode=time&time=1700000000",
        "symbol=BTCUSDT&mode=id&trade_id=1&before=500&after=500",
        "symbol=BTCUSDT&mode=id&trade_id=-1",
        "symbol=BTCUSDT&mode=id&trade_id=1&unknown=1",
        "symbol=BTCUSDT&mode=id&trade_id=1&trade_id=2",
    ] {
        let response = app
            .clone()
            .oneshot(request(
                &format!("/api/v1/binance/spot/historical-trades?{params}"),
                "127.0.0.1",
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{params}");
        let value: serde_json::Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(value["code"], "INVALID_INPUT");
    }
    assert!(fixture.requests().is_empty());
}
#[tokio::test]
async fn unknown_apis_never_fall_back_to_html() {
    let (_, app) = app().await;
    for path in [
        "/api/v1/binance/spot/rules",
        "/api/v1/account",
        "/api/missing",
    ] {
        let response = app
            .clone()
            .oneshot(request(path, "localhost"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(
            response.headers()["content-type"]
                .to_str()
                .unwrap()
                .starts_with("application/json")
        );
    }
}
#[tokio::test]
async fn local_host_boundary_rejects_rebinding() {
    let (_, app) = app().await;
    assert_eq!(
        app.clone()
            .oneshot(request("/health", "evil.example"))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.clone()
            .oneshot(request("/health", "localhost.evil.example"))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.clone()
            .oneshot(request("/health", "localhost:not-a-port"))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let duplicate_host = Request::builder()
        .uri("/health")
        .header("host", "localhost")
        .header("host", "evil.example")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.clone().oneshot(duplicate_host).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    let missing_host = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.clone().oneshot(missing_host).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.oneshot(request("/health", "[::1]:8080"))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
}

#[tokio::test]
async fn disk_assets_serve_only_known_spa_deep_links() {
    let fixture = Fixture::start(|_, req| history(req, true)).await;
    let root = static_fixture();
    let service = ExchangeDataService::new(fixture.settings()).unwrap();
    let app = router(
        service,
        ServerOptions {
            static_dir: Some(root.clone()),
            ..ServerOptions::default()
        },
    )
    .unwrap();

    for path in [
        "/",
        "/dashboard",
        "/binance/spot-history-trade",
        "/binance/future-history-trade",
        "/tools/time-convert",
    ] {
        let response = app
            .clone()
            .oneshot(request(path, "localhost"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        assert_eq!(response.headers()["x-content-type-options"], "nosniff");
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body.as_ref(), b"<main>cryptool ui</main>");
    }
    let asset = app
        .clone()
        .oneshot(request("/assets/app.js", "localhost"))
        .await
        .unwrap();
    assert_eq!(asset.status(), StatusCode::OK);
    assert!(
        asset.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/javascript")
    );
    assert_eq!(
        app.oneshot(request("/unknown-deep-link", "localhost"))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn api_methods_and_cross_site_origins_are_rejected_as_json() {
    let (_, app) = app().await;
    let post = Request::builder()
        .method("POST")
        .uri("/api/v1/binance/spot/historical-trades?symbol=BTCUSDT&mode=id&trade_id=1")
        .header("host", "localhost")
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(post).await.unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert!(
        response.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("application/json")
    );

    let cross_site = Request::builder()
        .uri("/health")
        .header("host", "localhost:8080")
        .header("origin", "https://evil.example")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.clone().oneshot(cross_site).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    let origin_with_path = Request::builder()
        .uri("/health")
        .header("host", "localhost:8080")
        .header("origin", "http://localhost:8080/path")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.clone()
            .oneshot(origin_with_path)
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let duplicate_origin = Request::builder()
        .uri("/health")
        .header("host", "localhost:8080")
        .header("origin", "http://localhost:8080")
        .header("origin", "https://evil.example")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.clone()
            .oneshot(duplicate_origin)
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let same_site = Request::builder()
        .uri("/health")
        .header("host", "localhost:8080")
        .header("origin", "http://localhost:8080")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.clone().oneshot(same_site).await.unwrap().status(),
        StatusCode::OK
    );

    let body_on_get = Request::builder()
        .uri("/health")
        .header("host", "localhost")
        .header("content-length", "1")
        .body(Body::from("x"))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(body_on_get).await.unwrap().status(),
        StatusCode::BAD_REQUEST
    );
    let unframed_body_on_get = Request::builder()
        .uri("/health")
        .header("host", "localhost")
        .body(Body::from("x"))
        .unwrap();
    assert_eq!(
        app.clone()
            .oneshot(unframed_body_on_get)
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    let streamed_body_on_get = Request::builder()
        .uri("/health")
        .header("host", "localhost")
        .header("transfer-encoding", "chunked")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.clone()
            .oneshot(streamed_body_on_get)
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );

    let oversized_path = format!("/{}", "x".repeat(8 * 1024));
    assert_eq!(
        app.oneshot(request(&oversized_path, "localhost"))
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn head_is_method_not_allowed_without_upstream_io() {
    let (fixture, app) = app().await;
    let query = Request::builder()
        .method("HEAD")
        .uri("/api/v1/binance/spot/historical-trades?symbol=BTCUSDT&mode=id&trade_id=1000&before=0&after=0")
        .header("host", "localhost")
        .body(Body::empty())
        .unwrap();
    let query_status = app.clone().oneshot(query).await.unwrap().status();
    let health = Request::builder()
        .method("HEAD")
        .uri("/health")
        .header("host", "localhost")
        .body(Body::empty())
        .unwrap();
    let health_status = app.oneshot(health).await.unwrap().status();

    assert_eq!(query_status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(health_status, StatusCode::METHOD_NOT_ALLOWED);
    assert!(fixture.requests().is_empty());
}
