use dioxus::prelude::Callback;
use wasm_bindgen::JsValue;

use super::{FullscreenToggle, Preferences};

const PREFERENCE_KEY: &str = "cryptool.exchange-data.preferences";

pub fn load_preferences() -> Preferences {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(PREFERENCE_KEY).ok().flatten())
        .and_then(|value| serde_json::from_str(&value).ok())
        .unwrap_or_default()
}

pub fn save_preferences(preferences: Preferences) -> Result<(), String> {
    let value = serde_json::to_string(&preferences).map_err(|_| "无法保存主题偏好".to_string())?;
    let storage = web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .ok_or_else(|| "浏览器存储不可用".to_string())?;
    storage
        .set_item(PREFERENCE_KEY, &value)
        .map_err(|_| "浏览器拒绝保存主题偏好".to_string())
}

pub fn use_fullscreen_toggle() -> FullscreenToggle {
    Callback::new(move |_| {
        if let Some(document) = web_sys::window().and_then(|window| window.document()) {
            if document.fullscreen_element().is_some() {
                document.exit_fullscreen();
            } else if let Some(element) = document.document_element() {
                let _ = element.request_fullscreen();
            }
        }
    })
}

pub fn normalize_legacy_hash() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(hash) = window.location().hash() else {
        return;
    };
    let Some(route) = hash
        .strip_prefix('#')
        .filter(|value| value.starts_with('/'))
    else {
        return;
    };
    let _ = window
        .history()
        .and_then(|history| history.replace_state_with_url(&JsValue::NULL, "", Some(route)));
}
