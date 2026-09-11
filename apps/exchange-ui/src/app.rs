use dioxus::prelude::*;

use crate::{client, platform, routes::Route, state::TimeHistory};

static MAIN_CSS: Asset = asset!("/assets/main.css");
static FAVICON: Asset = asset!("/assets/favicon-green.svg");

#[component]
#[allow(non_snake_case)]
pub fn App() -> Element {
    use_hook(platform::normalize_legacy_hash);
    let bootstrap = use_hook(client::bootstrap);
    use_context_provider(|| bootstrap.clone());
    use_context_provider(|| Signal::new(platform::load_preferences()));
    use_context_provider(|| Signal::new(TimeHistory::default()));

    rsx! {
        document::Title { "Exchange Data" }
        document::Link { rel: "icon", r#type: "image/svg+xml", href: FAVICON }
        document::Stylesheet { href: MAIN_CSS }
        Router::<Route> {}
    }
}
