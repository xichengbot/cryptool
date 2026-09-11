use dioxus::prelude::*;
use exchange_types::{Market, TradeWindow, format_beijing};

const COMPONENTS_CSS: Asset = asset!("/assets/components.css");
const DEFAULT_PAGE_SIZE: usize = 100;

#[component]
pub fn TradeTable(window: TradeWindow) -> Element {
    let mut page = use_signal(|| 0_usize);
    let mut page_size = use_signal(|| DEFAULT_PAGE_SIZE);
    let window_dependency = window.clone();

    use_effect(use_reactive((&window_dependency,), move |_| {
        page.set(0);
    }));

    let total = window.items.len();
    let current_size = page_size();
    let page_count = if total == 0 {
        0
    } else {
        total.div_ceil(current_size)
    };
    let current_page = if page_count == 0 {
        0
    } else {
        page().min(page_count - 1)
    };
    let start = current_page.saturating_mul(current_size).min(total);
    let end = (start + current_size).min(total);
    let show_best_match = window.meta.query.market == Market::Spot;
    let anchor_index = window
        .meta
        .anchor_id
        .as_deref()
        .and_then(|anchor_id| window.items.iter().position(|row| row.id == anchor_id));
    let anchor_page = anchor_index.map(|index| index / current_size);
    let anchor_is_visible = anchor_index.is_some_and(|index| (start..end).contains(&index));

    rsx! {
        document::Stylesheet { href: COMPONENTS_CSS }

        section { class: "xc-panel xc-trade-table-panel", aria_labelledby: "trade-table-title",
            div { class: "xc-section-heading xc-table-summary",
                div {
                    h2 { id: "trade-table-title", "成交明细" }
                    p {
                        "已加载 {total} 条；锚点前 {window.meta.actual_before} 条，锚点后 {window.meta.actual_after} 条；请求 {window.meta.requested} 条"
                        if let Some(anchor_id) = window.meta.anchor_id.as_deref() {
                            "；锚点 ID "
                            span { class: "xc-mono", "{anchor_id}" }
                            if anchor_index.is_none() {
                                "（不在已加载记录中）"
                            }
                        }
                    }
                }
                if let Some(target_page) = anchor_page
                    && !anchor_is_visible
                {
                    button {
                        class: "xc-secondary-button",
                        r#type: "button",
                        onclick: move |_| page.set(target_page),
                        "跳到锚点（第 {target_page + 1} 页）"
                    }
                }
            }

            div { class: "xc-table-scroll", tabindex: "0", aria_label: "成交明细，可水平滚动",
                table { class: "xc-trade-table",
                    thead {
                        tr {
                            th { scope: "col", div { class: "xc-resizable-column xc-col-time", "北京时间" } }
                            th { scope: "col", div { class: "xc-resizable-column xc-col-raw-time", "原始时间戳（单位）" } }
                            th { scope: "col", div { class: "xc-resizable-column xc-col-id", "成交 ID" } }
                            th { scope: "col", div { class: "xc-resizable-column xc-col-number", "价格" } }
                            th { scope: "col", div { class: "xc-resizable-column xc-col-number", "数量" } }
                            th { scope: "col", div { class: "xc-resizable-column xc-col-number", "报价数量" } }
                            th { scope: "col", div { class: "xc-resizable-column xc-col-flag", "买方挂单" } }
                            if show_best_match {
                                th { scope: "col", div { class: "xc-resizable-column xc-col-flag", "最佳匹配" } }
                            }
                        }
                    }
                    tbody {
                        for record in &window.items[start..end] {
                            {
                                let is_anchor = window.meta.anchor_id.as_deref() == Some(record.id.as_str());
                                let beijing = format_beijing(record.timestamp)
                                    .unwrap_or_else(|_| "时间无效".to_string());
                                rsx! {
                                    tr {
                                        key: "{record.id}",
                                        class: if is_anchor { "xc-anchor-row" } else { "" },
                                        aria_current: if is_anchor { "true" },
                                        td { class: "xc-mono xc-nowrap", "{beijing}" }
                                        td { class: "xc-mono xc-nowrap", "{record.timestamp.value} {record.timestamp.unit}" }
                                        td { class: "xc-mono xc-nowrap", "{record.id}" }
                                        td { class: "xc-mono xc-number-cell", "{record.price}" }
                                        td { class: "xc-mono xc-number-cell", "{record.qty}" }
                                        td { class: "xc-mono xc-number-cell", "{record.quote_qty}" }
                                        td {
                                            span {
                                                class: if record.maker { "xc-flag xc-flag-yes" } else { "xc-flag xc-flag-no" },
                                                if record.maker { "是" } else { "否" }
                                            }
                                        }
                                        if show_best_match {
                                            td {
                                                match record.best_match {
                                                    Some(true) => rsx! { span { class: "xc-flag xc-flag-neutral", "是" } },
                                                    Some(false) => rsx! { span { class: "xc-flag xc-flag-muted", "否" } },
                                                    None => rsx! { span { class: "xc-flag xc-flag-muted", "—" } },
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

            div { class: "xc-pagination",
                label { r#for: "trade-page-size", "每页" }
                select {
                    id: "trade-page-size",
                    value: "{current_size}",
                    onchange: move |event| {
                        let next = match event.value().as_str() {
                            "1000" => 1_000,
                            _ => DEFAULT_PAGE_SIZE,
                        };
                        page_size.set(next);
                        page.set(0);
                    },
                    option { value: "100", "100 条" }
                    option { value: "1000", "全部（最多 1000 条）" }
                }
                span { class: "xc-page-indicator",
                    if page_count == 0 {
                        "第 0 / 0 页 · 0 条"
                    } else {
                        "第 {current_page + 1} / {page_count} 页 · {total} 条"
                    }
                }
                button {
                    class: "xc-secondary-button",
                    r#type: "button",
                    disabled: current_page == 0,
                    onclick: move |_| page.set(current_page.saturating_sub(1)),
                    "上一页"
                }
                button {
                    class: "xc-secondary-button",
                    r#type: "button",
                    disabled: page_count == 0 || current_page + 1 >= page_count,
                    onclick: move |_| page.set(current_page + 1),
                    "下一页"
                }
            }
        }
    }
}
