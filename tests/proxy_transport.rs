mod support;
use cryptool::{
    app::exchange_data::ExchangeDataService,
    config::{ProxyMode, ProxySettings, Settings},
};
use exchange_types::{Market, TradeAnchor, TradeQuery};
use support::{Fixture, history};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

fn query() -> TradeQuery {
    TradeQuery::new(Market::Spot, "BTCUSDT", TradeAnchor::Id(1000), 0, 0).unwrap()
}

#[tokio::test]
async fn http_proxy_authentication_is_sent_only_to_the_proxy() {
    let proxy = Fixture::start(|_, request| history(request, true)).await;
    let mut settings = proxy.settings();
    settings.binance.base_url = "http://exchange.test".into();
    settings.binance.api_key = None;
    settings.binance.proxy = ProxySettings {
        mode: ProxyMode::Http,
        url: Some(proxy.url.clone()),
        username: Some("fixture-user".into()),
        password: Some("fixture-pass".into()),
    };
    let window = ExchangeDataService::new(settings)
        .unwrap()
        .query(query())
        .await
        .unwrap();
    assert_eq!(window.items[0].id, "1000");
    let requests = proxy.requests();
    assert_eq!(requests[0].headers["host"], "exchange.test");
    assert_eq!(
        requests[0].headers["proxy-authorization"],
        "Basic Zml4dHVyZS11c2VyOmZpeHR1cmUtcGFzcw=="
    );
    assert!(!requests[0].headers.contains_key("authorization"));
    assert!(!requests[0].headers.contains_key("x-mbx-apikey"));
}

#[tokio::test]
async fn socks5h_uses_proxy_dns_and_username_password_authentication() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let proxy = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        assert_eq!(stream.read_u8().await.unwrap(), 5);
        let count = stream.read_u8().await.unwrap();
        let mut methods = vec![0; usize::from(count)];
        stream.read_exact(&mut methods).await.unwrap();
        assert!(methods.contains(&2));
        stream.write_all(&[5, 2]).await.unwrap();
        assert_eq!(stream.read_u8().await.unwrap(), 1);
        let len = stream.read_u8().await.unwrap();
        let mut user = vec![0; usize::from(len)];
        stream.read_exact(&mut user).await.unwrap();
        let len = stream.read_u8().await.unwrap();
        let mut pass = vec![0; usize::from(len)];
        stream.read_exact(&mut pass).await.unwrap();
        assert_eq!(user, b"fixture-user");
        assert_eq!(pass, b"fixture-pass");
        stream.write_all(&[1, 0]).await.unwrap();
        let mut header = [0; 4];
        stream.read_exact(&mut header).await.unwrap();
        assert_eq!(header, [5, 1, 0, 3]);
        let len = stream.read_u8().await.unwrap();
        let mut domain = vec![0; usize::from(len)];
        stream.read_exact(&mut domain).await.unwrap();
        assert_eq!(domain, b"exchange.test");
        assert_eq!(stream.read_u16().await.unwrap(), 80);
        stream
            .write_all(&[5, 0, 0, 1, 127, 0, 0, 1, 0, 80])
            .await
            .unwrap();
        let mut request = Vec::new();
        let mut byte = [0];
        while !request.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).await.unwrap();
            request.push(byte[0]);
            assert!(request.len() < 8192);
        }
        let request = String::from_utf8(request).unwrap().to_ascii_lowercase();
        assert!(!request.contains("fixture-pass"));
        assert!(!request.contains("proxy-authorization"));
        let body = r#"[{"id":1000,"price":"1","qty":"1","quoteQty":"1","time":1700000000123456,"isBuyerMaker":true,"isBestMatch":true}]"#;
        stream
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    });
    let mut settings = Settings::default();
    settings.binance.base_url = "http://exchange.test".into();
    settings.binance.proxy = ProxySettings {
        mode: ProxyMode::Socks5,
        url: Some(format!("socks5h://{addr}")),
        username: Some("fixture-user".into()),
        password: Some("fixture-pass".into()),
    };
    settings.exchange_data.total_timeout_ms = 2000;
    let window = ExchangeDataService::new(settings)
        .unwrap()
        .query(query())
        .await
        .unwrap();
    assert_eq!(window.items[0].id, "1000");
    proxy.await.unwrap();
}
