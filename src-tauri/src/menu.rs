//! The macOS menu bar.
//!
//! Actions that need native APIs (dialogs, Finder, printing) run here; the rest
//! are forwarded to the focused window as a `menu:action` event.

use std::path::Path;
use std::sync::Mutex;

use tauri::menu::{
    AboutMetadata, HELP_SUBMENU_ID, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu,
    WINDOW_SUBMENU_ID,
};
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_opener::OpenerExt;

use crate::documents::{self, Documents};
use crate::{platform, settings};

#[derive(Default)]
pub struct MenuState(Mutex<Option<Handles>>);

struct Handles {
    recent: Submenu<Wry>,
    /// Items that only make sense when a document window is focused.
    doc_items: Vec<MenuItem<Wry>>,
}

pub fn build(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let name = app.package_info().name.clone();
    let item =
        |id: &str, text: &str, accel: Option<&str>| MenuItem::with_id(app, id, text, true, accel);
    let sep = || PredefinedMenuItem::separator(app);

    let about = PredefinedMenuItem::about(
        app,
        Some(&format!("About {name}")),
        Some(AboutMetadata {
            name: Some(name.clone()),
            version: Some(app.package_info().version.to_string()),
            comments: Some("A small Markdown viewer.".into()),
            ..Default::default()
        }),
    )?;
    let app_menu = Submenu::with_items(
        app,
        &name,
        true,
        &[
            &about,
            &sep()?,
            &item("settings", "Settings…", Some("CmdOrCtrl+,"))?,
            &sep()?,
            &PredefinedMenuItem::services(app, None)?,
            &sep()?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::show_all(app, None)?,
            &sep()?,
            &PredefinedMenuItem::quit(app, None)?,
        ],
    )?;

    let recent = Submenu::with_id(app, "recent", "Open Recent", true)?;
    let reveal = item("reveal", "Reveal in Finder", Some("CmdOrCtrl+Shift+R"))?;
    let open_editor = item("open-editor", "Open in Editor", Some("CmdOrCtrl+E"))?;
    let copy_path = item("copy-path", "Copy File Path", Some("CmdOrCtrl+Alt+C"))?;
    let print = item("print", "Print…", Some("CmdOrCtrl+P"))?;
    let file_menu = Submenu::with_items(
        app,
        "File",
        true,
        &[
            &item("open", "Open…", Some("CmdOrCtrl+O"))?,
            &recent,
            &sep()?,
            &PredefinedMenuItem::close_window(app, None)?,
            &sep()?,
            &reveal,
            &open_editor,
            &copy_path,
            &sep()?,
            &print,
        ],
    )?;

    let find = item("find", "Find…", Some("CmdOrCtrl+F"))?;
    let find_next = item("find-next", "Find Next", Some("CmdOrCtrl+G"))?;
    let find_prev = item("find-prev", "Find Previous", Some("CmdOrCtrl+Shift+G"))?;
    let find_menu = Submenu::with_items(app, "Find", true, &[&find, &find_next, &find_prev])?;
    let edit_menu = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &sep()?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
            &sep()?,
            &find_menu,
        ],
    )?;

    let toggle_sidebar = item("toggle-sidebar", "Toggle Sidebar", Some("CmdOrCtrl+Ctrl+S"))?;
    let zoom_reset = item("zoom-reset", "Actual Size", Some("CmdOrCtrl+0"))?;
    let zoom_in = item("zoom-in", "Zoom In", Some("CmdOrCtrl+="))?;
    let zoom_out = item("zoom-out", "Zoom Out", Some("CmdOrCtrl+-"))?;
    let reload = item("reload", "Reload", Some("CmdOrCtrl+R"))?;
    // macOS adds "Show Tab Bar" etc. to the menu titled "View".
    let view_menu = Submenu::with_items(
        app,
        "View",
        true,
        &[
            &toggle_sidebar,
            &sep()?,
            &zoom_reset,
            &zoom_in,
            &zoom_out,
            &sep()?,
            &reload,
            &sep()?,
            &PredefinedMenuItem::fullscreen(app, None)?,
        ],
    )?;

    let back = item("back", "Back", Some("CmdOrCtrl+["))?;
    let forward = item("forward", "Forward", Some("CmdOrCtrl+]"))?;
    let go_menu = Submenu::with_items(app, "Go", true, &[&back, &forward])?;

    // Using WINDOW_SUBMENU_ID makes this the app's Windows menu, so macOS adds
    // the window list and tab commands (Merge All Windows, …) itself.
    let window_menu = Submenu::with_id_and_items(
        app,
        WINDOW_SUBMENU_ID,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, None)?,
        ],
    )?;
    let help_menu = Submenu::with_id_and_items(
        app,
        HELP_SUBMENU_ID,
        "Help",
        true,
        &[&item("help", &format!("{name} Help"), None)?],
    )?;

    let menu = Menu::with_items(
        app,
        &[
            &app_menu,
            &file_menu,
            &edit_menu,
            &view_menu,
            &go_menu,
            &window_menu,
            &help_menu,
        ],
    )?;

    *app.state::<MenuState>().0.lock().unwrap() = Some(Handles {
        recent,
        doc_items: vec![
            reveal,
            open_editor,
            copy_path,
            print,
            find,
            find_next,
            find_prev,
            reload,
            back,
            forward,
            toggle_sidebar,
            zoom_reset,
            zoom_in,
            zoom_out,
        ],
    });
    Ok(menu)
}

