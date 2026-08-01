# cryptool

`cryptool` 是一个基于 Rust 实现的 Binance 命令行量化运维工具，通过 CLI 提供交易规则查询、账户资产查询等能力。

## 核心模块

```text
market      交易规则查询
account     账户资产查询
config      配置管理
```

```
 cargo run -- --output json market symbol BTCUSDT
 cargo run -- --output json market all
 
cargo run -- --config config.example.toml --output json market all --status trading
cargo run -- --config config.example.toml --output json market all --status halt
cargo run -- --config config.example.toml --output json market all --status break
cargo run -- --config config.example.toml --output json market all --status all
```

## 合约交易规范查询命令
```
cargo run -- --output json futures-market all --status trading
cargo run -- --output json futures-market symbol BTCUSDT
```

## 逐笔查询
```
cargo run -- --config ./config.toml market trades BTCUSDT --trade-id 6533409332 --before 20 --after 20
cargo run -- --config ./config.toml futures-market trades BTCUSDT --time 2026-07-26T12:34:56Z --before 20 --after 20
```


