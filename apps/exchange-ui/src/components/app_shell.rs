use dioxus::prelude::*;

use crate::{
    client::ClientBootstrap,
    platform::{self, Preferences},
    routes::Route,
};

#[component]
#[allow(non_snake_case)]
pub fn AppShell() -> Element {
    let current = use_route::<Route>();
    let mut tabs = use_signal(|| vec![Route::Dashboard {}]);
    let mut menu_open = use_signal(|| false);
    let mut search = use_signal(String::new);
    let mut preferences = use_context::<Signal<Preferences>>();
    let mut preference_error = use_signal(|| None::<String>);
    let fullscreen = platform::use_fullscreen_toggle();
    let bootstrap = use_context::<ClientBootstrap>();

    let current_dependency = current.clone();
    use_effect(use_reactive((&current_dependency,), move |(route,)| {
        if route.tab_allowed() && !tabs.read().contains(&route) {
            tabs.write().push(route);
        }
    }));

    let query = search().trim().to_lowercase();
    let menu_items = [
        ("概览", "首页", Route::Dashboard {}),
        ("Binance", "现货历史成交", Route::SpotHistoryTrade {}),
        (
            "Binance",
            "USD-M 合约历史成交",
            Route::FutureHistoryTrade {},
        ),
        ("工具", "时间转换", Route::TimeConvert {}),
    ];

    rsx! {
        div { class: if preferences().dark_mode { "app dark" } else { "app" },
            aside { class: if menu_open() { "sidebar open" } else { "sidebar" },
                div { class: "brand",
                    span { class: "brand-mark", span { class: "brand-glyph", "鲸" } }
                    div { strong { "Exchange Data" } small { "交易所数据平台" } }
                }
                label { class: "menu-search",
                    span { class: "sr-only", "搜索菜单" }
                    input {
                        r#type: "search",
                        value: "{search}",
                        placeholder: "搜索菜单…",
                        aria_label: "搜索菜单",
                        oninput: move |event| search.set(event.value()),
                    }
                }
                nav { aria_label: "主菜单",
                    for (group, label, route) in menu_items {
                        if query.is_empty() || label.to_lowercase().contains(&query) || group.to_lowercase().contains(&query) {
                            div { class: "nav-entry",
                                small { "{group}" }
                                Link {
                                    class: if current == route { "nav-link active" } else { "nav-link" },
                                    to: route.clone(),
                                    onclick: move |_| {
                                        open_tab(tabs, route.clone());
                                        menu_open.set(false);
                                    },
                                    "{label}"
                                }
                            }
                        }
                    }
                }
                div { class: "sidebar-status", title: "查询配置状态", "{bootstrap.status}" }
            }
            div { class: "workspace",
                header { class: "topbar",
                    button {
                        class: "icon-button mobile-menu",
                        aria_label: "切换侧栏",
                        onclick: move |_| menu_open.toggle(),
                        "☰"
                    }
                    p { class: "breadcrumb", "{current.breadcrumb()}" }
                    div { class: "topbar-actions",
                        button {
                            class: "icon-button",
                            aria_label: if preferences().dark_mode { "切换浅色主题" } else { "切换深色主题" },
                            title: "切换主题",
                            onclick: move |_| {
                                let next = Preferences { dark_mode: !preferences().dark_mode };
                                preferences.set(next);
                                preference_error.set(platform::save_preferences(next).err());
                            },
                            if preferences().dark_mode { "☀" } else { "☾" }
                        }
                        button {
                            class: "icon-button",
                            aria_label: "切换全屏",
                            title: "切换全屏",
                            onclick: move |_| fullscreen.call(()),
                            "⛶"
                        }
                    }
                }
                nav { class: "page-tabs", aria_label: "已打开页面",
                    for route in tabs.read().iter().cloned() {
                        PageTab { key: "{route}", route, current: current.clone(), tabs }
                    }
                }
                if let Some(error) = preference_error() {
                    div { class: "inline-notice warning", role: "status", "{error}" }
                }
                main { class: "content", Outlet::<Route> {} }
                footer { class: "statusbar", "{bootstrap.status}" }
            }
        }
    }
}

#[component]
fn PageTab(route: Route, current: Route, mut tabs: Signal<Vec<Route>>) -> Element {
    let navigator = use_navigator();
    let active = current == route;
    let close_route = route.clone();
    rsx! {
        div { class: if active { "page-tab active" } else { "page-tab" },
            Link { to: route.clone(), aria_current: if active { "page" }, "{route.title()}" }
            if route != (Route::Dashboard {}) {
                button {
                    aria_label: format!("关闭{}页签", route.title()),
                    onclick: move |_| {
                        tabs.write().retain(|tab| tab != &close_route);
                        if active {
                            navigator.replace(Route::Dashboard {});
                        }
                    },
                    "×"
                }
            }
        }
    }
}

fn open_tab(mut tabs: Signal<Vec<Route>>, route: Route) {
    if route.tab_allowed() && !tabs.read().contains(&route) {
        tabs.write().push(route);
    }
}