/// Rebuilds File ▸ Open Recent from the stored list.
pub fn rebuild_recent(app: &AppHandle) {
    let state = app.state::<MenuState>();
    let guard = state.0.lock().unwrap();
    let Some(handles) = guard.as_ref() else {
        return;
    };
    let submenu = &handles.recent;
    if let Ok(items) = submenu.items() {
        for item in items {
            let _ = submenu.remove(&item);
        }
    }
    let recent = settings::recent(app);
    for (i, path) in recent.iter().enumerate() {
        if let Ok(item) = MenuItem::with_id(
            app,
            format!("recent:{i}"),
            recent_label(path),
            true,
            None::<&str>,
        ) {
            let _ = submenu.append(&item);
        }
    }
    if !recent.is_empty()
        && let Ok(sep) = PredefinedMenuItem::separator(app)
    {
        let _ = submenu.append(&sep);
    }
    if let Ok(clear) = MenuItem::with_id(
        app,
        "recent-clear",
        "Clear Menu",
        !recent.is_empty(),
        None::<&str>,
    ) {
        let _ = submenu.append(&clear);
    }
}

fn recent_label(path: &Path) -> String {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    match path.parent().and_then(|p| p.file_name()) {
        Some(parent) => format!("{name} — {}", parent.to_string_lossy()),
        None => name,
    }
}

/// Enables document commands only while a document window is focused.
pub fn refresh(app: &AppHandle) {
    let has_doc = documents::focused_window(app)
        .is_some_and(|w| app.state::<Documents>().path_for(w.label()).is_some());
    let state = app.state::<MenuState>();
    if let Some(handles) = state.0.lock().unwrap().as_ref() {
        for item in &handles.doc_items {
            let _ = item.set_enabled(has_doc);
        }
    }
}

pub fn handle(app: &AppHandle, event: MenuEvent) {
    let id = event.id().as_ref();
    let focused = documents::focused_window(app).filter(|w| documents::is_doc_window(w.label()));
    let focused_path = focused
        .as_ref()
        .and_then(|w| app.state::<Documents>().path_for(w.label()));

    match id {
        "open" => documents::show_open_dialog(app, focused.map(|w| w.label().to_string())),
        "settings" => documents::show_settings_window(app),
        "help" => open_help(app),
        "recent-clear" => settings::clear_recent(app),
        _ if id.starts_with("recent:") => {
            let index: usize = id["recent:".len()..].parse().unwrap_or(usize::MAX);
            if let Some(path) = settings::recent(app).get(index).cloned() {
                documents::open_paths(app, vec![path], focused.as_ref().map(|w| w.label()));
            }
        }
        "reveal" => {
            if let Some(path) = focused_path {
                let _ = app.opener().reveal_item_in_dir(path);
            }
        }
        "open-editor" => {
            if let Some(path) = focused_path {
                open_in_editor(app, &path);
            }
        }
        "copy-path" => {
            if let Some(path) = focused_path {
                let _ = platform::copy_to_clipboard(&path.to_string_lossy());
            }
        }
        "print" => {
            if let Some(window) = focused {
                let _ = window.print();
            }
        }
        _ => {
            if let Some(window) = focused {
                let _ = app.emit_to(window.label(), "menu:action", id);
            }
        }
    }
}

pub fn open_in_editor(app: &AppHandle, path: &Path) {
    let editor = settings::current(app).editor_app;
    let result = match editor {
        Some(editor) => app.opener().open_path(path.to_string_lossy(), Some(editor)),
        None => std::process::Command::new("open")
            .arg("-t")
            .arg(path)
            .status()
            .map(|_| ())
            .map_err(Into::into),
    };
    if let Err(err) = result {
        eprintln!("[mdviewer] couldn’t open editor: {err}");
    }
}

fn open_help(app: &AppHandle) {
    match app
        .path()
        .resolve("resources/Help.md", BaseDirectory::Resource)
    {
        Ok(path) => documents::open_paths(app, vec![path], None),
        Err(err) => eprintln!("[mdviewer] help not found: {err}"),
    }
}
