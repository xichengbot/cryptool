use dioxus::prelude::*;

use crate::state::{TimeHistory, TimeUnitChoice};

const COMPONENTS_CSS: Asset = asset!("/assets/components.css");

#[component]
pub fn TimeConvertPage() -> Element {
    let mut history = use_context::<Signal<TimeHistory>>();
    let mut input = use_signal(String::new);
    let mut unit = use_signal(|| TimeUnitChoice::Auto);
    let mut error = use_signal(|| None::<String>);

    rsx! {
        document::Stylesheet { href: COMPONENTS_CSS }

        section {
            class: "xc-page-stack",
            aria_labelledby: "time-convert-title",
            header { class: "xc-page-heading",
                div {
                    h1 { id: "time-convert-title", "时间转换" }
                    p { "在 Unix 时间戳、UTC 与北京时间之间转换。结果保留输入精度。" }
                }
            }

            form {
                class: "xc-panel xc-time-form",
                onsubmit: move |event| {
                    event.prevent_default();
                    let raw_input = input();
                    match history.write().convert_and_push(&raw_input, unit()) {
                        Ok(_) => error.set(None),
                        Err(cause) => error.set(Some(readable_time_error(&cause.message))),
                    }
                },
                div { class: "xc-field xc-time-value-field",
                    label { r#for: "time-value", "时间或时间戳" }
                    input {
                        id: "time-value",
                        name: "time-value",
                        class: "xc-text-input xc-mono",
                        r#type: "text",
                        value: "{input}",
                        placeholder: "例如 1725897600123456",
                        autocomplete: "off",
                        spellcheck: "false",
                        oninput: move |event| {
                            input.set(event.value());
                            error.set(None);
                        },
                    }
                }
                fieldset { class: "xc-unit-choices",
                    legend { "数字单位" }
                    for (choice, value, label) in [
                        (TimeUnitChoice::Auto, "auto", "自动"),
                        (TimeUnitChoice::Seconds, "s", "秒 (s)"),
                        (TimeUnitChoice::Millis, "ms", "毫秒 (ms)"),
                        (TimeUnitChoice::Micros, "us", "微秒 (us)"),
                    ] {
                        label { class: "xc-radio-label",
                            input {
                                r#type: "radio",
                                name: "time-unit",
                                value,
                                checked: unit() == choice,
                                onchange: move |_| {
                                    unit.set(choice);
                                    error.set(None);
                                },
                            }
                            span { "{label}" }
                        }
                    }
                }
                button { class: "xc-primary-button", r#type: "submit", "转换" }
            }

            if let Some(message) = error() {
                div { class: "xc-alert xc-alert-error", role: "alert",
                    strong { "无法转换：" }
                    span { "{message}" }
                }
            }

            aside { class: "xc-panel xc-examples", aria_labelledby: "time-example-title",
                h2 { id: "time-example-title", "示例输入" }
                ul {
                    li { code { "1725897600" } " — 自动识别为秒" }
                    li { code { "1725897600123" } " — 自动识别为毫秒" }
                    li { code { "1725897600123456" } " — 自动识别为微秒" }
                    li { code { "2024-09-10T00:00:00.123456Z" } " — RFC 3339（UTC）" }
                    li { code { "2024-09-10 08:00:00.123456" } " — 北京时间" }
                }
            }

            section { class: "xc-panel", aria_labelledby: "time-history-title",
                div { class: "xc-section-heading",
                    h2 { id: "time-history-title", "本次会话记录" }
                    span { class: "xc-count-label", "共 {history.read().entries().len()} 条" }
                }
                if history.read().entries().is_empty() {
                    p { class: "xc-empty-state", "输入时间后，转换结果会按最新优先显示在这里。" }
                } else {
                    div { class: "xc-result-list",
                        for (index, entry) in history.read().entries().iter().enumerate() {
                            article {
                                class: "xc-time-result",
                                key: "{entry.timestamp.value}-{entry.timestamp.unit}-{index}",
                                dl {
                                    div {
                                        dt { "输入" }
                                        dd { class: "xc-mono", "{entry.input}" }
                                    }
                                    div {
                                        dt { "时间戳" }
                                        dd { class: "xc-mono", "{entry.timestamp.value} {entry.timestamp.unit}" }
                                    }
                                    div {
                                        dt { "UTC" }
                                        dd { class: "xc-mono", "{entry.utc}" }
                                    }
                                    div {
                                        dt { "北京时间" }
                                        dd { class: "xc-mono", "{entry.beijing}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn readable_time_error(message: &str) -> String {
    if message.contains("must not be empty") {
        "请输入时间或时间戳".into()
    } else if message.contains("ambiguous; use 10, 13, or 16 digits") {
        "自动识别要求 10 位秒、13 位毫秒或 16 位微秒时间戳；也可以手动选择单位".into()
    } else if message.contains("invalid numeric timestamp") {
        "选择明确单位时只能输入整数时间戳".into()
    } else if message.contains("fractional seconds must not exceed 6 digits") {
        "小数秒最多保留 6 位（微秒）".into()
    } else if message.contains("cannot be represented in microseconds") {
        "时间戳超出可转换范围".into()
    } else if message.contains("outside the supported date range") {
        "时间超出支持的日期范围".into()
    } else {
        "格式无效；请输入 Unix 时间戳、RFC 3339，或 YYYY-MM-DD HH:MM:SS[.微秒] 北京时间".into()
    }
}
