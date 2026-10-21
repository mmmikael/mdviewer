mod commands;
mod documents;
mod menu;
mod platform;
mod settings;
mod watcher;

use std::path::PathBuf;
use std::time::Duration;

use tauri::{AppHandle, Manager, RunEvent, WindowEvent};
use tauri_plugin_window_state::StateFlags;

use documents::Documents;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(
            tauri_plugin_window_state::Builder::default()
                // Windows are shown by the page once rendered, never by the plugin.
                .with_state_flags(StateFlags::all() & !StateFlags::VISIBLE)
                .build(),
        )
        .manage(Documents::default())
        .manage(watcher::Watchers::default())
        .manage(settings::AppState::default())
        .manage(menu::MenuState::default())
        .setup(|app| {
            let handle = app.handle();
            settings::load(handle);
            let menu = menu::build(handle)?;
            app.set_menu(menu)?;
            menu::rebuild_recent(handle);
            Ok(())
        })
        .on_menu_event(menu::handle)
        .invoke_handler(tauri::generate_handler![
            commands::show_window,
            commands::take_pending_open,
            commands::open_document,
            commands::reload_document,
            commands::open_paths,
            commands::show_open_dialog,
            commands::open_external_file,
            commands::copy_text,
            commands::get_settings,
            commands::set_settings,
            commands::get_recent,
            commands::show_settings,
            commands::choose_editor_app,
            commands::get_default_viewer,
            commands::make_default_viewer,
        ])
        .build(tauri::generate_context!())
        .expect("error while building MDViewer");

    app.run(|app, event| match event {
        RunEvent::Ready => on_ready(app),
        // Files opened from Finder, the Dock or `open -a MDViewer file.md`.
        #[cfg(target_os = "macos")]
        RunEvent::Opened { urls } => {
            let paths = urls
                .into_iter()
                .filter_map(|url| url.to_file_path().ok())
                .collect();
            documents::open_paths(app, paths, None);
        }
        // Dock icon clicked while no window is open.
        #[cfg(target_os = "macos")]
        RunEvent::Reopen {
            has_visible_windows: false,
            ..
        } => {
            let _ = documents::create_window(app, None);
        }
        // Like other document apps, stay running after the last window closes.
        RunEvent::ExitRequested {
            api, code: None, ..
        } => api.prevent_exit(),
        RunEvent::WindowEvent { label, event, .. } => match event {
            WindowEvent::Destroyed => {
                app.state::<Documents>().remove(&label);
                watcher::unwatch(app, &label);
                menu::refresh(app);
            }
            // AppKit puts the traffic lights back whenever it lays out the title bar
            // (showing, focusing and resizing the window).
            WindowEvent::Focused(_) | WindowEvent::Resized(_) | WindowEvent::ThemeChanged(_) => {
                if matches!(event, WindowEvent::Focused(true)) {
                    menu::refresh(app);
                }
                if documents::is_doc_window(&label)
                    && let Some(window) = app.get_webview_window(&label)
                {
                    platform::position_traffic_lights(&window);
                }
            }
            _ => {}
        },
        _ => {}
    });
}

fn on_ready(app: &AppHandle) {
    let paths = command_line_paths();
    if !paths.is_empty() {
        documents::open_paths(app, paths, None);
    }
    // If nothing asked to open a file shortly after launch, show an empty window.
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(400));
        let inner = handle.clone();
        let _ = handle.run_on_main_thread(move || {
            if !documents::has_doc_windows(&inner) {
                let _ = documents::create_window(&inner, None);
            }
        });
    });
}

/// Files passed on the command line, e.g. `MDViewer README.md` or
/// `pnpm tauri dev -- -- README.md`.
fn command_line_paths() -> Vec<PathBuf> {
    std::env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .filter(|p| p.is_file())
        .collect()
}
