use dioxus::prelude::Callback;
use serde::{Deserialize, Serialize};

#[cfg(feature = "desktop")]
mod desktop;
#[cfg(feature = "web")]
mod web;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Preferences {
    pub dark_mode: bool,
}

#[cfg(feature = "web")]
pub use web::{load_preferences, normalize_legacy_hash, save_preferences, use_fullscreen_toggle};

#[cfg(feature = "desktop")]
pub use desktop::{load_preferences, save_preferences, use_fullscreen_toggle};

#[cfg(feature = "desktop")]
pub fn normalize_legacy_hash() {}

pub type FullscreenToggle = Callback<()>;
