//! User settings and the recent-files list, persisted with the store plugin.

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Theme};
use tauri_plugin_store::StoreExt;

const STORE_FILE: &str = "settings.json";
const MAX_RECENT: usize = 15;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemePref {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentWidth {
    Narrow,
    #[default]
    Normal,
    Wide,
    Full,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub theme: ThemePref,
    pub zoom: f64,
    pub content_width: ContentWidth,
    pub show_sidebar: bool,
    pub block_remote_images: bool,
    /// App used by File ▸ Open in Editor. `None` means the default text editor.
    pub editor_app: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemePref::System,
            zoom: 1.0,
            content_width: ContentWidth::Normal,
            show_sidebar: true,
            block_remote_images: false,
            editor_app: None,
        }
    }
}

impl Settings {
    pub fn window_theme(&self) -> Option<Theme> {
        match self.theme {
            ThemePref::System => None,
            ThemePref::Light => Some(Theme::Light),
            ThemePref::Dark => Some(Theme::Dark),
        }
    }

    fn sanitized(mut self) -> Self {
        if !self.zoom.is_finite() {
            self.zoom = 1.0;
        }
        self.zoom = self.zoom.clamp(0.5, 3.0);
        self
    }
}

#[derive(Default)]
pub struct AppState {
    pub settings: Mutex<Settings>,
    pub recent: Mutex<Vec<PathBuf>>,
}

pub fn load(app: &AppHandle) {
    let Ok(store) = app.store(STORE_FILE) else {
        return;
    };
    let state = app.state::<AppState>();
    if let Some(value) = store.get("settings")
        && let Ok(settings) = serde_json::from_value::<Settings>(value)
    {
        *state.settings.lock().unwrap() = settings.sanitized();
    }
    if let Some(value) = store.get("recent")
        && let Ok(recent) = serde_json::from_value::<Vec<PathBuf>>(value)
    {
        *state.recent.lock().unwrap() = recent;
    }
}

pub fn current(app: &AppHandle) -> Settings {
    app.state::<AppState>().settings.lock().unwrap().clone()
}

pub fn update(app: &AppHandle, settings: Settings) {
    let settings = settings.sanitized();
    *app.state::<AppState>().settings.lock().unwrap() = settings.clone();
    if let Ok(store) = app.store(STORE_FILE) {
        store.set(
            "settings",
            serde_json::to_value(&settings).unwrap_or_default(),
        );
    }
    for window in app.webview_windows().values() {
        let _ = window.set_theme(settings.window_theme());
    }
    let _ = app.emit("settings:changed", &settings);
}

pub fn recent(app: &AppHandle) -> Vec<PathBuf> {
    app.state::<AppState>().recent.lock().unwrap().clone()
}

pub fn add_recent(app: &AppHandle, path: PathBuf) {
    {
        let state = app.state::<AppState>();
        let mut recent = state.recent.lock().unwrap();
        recent.retain(|p| p != &path);
        recent.insert(0, path.clone());
        recent.truncate(MAX_RECENT);
    }
    save_recent(app);
    crate::platform::note_recent_document(app, &path);
}

pub fn remove_recent(app: &AppHandle, path: &PathBuf) {
    app.state::<AppState>()
        .recent
        .lock()
        .unwrap()
        .retain(|p| p != path);
    save_recent(app);
}

pub fn clear_recent(app: &AppHandle) {
    app.state::<AppState>().recent.lock().unwrap().clear();
    save_recent(app);
    crate::platform::clear_recent_documents(app);
}

fn save_recent(app: &AppHandle) {
    if let Ok(store) = app.store(STORE_FILE) {
        store.set(
            "recent",
            serde_json::to_value(recent(app)).unwrap_or_default(),
        );
    }
    crate::menu::rebuild_recent(app);
    let _ = app.emit("recent:changed", ());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let s: Settings = serde_json::from_str(r#"{"theme":"dark"}"#).unwrap();
        assert_eq!(s.theme, ThemePref::Dark);
        assert_eq!(s.zoom, 1.0);
        assert!(s.show_sidebar);
    }

    #[test]
    fn zoom_is_clamped() {
        let s = Settings {
            zoom: 42.0,
            ..Default::default()
        }
        .sanitized();
        assert_eq!(s.zoom, 3.0);
    }
}
