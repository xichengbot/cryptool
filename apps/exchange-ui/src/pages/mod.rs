mod dashboard;
pub mod time_convert;
mod trade;

pub use dashboard::{Dashboard, Welcome};
pub use time_convert::TimeConvertPage;
pub use trade::{FutureHistoryTrade, SpotHistoryTrade};

use dioxus::prelude::*;

#[component]
pub fn NotFound(segments: Vec<String>) -> Element {
    let path = segments.join("/");
    rsx! {
        section { class: "empty-state",
            p { class: "eyebrow", "404" }
            h1 { "页面不存在" }
            p { class: "muted", "未找到 /{path}" }
            Link { class: "primary-button", to: crate::routes::Route::Dashboard {}, "返回首页" }
        }
    }
}
