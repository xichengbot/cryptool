# Task 2 API adapter report

## Result

- Added the Axum router for health and historical-trade queries with canonical `exchange-types` DTOs and stable HTTP error mapping.
- Added strict query-form parsing before service I/O: allowlisted unique fields, exact decimal IDs/counts, explicit time units, bounded URI, and `TradeQuery::new` validation.
- Added Host and Origin enforcement, empty GET-body enforcement, JSON API 404/405 responses, security headers, and structured request traces limited to method/path/status/latency.
- Added disk static serving with startup canonicalization and `index.html` validation, exact SPA deep-link fallbacks, and `ServeDir` asset handling.
- Added `embed-web` asset lookup from `web-dist/public/` without runtime current-directory dependence.
- Added `cryptool serve` with loopback default, optional `--web-dir`, repeated `--allow-host`, non-loopback exposure guard, stderr logging, and Ctrl-C graceful shutdown.

`ServerOptions` has two public fields:

- `allowed_hosts: Vec<String>`; defaults to `localhost`, `127.0.0.1`, and `::1`.
- `static_dir: Option<PathBuf>`; defaults to `None`.

## Verification

- Initial RED: `cargo test -p cryptool --features server --test server_contract` failed because `cryptool::server` did not exist.
- Security RED: added invalid/duplicate Host, duplicate/path-bearing Origin, GET body, and URI-size cases; the old implementation returned 200 before the guard was tightened.
- `CARGO_TARGET_DIR=/Users/xichengbot/rustProjects/cryptool/target cargo test -p cryptool --features server` passed: 6 library, 6 CLI, 16 native query, 2 proxy transport, and 6 server contract tests.
- `CARGO_TARGET_DIR=/Users/xichengbot/rustProjects/cryptool/target cargo clippy -p cryptool --features server --lib --bin cryptool -- -D warnings` passed.
- `CARGO_TARGET_DIR=/Users/xichengbot/rustProjects/cryptool/target cargo run -q -p cryptool --features server -- serve --help` passed.
- Live process check returned `{"status":"healthy"}` from `127.0.0.1:39091/health`, then exited on SIGINT. Starting on `0.0.0.0:39092` without `--allow-host` failed before bind as required.
- `git diff --check` passed for all owned files.

The full test command emits one existing `dead_code` warning for `tests/support/mod.rs::Request.headers`, a shared parent-owned fixture field. The owned library and binary are warning-free under Clippy `-D warnings`.

`embed-web` was not compiled because `web-dist/public/` has not been staged yet; the feature intentionally gives a compile-time missing-folder error until the UI bundle is produced.

## Review follow-up

- GET body enforcement now checks both framing headers and Axum `Body` state. A body is accepted only when it is already end-of-stream and its size hint is exactly zero; unknown or nonzero bodies are rejected without reading them.
- `/health` and the historical-trades endpoint explicitly return 405 for HEAD. The query handler is not invoked, so HEAD cannot cause upstream I/O.
- The unframed nonempty-body test reproduced the original 200 response before the fix and now passes with 400.
- The HEAD test reproduced the original 200 response before the fix and now passes with 405 for both endpoints while asserting that the fixture received zero requests.
- `CARGO_TARGET_DIR=/Users/xichengbot/rustProjects/cryptool/target cargo test -p cryptool --features server --test server_contract` passed all 7 tests.
- `CARGO_TARGET_DIR=/Users/xichengbot/rustProjects/cryptool/target cargo clippy -p cryptool --features server --lib -- -D warnings` passed.
