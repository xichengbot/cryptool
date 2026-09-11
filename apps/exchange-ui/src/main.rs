#[cfg(all(feature = "web", feature = "desktop"))]
compile_error!("features 'web' and 'desktop' are mutually exclusive");

#[cfg(all(feature = "desktop", target_arch = "wasm32"))]
compile_error!("feature 'desktop' is unavailable for wasm32");

#[cfg(all(
    feature = "desktop",
    not(target_arch = "wasm32"),
    not(any(target_os = "macos", target_os = "windows"))
))]
compile_error!("Exchange Data desktop supports only macOS and Windows");

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
compile_error!("feature 'web' must be built for wasm32-unknown-unknown");

#[cfg(not(any(feature = "web", feature = "desktop")))]
compile_error!("enable exactly one renderer feature: 'web' or 'desktop'");

#[cfg(any(feature = "web", feature = "desktop"))]
fn main() {
    dioxus::launch(exchange_ui::app::App);
}
