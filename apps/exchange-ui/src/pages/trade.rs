use dioxus::core::Task;
use dioxus::prelude::*;
use exchange_types::{Market, QueryError};

use crate::{
    client::ClientBootstrap,
    components::TradeTable,
    state::{AnchorMode, CountField, QueryForm, QueryPhase, QueryState, TimeUnitChoice},
};

#[component]
#[allow(non_snake_case)]
pub fn SpotHistoryTrade() -> Element {
    rsx! { TradePage { market: Market::Spot } }
}

#[component]
#[allow(non_snake_case)]
pub fn FutureHistoryTrade() -> Element {
    rsx! { TradePage { market: Market::Usdm } }
}

#[component]
fn TradePage(market: Market) -> Element {
    let mut form = use_signal(QueryForm::default);
    let mut validation_error = use_signal(|| None::<String>);
    let mut query_state = use_signal(QueryState::default);
    let mut task = use_signal(|| None::<Task>);
    let bootstrap = use_context::<ClientBootstrap>();

    use_drop(move || {
        if let Some(running) = task.take() {
            running.cancel();
        }
    });

    let view = form();
    let state = query_state.read();
    let market_name = if market == Market::Spot {
        "现货历史成交"
    } else {
        "USD-M 合约历史成交"
    };
    let market_label = if market == Market::Spot {
        "Spot"
    } else {
        "USD-M"
    };

    rsx! {
        section { class: "page-stack trade-page", "data-testid": "trade-page",
            div { class: "page-heading",
                div {
                    p { class: "eyebrow", "BINANCE / {market_label}" }
                    h1 { "{market_name}" }
                    p { class: "muted", "按成交 ID 或时间锚点读取前后窗口，结果在本页内分页。" }
                }
            }
            form {
                class: "query-form panel",
                "data-testid": "trade-query-form",
                onsubmit: move |event| {
                    event.prevent_default();
                    let query = match form.read().parse(market) {
                        Ok(query) => query,
                        Err(error) => {
                            validation_error.set(Some(readable_query_error(&error)));
                            return;
                        }
                    };
                    let Some(client) = bootstrap.client.clone() else {
                        validation_error.set(Some(format!("查询不可用：{}", bootstrap.status)));
                        return;
                    };
                    validation_error.set(None);
                    if let Some(running) = task.take() {
                        running.cancel();
                    }
                    let generation = query_state.write().begin();
                    let running = spawn(async move {
                        match client.query_trades(query).await {
                            Ok(window) => {
                                query_state.write().succeed(generation, window);
                            }
                            Err(error) => {
                                query_state.write().fail(generation, readable_query_error(&error));
                            }
                        }
                    });
                    task.set(Some(running));
                },
                div { class: "form-grid",
                    label { class: "field symbol-field",
                        span { "交易对" }
                        input {
                            name: "symbol",
                            value: "{view.symbol}",
                            autocomplete: "off",
                            aria_label: "交易对",
                            oninput: move |event| form.write().symbol = event.value(),
                        }
                    }
                    fieldset { class: "field mode-field",
                        legend { "锚点类型" }
                        label { class: "radio-label",
                            input {
                                r#type: "radio",
                                name: "anchor-mode",
                                value: "id",
                                checked: view.anchor_mode == AnchorMode::Id,
                                onchange: move |_| form.write().anchor_mode = AnchorMode::Id,
                            }
                            "成交 ID"
                        }
                        label { class: "radio-label",
                            input {
                                r#type: "radio",
                                name: "anchor-mode",
                                value: "time",
                                checked: view.anchor_mode == AnchorMode::Time,
                                onchange: move |_| form.write().anchor_mode = AnchorMode::Time,
                            }
                            "时间"
                        }
                    }
                    label { class: "field anchor-field",
                        span { if view.anchor_mode == AnchorMode::Id { "成交 ID" } else { "时间锚点" } }
                        input {
                            name: "anchor",
                            value: "{view.anchor}",
                            inputmode: if view.anchor_mode == AnchorMode::Id { "numeric" } else { "text" },
                            placeholder: if view.anchor_mode == AnchorMode::Id { "例如 123456789" } else { "Unix 时间或 2026-09-09 20:00:00" },
                            aria_label: "查询锚点",
                            oninput: move |event| form.write().anchor = event.value(),
                        }
                    }
                    if view.anchor_mode == AnchorMode::Time {
                        label { class: "field unit-field",
                            span { "时间单位" }
                            select {
                                name: "time-unit",
                                value: "{view.time_unit.value()}",
                                aria_label: "时间单位",
                                onchange: move |event| form.write().time_unit = TimeUnitChoice::from_value(&event.value()),
                                option { value: "auto", "自动识别" }
                                option { value: "s", "秒 (s)" }
                                option { value: "ms", "毫秒 (ms)" }
                                option { value: "us", "微秒 (us)" }
                            }
                        }
                    }
                    CountInput { label: "前序数量", value: view.before, field: CountField::Before, form, validation_error }
                    CountInput { label: "后序数量", value: view.after, field: CountField::After, form, validation_error }
                }
                if let Some(error) = validation_error() {
                    p { class: "form-error", role: "alert", "{error}" }
                }
                div { class: "form-actions",
                    button {
                        class: "primary-button",
                        r#type: "submit",
                        "data-testid": "trade-submit",
                        if state.phase() == QueryPhase::Loading { "重新查询" } else { "查询成交" }
                    }
                    span { class: "form-hint", "按 Enter 提交；最多返回 1,000 条" }
                }
            }
            match state.phase() {
                QueryPhase::Idle => rsx! { ResultNotice { kind: "idle", text: "输入锚点后开始查询。" } },
                QueryPhase::Loading => rsx! {
                    ResultNotice { kind: "loading", text: "正在查询交易所数据…" }
                    if let Some(window) = state.window().cloned() {
                        p { class: "stale-label", "以下为上一次成功结果，新查询完成前仅供参考。" }
                        TradeTable { window }
                    }
                },
                QueryPhase::Error => rsx! {
                    ResultNotice { kind: "error", text: state.error().unwrap_or("查询失败") }
                    if let Some(window) = state.window().cloned() {
                        p { class: "stale-label", "查询失败，以下保留上一次成功结果。" }
                        TradeTable { window }
                    }
                },
                QueryPhase::Empty => rsx! { ResultNotice { kind: "empty", text: "查询成功，但该窗口没有可显示的成交。" } },
                QueryPhase::Partial | QueryPhase::Success => rsx! {
                    if let Some(window) = state.window().cloned() {
                        TradeResults { window }
                    }
                },
            }
        }
    }
}

