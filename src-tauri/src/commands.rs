//! Commands the web UI can call.

use std::path::PathBuf;

use tauri::{AppHandle, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::documents::{self, Document, Documents};
use crate::settings::{self, Settings};
use crate::{menu, watcher};

/// The file this window was created for, if any. Asked once by each new window.
#[tauri::command]
pub fn take_pending_open(window: WebviewWindow, docs: State<Documents>) -> Option<PathBuf> {
    docs.take_pending(window.label())
}

/// Called by the page once its first render is done, to avoid a blank flash.
#[tauri::command]
pub fn show_window(app: AppHandle, window: WebviewWindow) {
    let _ = window.show();
    if documents::is_doc_window(window.label()) {
        crate::platform::position_traffic_lights(&window);
        debug_export(&app, &window);
    }
}

/// With `MDV_DEBUG_EXPORT_DIR` set (debug builds only), every exporter runs once
/// for each document window, so exports can be checked without clicking.
fn debug_export(app: &AppHandle, window: &WebviewWindow) {
    #[cfg(debug_assertions)]
    {
        use tauri::{Emitter, Manager};
        if std::env::var_os("MDV_DEBUG_EXPORT_DIR").is_some()
            && app.state::<Documents>().path_for(window.label()).is_some()
        {
            let _ = app.emit_to(window.label(), "menu:action", "export-html");
            let _ = app.emit_to(window.label(), "menu:action", "copy-rich");
            crate::export::export_pdf(app, window.clone());
        }
    }
    #[cfg(not(debug_assertions))]
    let _ = (app, window);
}

/// Loads a file into the calling window and starts watching it.
#[tauri::command]
pub fn open_document(
    app: AppHandle,
    window: WebviewWindow,
    docs: State<Documents>,
    path: PathBuf,
) -> Result<Document, String> {
    let path = dunce::canonicalize(&path).map_err(|e| {
        settings::remove_recent(&app, &path);
        format!("Couldn’t open “{}”: {e}", path.display())
    })?;
    let doc = documents::read_document(&path)?;

    docs.set(window.label(), path.clone());
    documents::allow_assets(&app, &path);
    if let Err(err) = watcher::watch(&app, window.label(), &path) {
        eprintln!("[mdviewer] couldn’t watch {}: {err}", path.display());
    }
    let _ = window.set_title(&doc.name);
    settings::add_recent(&app, path);
    menu::refresh(&app);
    Ok(doc)
}

/// Re-reads the calling window's file (View ▸ Reload).
#[tauri::command]
pub fn reload_document(window: WebviewWindow, docs: State<Documents>) -> Result<Document, String> {
    let path = docs
        .path_for(window.label())
        .ok_or_else(|| "No document is open.".to_string())?;
    documents::read_document(&path)
}

/// Opens files in their own windows (drag and drop, ⌘-click on links).
/// The calling window is reused if it has no document yet.
#[tauri::command]
pub fn open_paths(app: AppHandle, window: WebviewWindow, paths: Vec<PathBuf>) {
    documents::open_paths(&app, paths, Some(window.label()));
}

#[tauri::command]
pub fn show_open_dialog(app: AppHandle, window: WebviewWindow) {
    documents::show_open_dialog(&app, Some(window.label().to_string()));
}

/// Opens a non-Markdown file linked from a document with its default app.
#[tauri::command]
pub fn open_external_file(app: AppHandle, path: PathBuf) -> Result<(), String> {
    if path.is_dir() {
        return app
            .opener()
            .reveal_item_in_dir(&path)
            .map_err(|e| e.to_string());
    }
    if !path.exists() {
        return Err(format!("“{}” doesn’t exist.", path.display()));
    }
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(|e| e.to_string())
}

/// File ▸ Export as HTML…: the page builds a standalone HTML file, this asks
/// where to save it. Returns the saved path, or nothing if cancelled.
#[tauri::command]
pub async fn export_html(
    app: AppHandle,
    window: WebviewWindow,
    html: String,
) -> Result<Option<PathBuf>, String> {
    let name = crate::export::default_name(&app, &window, "html");
    let Some(path) = crate::export::pick_save_path(&app, &window, &name, "HTML", "html") else {
        return Ok(None);
    };
    std::fs::write(&path, html).map_err(|e| format!("Couldn’t save “{name}”: {e}"))?;
    Ok(Some(path))
}

/// Edit ▸ Copy as Rich Text: HTML for rich text editors plus a plain fallback.
#[tauri::command]
pub fn copy_rich(app: AppHandle, html: String, text: String) -> Result<(), String> {
    crate::export::copy_rich(&app, html, text)
}

/// Clipboard writes from the page aren't reliable outside a click handler
/// (e.g. from a context menu), so go through the native pasteboard.
#[tauri::command]
pub fn copy_text(text: String) -> Result<(), String> {
    crate::platform::copy_to_clipboard(&text).map_err(|e| e.to_string())
}

/// Whether MDViewer opens Markdown files on double-click, and which app does now.
#[tauri::command]
pub fn get_default_viewer() -> crate::platform::DefaultViewer {
    crate::platform::default_viewer()
}

/// Makes MDViewer the default app for Markdown files. Async so the wait for
/// macOS (which may ask the user to confirm) doesn't block the main thread.
#[tauri::command]
pub async fn make_default_viewer(app: AppHandle) -> Result<crate::platform::DefaultViewer, String> {
    crate::platform::make_default_viewer(&app)?;
    Ok(crate::platform::default_viewer())
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Settings {
    settings::current(&app)
}

#[tauri::command]
pub fn set_settings(app: AppHandle, settings: Settings) {
    settings::update(&app, settings);
}

#[tauri::command]
pub fn get_recent(app: AppHandle) -> Vec<PathBuf> {
    settings::recent(&app)
}

/// Removes one file from the recent list (and File ▸ Open Recent).
#[tauri::command]
pub fn remove_recent(app: AppHandle, path: PathBuf) {
    settings::remove_recent(&app, &path);
}

/// Clears the whole recent list, like File ▸ Open Recent ▸ Clear Menu.
#[tauri::command]
pub fn clear_recent(app: AppHandle) {
    settings::clear_recent(&app);
}

#[tauri::command]
pub fn show_settings(app: AppHandle) {
    documents::show_settings_window(&app);
}

/// Lets the user pick the app used by File ▸ Open in Editor.
#[tauri::command]
pub async fn choose_editor_app(app: AppHandle) -> Option<String> {
    app.dialog()
        .file()
        .set_title("Choose an Editor")
        .set_directory("/Applications")
        .add_filter("Applications", &["app"])
        .blocking_pick_file()
        .and_then(|f| f.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned())
}
