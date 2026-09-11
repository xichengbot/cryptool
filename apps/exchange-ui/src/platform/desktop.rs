use std::fs;
use std::path::PathBuf;

use dioxus::prelude::Callback;
use directories::ProjectDirs;

use super::{FullscreenToggle, Preferences};

fn preference_path() -> Option<PathBuf> {
    ProjectDirs::from("com", "cryptool", "exchange-data")
        .map(|dirs| dirs.config_dir().join("preferences.json"))
}

pub fn load_preferences() -> Preferences {
    preference_path()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|value| serde_json::from_str(&value).ok())
        .unwrap_or_default()
}

pub fn save_preferences(preferences: Preferences) -> Result<(), String> {
    let path = preference_path().ok_or_else(|| "无法确定偏好目录".to_string())?;
    let parent = path.parent().ok_or_else(|| "偏好路径无效".to_string())?;
    fs::create_dir_all(parent).map_err(|_| "无法创建偏好目录".to_string())?;
    let temporary = path.with_extension(format!("json.{}.tmp", std::process::id()));
    let bytes =
        serde_json::to_vec_pretty(&preferences).map_err(|_| "无法编码主题偏好".to_string())?;
    fs::write(&temporary, bytes).map_err(|_| "无法写入主题偏好".to_string())?;
    fs::rename(&temporary, &path).map_err(|_| "无法原子替换主题偏好".to_string())
}

pub fn use_fullscreen_toggle() -> FullscreenToggle {
    let window = dioxus::desktop::use_window();
    Callback::new(move |_| {
        window.set_fullscreen(window.fullscreen().is_none());
    })
}
