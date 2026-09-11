use cryptool::config::Settings;
use serde_json::{Value, json};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
const SENSITIVE_ENV: &[&str] = &[
    "BINANCE_API_KEY",
    "BINANCE_PRIVATE_KEY",
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "ALL_PROXY",
    "NO_PROXY",
    "http_proxy",
    "https_proxy",
    "all_proxy",
    "no_proxy",
];

struct TempConfig(PathBuf);

impl TempConfig {
    fn new() -> Self {
        let sequence = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "cryptool-cli-regression-{}-{sequence}.toml",
            std::process::id()
        ));
        fs::write(
            &path,
            r#"
[binance]
api_key = "dummy-test-key"
private_key = "dummy-test-private-key"
base_url = "http://127.0.0.1:9"
usdm_futures_base_url = "http://127.0.0.1:12345/custom-futures"
recv_window = 5000
timeout_ms = 1000

[output]
format = "json"
"#,
        )
        .unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempConfig {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn clean_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut command = Command::new(program);
    for key in SENSITIVE_ENV {
        command.env_remove(key);
    }
    command
}

fn run_cli(args: &[&str]) -> Output {
    clean_command(env!("CARGO_BIN_EXE_cryptool"))
        .args(args)
        .output()
        .unwrap()
}

fn exchange_info_fixture() -> Value {
    json!({
        "symbols": [{
            "symbol": "BTCUSDT",
            "status": "TRADING",
            "baseAsset": "BTC",
            "quoteAsset": "USDT",
            "baseAssetPrecision": 8,
            "quotePrecision": 8,
            "filters": [
                {"filterType": "PRICE_FILTER", "tickSize": "0.01000000"},
                {"filterType": "LOT_SIZE", "minQty": "0.00001000", "stepSize": "0.00001000"},
                {"filterType": "MIN_NOTIONAL", "minNotional": "5.00000000"}
            ]
        }]
    })
}

fn serve_once(body: Value) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .unwrap();
                    let mut request = Vec::new();
                    let mut buffer = [0_u8; 1024];
                    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                        let read = stream.read(&mut buffer).unwrap();
                        if read == 0 {
                            break;
                        }
                        request.extend_from_slice(&buffer[..read]);
                    }
                    let request_line = String::from_utf8(request)
                        .unwrap()
                        .lines()
                        .next()
                        .unwrap()
                        .to_string();
                    let body = serde_json::to_vec(&body).unwrap();
                    write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    )
                    .unwrap();
                    stream.write_all(&body).unwrap();
                    return request_line;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        panic!("CLI did not connect to local fixture within five seconds");
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("failed to accept fixture connection: {error}"),
            }
        }
    });
    (format!("http://{address}"), handle)
}

#[test]
fn json_output_is_clean_and_future_flag_keeps_spot_route() {
    let (base_url, server) = serve_once(exchange_info_fixture());
    let output = run_cli(&[
        "--future",
        "--base-url",
        &base_url,
        "--output",
        "json",
        "market",
        "symbol",
        "BTCUSDT",
    ]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let actual: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        actual,
        json!({
            "symbol": "BTCUSDT",
            "status": "Trading",
            "base_asset": "BTC",
            "quote_asset": "USDT",
            "price_precision": 8,
            "quantity_precision": 8,
            "min_quantity": "0.00001000",
            "min_notional": "5.00000000",
            "tick_size": "0.01000000",
            "step_size": "0.00001000"
        })
    );
    assert_eq!(
        server.join().unwrap(),
        "GET /api/v3/exchangeInfo?symbol=BTCUSDT HTTP/1.1"
    );
}

#[test]
fn account_balance_remains_explicitly_not_implemented() {
    let output = run_cli(&["account", "balance"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("not implemented: account balance"));
}

#[test]
fn cli_preserves_the_500_trade_limit() {
    let accepted = run_cli(&[
        "market",
        "trades",
        "BTCUSDT",
        "--trade-id",
        "42",
        "--before",
        "499",
        "--after",
        "0",
    ]);
    let accepted_stderr = String::from_utf8_lossy(&accepted.stderr);
    assert!(!accepted.status.success());
    assert!(accepted_stderr.contains("BINANCE_API_KEY is required"));

    let rejected = run_cli(&[
        "market",
        "trades",
        "BTCUSDT",
        "--trade-id",
        "42",
        "--before",
        "500",
        "--after",
        "0",
    ]);
    assert!(!rejected.status.success());
    assert!(
        String::from_utf8_lossy(&rejected.stderr).contains("before + after + 1 must be in 1..=500")
    );
}

#[test]
fn cli_requires_exactly_one_trade_anchor() {
    let neither = run_cli(&["market", "trades", "BTCUSDT"]);
    assert!(!neither.status.success());

    let both = run_cli(&[
        "market",
        "trades",
        "BTCUSDT",
        "--trade-id",
        "42",
        "--time",
        "2026-07-26T12:34:56Z",
    ]);
    assert!(!both.status.success());
    assert!(String::from_utf8_lossy(&both.stderr).contains("cannot be used with"));
}

#[test]
fn settings_load_helper() {
    let Some(path) = std::env::var_os("CRYPTOOL_TEST_CONFIG_HELPER") else {
        return;
    };
    let settings = Settings::load(Some(Path::new(&path)), None).unwrap();
    assert_eq!(
        settings.binance.usdm_futures_base_url,
        "http://127.0.0.1:12345/custom-futures"
    );
}

#[test]
fn settings_load_preserves_configured_futures_url_without_real_credentials() {
    let config = TempConfig::new();
    let output = clean_command(std::env::current_exe().unwrap())
        .args(["--exact", "settings_load_helper", "--nocapture"])
        .env("CRYPTOOL_TEST_CONFIG_HELPER", config.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "child stdout: {}\nchild stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
