use exchange_types::{
    ErrorCode, Market, QueryError, TimeUnit, Timestamp, TradeAnchor, TradeQuery, TradeRow,
    TradeWindow, TradeWindowMeta, format_beijing, format_utc, parse_time,
};

fn assert_invalid_input(error: QueryError) {
    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert!(!error.retryable);
    assert_eq!(error.retry_after_secs, None);
}

#[test]
fn market_and_time_units_use_stable_wire_values() {
    assert_eq!(serde_json::to_string(&Market::Spot).unwrap(), r#""spot""#);
    assert_eq!(serde_json::to_string(&Market::Usdm).unwrap(), r#""usdm""#);
    assert_eq!("spot".parse::<Market>().unwrap(), Market::Spot);
    assert_eq!(Market::Usdm.to_string(), "usdm");
    assert!("futures".parse::<Market>().is_err());

    assert_eq!(serde_json::to_string(&TimeUnit::Seconds).unwrap(), r#""s""#);
    assert_eq!(serde_json::to_string(&TimeUnit::Millis).unwrap(), r#""ms""#);
    assert_eq!(serde_json::to_string(&TimeUnit::Micros).unwrap(), r#""us""#);
    assert_eq!("ms".parse::<TimeUnit>().unwrap(), TimeUnit::Millis);
    assert_eq!(TimeUnit::Micros.to_string(), "us");
    assert!("ns".parse::<TimeUnit>().is_err());
}

#[test]
fn large_ids_and_timestamps_roundtrip_as_json_strings() {
    let anchor = TradeAnchor::Id(9_007_199_254_740_993);
    let anchor_json = serde_json::to_value(&anchor).unwrap();
    assert_eq!(anchor_json["kind"], "Id");
    assert_eq!(anchor_json["value"], "9007199254740993");
    assert_eq!(
        serde_json::from_value::<TradeAnchor>(anchor_json).unwrap(),
        anchor
    );

    let timestamp = Timestamp {
        value: 1_700_000_000_123_456,
        unit: TimeUnit::Micros,
    };
    let timestamp_json = serde_json::to_value(timestamp).unwrap();
    assert_eq!(timestamp_json["value"], "1700000000123456");
    assert_eq!(timestamp_json["unit"], "us");
    assert_eq!(
        serde_json::from_value::<Timestamp>(timestamp_json).unwrap(),
        timestamp
    );
}

#[test]
fn numeric_time_parsing_preserves_units_and_checks_scaling() {
    assert_eq!(
        parse_time("1700000000", None).unwrap(),
        Timestamp {
            value: 1_700_000_000,
            unit: TimeUnit::Seconds,
        }
    );
    assert_eq!(
        parse_time("1700000000123", None).unwrap(),
        Timestamp {
            value: 1_700_000_000_123,
            unit: TimeUnit::Millis,
        }
    );
    assert_eq!(
        parse_time("1700000000123456", None).unwrap(),
        Timestamp {
            value: 1_700_000_000_123_456,
            unit: TimeUnit::Micros,
        }
    );
    assert_eq!(
        parse_time("-1", Some(TimeUnit::Seconds))
            .unwrap()
            .to_micros()
            .unwrap(),
        -1_000_000
    );

    assert_invalid_input(parse_time("17000000000", None).unwrap_err());
    assert_invalid_input(parse_time("", Some(TimeUnit::Millis)).unwrap_err());
    assert_invalid_input(parse_time("not-a-time", Some(TimeUnit::Micros)).unwrap_err());
    assert_invalid_input(parse_time(&i64::MAX.to_string(), Some(TimeUnit::Seconds)).unwrap_err());
}

#[test]
fn date_parsing_preserves_microseconds_and_offsets() {
    let utc = parse_time("2023-11-14T22:13:20.123456Z", None).unwrap();
    let offset = parse_time("2023-11-15T06:13:20.123456+08:00", None).unwrap();
    let shanghai = parse_time("2023-11-15 06:13:20.123456", None).unwrap();

    assert_eq!(utc, offset);
    assert_eq!(utc, shanghai);
    assert_eq!(utc.unit, TimeUnit::Micros);
    assert_eq!(format_utc(utc).unwrap(), "2023-11-14 22:13:20.123456Z");
    assert_eq!(
        format_beijing(utc).unwrap(),
        "2023-11-15 06:13:20.123456+08:00"
    );

    assert_invalid_input(parse_time("2023-11-14T22:13:20.1234567Z", None).unwrap_err());
    assert_invalid_input(parse_time("2023-11-15 06:13:20.1234567", None).unwrap_err());
}

#[test]
fn historical_beijing_format_preserves_offset_seconds() {
    let utc = parse_time("1900-01-01T00:00:00Z", None).unwrap();
    let shanghai = parse_time("1900-01-01 08:05:43", None).unwrap();

    assert_eq!(utc, shanghai);
    assert_eq!(
        format_beijing(utc).unwrap(),
        "1900-01-01 08:05:43.000000+08:05:43"
    );
}

#[test]
fn formatting_reflects_source_unit_and_rejects_out_of_range_values() {
    assert_eq!(
        format_utc(Timestamp {
            value: 1_700_000_000,
            unit: TimeUnit::Seconds,
        })
        .unwrap(),
        "2023-11-14 22:13:20Z"
    );
    assert_eq!(
        format_utc(Timestamp {
            value: 1_700_000_000_123,
            unit: TimeUnit::Millis,
        })
        .unwrap(),
        "2023-11-14 22:13:20.123Z"
    );
    assert_eq!(
        format_utc(Timestamp {
            value: -1,
            unit: TimeUnit::Micros,
        })
        .unwrap(),
        "1969-12-31 23:59:59.999999Z"
    );
    assert_invalid_input(
        format_utc(Timestamp {
            value: i64::MAX,
            unit: TimeUnit::Seconds,
        })
        .unwrap_err(),
    );
}

#[test]
fn trade_queries_normalize_symbols_and_enforce_contract_limits() {
    let accepted = TradeQuery::new(
        Market::Spot,
        "  btc_usdt  ",
        TradeAnchor::Id(u64::MAX),
        999,
        0,
    )
    .unwrap();
    assert_eq!(accepted.symbol, "BTC_USDT");
    assert_eq!(accepted.limit().unwrap(), 1000);

    assert_invalid_input(
        TradeQuery::new(Market::Spot, "BTCUSDT", TradeAnchor::Id(1), 999, 1).unwrap_err(),
    );
    assert_invalid_input(
        TradeQuery::new(
            Market::Usdm,
            "BTCUSDT",
            TradeAnchor::Time(Timestamp {
                value: -1,
                unit: TimeUnit::Millis,
            }),
            1,
            1,
        )
        .unwrap_err(),
    );
}

#[test]
fn trade_queries_accept_unicode_symbols() {
    let query = TradeQuery::new(Market::Spot, "  牛来usdt  ", TradeAnchor::Id(1), 0, 0).unwrap();

    assert_eq!(query.symbol, "牛来USDT");
}

#[test]
fn validation_rejects_invalid_raw_dtos_and_symbols() {
    let invalid_window = TradeQuery {
        market: Market::Spot,
        symbol: "BTCUSDT".to_string(),
        anchor: TradeAnchor::Id(1),
        before: u16::MAX,
        after: u16::MAX,
    };
    assert_invalid_input(invalid_window.validate().unwrap_err());

    for symbol in [
        "",
        "   ",
        "BTC-USDT",
        "A2345678901234567890123456789012345678901",
    ] {
        let error = TradeQuery::new(Market::Spot, symbol, TradeAnchor::Id(1), 0, 0).unwrap_err();
        assert_invalid_input(error);
    }
}

#[test]
fn dto_and_error_wire_shapes_are_stable() {
    let query = TradeQuery::new(Market::Spot, "BTCUSDT", TradeAnchor::Id(42), 1, 2).unwrap();
    let window = TradeWindow {
        items: vec![TradeRow {
            id: "42".to_string(),
            price: "65000.00".to_string(),
            qty: "0.001".to_string(),
            quote_qty: "65.00".to_string(),
            timestamp: Timestamp {
                value: 1_700_000_000_123,
                unit: TimeUnit::Millis,
            },
            maker: true,
            best_match: Some(false),
        }],
        meta: TradeWindowMeta {
            query,
            anchor_id: Some("42".to_string()),
            requested: 4,
            received: 1,
            actual_before: 0,
            actual_after: 0,
            boundary: Some("start".to_string()),
            upstream_calls: 1,
            partial: true,
        },
    };
    let json = serde_json::to_string(&window).unwrap();
    assert_eq!(serde_json::from_str::<TradeWindow>(&json).unwrap(), window);

    let error = QueryError::new(ErrorCode::RateLimited, "slow down");
    assert_eq!(error.to_string(), "slow down");
    assert!(error.retryable);
    assert_eq!(error.retry_after_secs, None);
    assert_eq!(
        serde_json::to_value(&error).unwrap()["code"],
        "RATE_LIMITED"
    );

    for code in [
        ErrorCode::InvalidInput,
        ErrorCode::SymbolNotFound,
        ErrorCode::CredentialsNotConfigured,
        ErrorCode::Cancelled,
        ErrorCode::Configuration,
    ] {
        assert!(!QueryError::new(code, "message").retryable);
    }
    for code in [
        ErrorCode::RateLimited,
        ErrorCode::UpstreamError,
        ErrorCode::QueryBudgetExceeded,
    ] {
        assert!(QueryError::new(code, "message").retryable);
    }
}