#[component]
fn TradeResults(window: exchange_types::TradeWindow) -> Element {
    let partial = window.meta.partial;
    let boundary = window.meta.boundary.clone();
    rsx! {
        if partial || boundary.is_some() {
            div { class: if partial { "result-notice partial" } else { "result-notice boundary" }, role: "status",
                strong { if partial { "部分结果：" } else { "数据边界：" } }
                span {
                    "实际收到 {window.meta.received} / {window.meta.requested} 条"
                    if let Some(reason) = boundary { "；{reason}" }
                }
            }
        }
        TradeTable { window }
    }
}

#[component]
fn CountInput(
    label: &'static str,
    value: String,
    field: CountField,
    mut form: Signal<QueryForm>,
    mut validation_error: Signal<Option<String>>,
) -> Element {
    rsx! {
        label { class: "field count-field",
            span { "{label}" }
            div { class: "count-control",
                input {
                    r#type: "number",
                    min: "0",
                    max: "999",
                    step: "1",
                    value: "{value}",
                    aria_label: "{label}",
                    oninput: move |event| match field {
                        CountField::Before => form.write().before = event.value(),
                        CountField::After => form.write().after = event.value(),
                    },
                }
                button {
                    r#type: "button",
                    aria_label: format!("{label}加 50"),
                    onclick: move |_| {
                        validation_error.set(form.write().add_fifty(field).err().map(|error| readable_query_error(&error)));
                    },
                    "+50"
                }
            }
        }
    }
}

#[component]
fn ResultNotice(kind: &'static str, text: String) -> Element {
    rsx! {
        div { class: "result-notice {kind}", role: "status", "data-testid": "query-state", "{text}" }
    }
}

fn readable_query_error(error: &QueryError) -> String {
    let raw = &error.message;
    let message = if raw.contains("symbol must contain") {
        "交易对须为 1–40 位 ASCII 字母、数字或下划线".into()
    } else if raw.contains("before + after + 1") {
        "前序数量与后序数量之和再加锚点，不能超过 1,000 条".into()
    } else if raw.starts_with("invalid time")
        || raw.starts_with("numeric timestamp")
        || raw.starts_with("time value")
    {
        "时间输入无效；请使用完整日期时间，或选择单位后输入整数 Unix 时间".into()
    } else {
        raw.to_string()
    };
    if let Some(seconds) = error.retry_after_secs {
        format!("{message}；可在 {seconds} 秒后重试")
    } else {
        message
    }
}
