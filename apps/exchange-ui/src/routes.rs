use dioxus::prelude::*;

use crate::{components::AppShell, pages::*};

#[derive(Clone, Debug, Eq, PartialEq, Routable)]
#[rustfmt::skip]
pub enum Route {
    #[layout(AppShell)]
        #[route("/")]
        Welcome {},
        #[route("/dashboard")]
        Dashboard {},
        #[route("/binance/spot-history-trade")]
        SpotHistoryTrade {},
        #[route("/binance/future-history-trade")]
        FutureHistoryTrade {},
        #[route("/tools/time-convert", TimeConvertPage)]
        TimeConvert {},
        #[route("/:..segments")]
        NotFound { segments: Vec<String> },
}

impl Route {
    pub const fn title(&self) -> &'static str {
        match self {
            Self::Welcome {} => "欢迎",
            Self::Dashboard {} => "首页",
            Self::SpotHistoryTrade {} => "现货历史成交",
            Self::FutureHistoryTrade {} => "USD-M 合约历史成交",
            Self::TimeConvert {} => "时间转换",
            Self::NotFound { .. } => "页面不存在",
        }
    }

    pub const fn breadcrumb(&self) -> &'static str {
        match self {
            Self::Welcome {} | Self::Dashboard {} => "Exchange Data / 首页",
            Self::SpotHistoryTrade {} => "Exchange Data / Binance / 现货历史成交",
            Self::FutureHistoryTrade {} => "Exchange Data / Binance / USD-M 合约历史成交",
            Self::TimeConvert {} => "Exchange Data / 工具 / 时间转换",
            Self::NotFound { .. } => "Exchange Data / 未找到",
        }
    }

    pub fn tab_allowed(&self) -> bool {
        !matches!(self, Self::Welcome {} | Self::NotFound { .. })
    }
}
