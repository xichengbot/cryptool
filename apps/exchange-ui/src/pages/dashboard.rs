use dioxus::prelude::*;

use crate::routes::Route;

#[component]
pub fn Welcome() -> Element {
    rsx! {
        section { class: "welcome panel",
            p { class: "eyebrow", "EXCHANGE DATA" }
            h1 { "可靠地检查原始成交数据" }
            p { class: "lead", "从交易 ID 或明确时间锚点查看 Binance 成交窗口；所有金额、数量和 ID 均保持原始精度。" }
            div { class: "hero-actions",
                Link { class: "primary-button", to: Route::SpotHistoryTrade {}, "查询现货成交" }
                Link { class: "secondary-button", to: Route::TimeConvert {}, "转换时间" }
            }
        }
    }
}

#[component]
pub fn Dashboard() -> Element {
    rsx! {
        section { class: "page-stack",
            div { class: "page-heading",
                div {
                    p { class: "eyebrow", "工作台" }
                    h1 { "数据平台入口" }
                    p { class: "muted", "选择市场和锚点，查询前后成交窗口，或在本地转换时间。" }
                }
            }
            div { class: "dashboard-grid",
                DashboardCard { route: Route::SpotHistoryTrade {}, icon: "S", title: "现货历史成交", detail: "查询 Binance Spot 原始成交，包含 bestMatch。" }
                DashboardCard { route: Route::FutureHistoryTrade {}, icon: "F", title: "USD-M 合约成交", detail: "查询 USD-M Futures 历史成交窗口。" }
                DashboardCard { route: Route::TimeConvert {}, icon: "T", title: "时间转换", detail: "离线转换 Unix、UTC 和北京时间。" }
            }
        }
    }
}

#[component]
fn DashboardCard(
    route: Route,
    icon: &'static str,
    title: &'static str,
    detail: &'static str,
) -> Element {
    rsx! {
        Link { class: "dashboard-card panel", to: route,
            span { class: "card-icon", "{icon}" }
            h2 { "{title}" }
            p { "{detail}" }
            span { class: "card-link", "进入 →" }
        }
    }
}
