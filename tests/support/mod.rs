use cryptool::config::{ProxyMode, ProxySettings, Settings};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
};

#[derive(Clone, Debug)]
pub struct Request {
    pub path: String,
    pub query: BTreeMap<String, String>,
    pub headers: BTreeMap<String, String>,
}
pub struct Response {
    pub status: u16,
    pub body: Value,
    pub headers: Vec<(String, String)>,
    pub delay: Duration,
}
impl Response {
    pub fn ok(body: Value) -> Self {
        Self {
            status: 200,
            body,
            headers: vec![],
            delay: Duration::ZERO,
        }
    }
}
pub struct Fixture {
    pub url: String,
    requests: Arc<Mutex<Vec<Request>>>,
    task: JoinHandle<()>,
}
impl Fixture {
    pub async fn start(
        handler: impl Fn(usize, &Request) -> Response + Send + Sync + 'static,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let record = requests.clone();
        let task = tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let mut bytes = Vec::new();
                let mut buffer = [0; 1024];
                loop {
                    let read = stream.read(&mut buffer).await.unwrap();
                    if read == 0 {
                        break;
                    }
                    bytes.extend_from_slice(&buffer[..read]);
                    if bytes.windows(4).any(|v| v == b"\r\n\r\n") {
                        break;
                    }
                    assert!(bytes.len() < 16 * 1024);
                }
                if bytes.is_empty() {
                    continue;
                }
                let input = String::from_utf8(bytes).unwrap();
                let target = input
                    .lines()
                    .next()
                    .unwrap()
                    .split_whitespace()
                    .nth(1)
                    .unwrap();
                let url = if target.starts_with("http://") || target.starts_with("https://") {
                    target.to_string()
                } else {
                    format!("http://localhost{target}")
                };
                let parsed = reqwest::Url::parse(&url).unwrap();
                let request = Request {
                    path: parsed.path().into(),
                    query: parsed
                        .query_pairs()
                        .map(|(k, v)| (k.into_owned(), v.into_owned()))
                        .collect(),
                    headers: input
                        .lines()
                        .skip(1)
                        .filter_map(|line| line.split_once(':'))
                        .map(|(k, v)| (k.to_ascii_lowercase(), v.trim().to_string()))
                        .collect(),
                };
                let index = {
                    let mut all = record.lock().unwrap();
                    let n = all.len();
                    all.push(request.clone());
                    n
                };
                let response = handler(index, &request);
                tokio::time::sleep(response.delay).await;
                let body = response.body.to_string();
                let extra = response
                    .headers
                    .iter()
                    .map(|(k, v)| format!("{k}: {v}\r\n"))
                    .collect::<String>();
                let output = format!(
                    "HTTP/1.1 {} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{}\r\n{}",
                    response.status,
                    body.len(),
                    extra,
                    body
                );
                let _ = stream.write_all(output.as_bytes()).await;
            }
        });
        Self {
            url,
            requests,
            task,
        }
    }
    pub fn settings(&self) -> Settings {
        let mut settings = Settings::default();
        settings.binance.base_url = self.url.clone();
        settings.binance.usdm_futures_base_url = self.url.clone();
        settings.binance.api_key = Some("fixture-key".into());
        settings.binance.proxy = ProxySettings {
            mode: ProxyMode::Direct,
            ..Default::default()
        };
        settings
    }
    pub fn requests(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}
pub fn history(req: &Request, micros: bool) -> Response {
    assert!(req.path.ends_with("/historicalTrades"));
    assert!(req.headers.contains_key("host"));
    let start = req.query["fromId"].parse::<u64>().unwrap();
    let limit = req.query["limit"].parse::<u64>().unwrap();
    let rows=(start..start+limit).map(|id|json!({
        "id":id,"price":"123.45000000","qty":"0.10000000","quoteQty":"12.34500000",
        "time":if micros {1_700_000_000_123_456_i64 + id as i64-1000}else {1_700_000_000_123_i64 + id as i64-1000},
        "isBuyerMaker":id%2==0,"isBestMatch":true
    })).collect::<Vec<_>>();
    Response::ok(json!(rows))
}
