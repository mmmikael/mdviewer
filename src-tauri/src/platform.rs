//! Small bits of AppKit that Tauri doesn't expose.

use std::path::Path;

use tauri::AppHandle;

/// Adds the file to macOS' own recent documents, which feed the Dock menu
/// and Apple menu ▸ Recent Items.
pub fn note_recent_document(app: &AppHandle, path: &Path) {
    #[cfg(target_os = "macos")]
    {
        let path = path.to_string_lossy().into_owned();
        let _ = app.run_on_main_thread(move || {
            use objc2::MainThreadMarker;
            use objc2_app_kit::NSDocumentController;
            use objc2_foundation::{NSString, NSURL};

            let Some(mtm) = MainThreadMarker::new() else {
                return;
            };
            let url = NSURL::fileURLWithPath(&NSString::from_str(&path));
            NSDocumentController::sharedDocumentController(mtm).noteNewRecentDocumentURL(&url);
        });
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (app, path);
}

pub fn clear_recent_documents(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    {
        let _ = app.run_on_main_thread(|| {
            use objc2::MainThreadMarker;
            use objc2_app_kit::NSDocumentController;

            let Some(mtm) = MainThreadMarker::new() else {
                return;
            };
            // SAFETY: `sender` is unused by NSDocumentController.
            unsafe {
                NSDocumentController::sharedDocumentController(mtm).clearRecentDocuments(None)
            };
        });
    }
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

/// Where the close/minimise/zoom buttons sit, so they line up with our toolbar.
pub const TRAFFIC_LIGHTS: (f64, f64) = (18.0, 26.0);

/// Moves the window's traffic lights. Tauri's own `traffic_light_position` is
/// applied from a draw callback that recent macOS versions don't call for our
/// windows, so we do it here and again after every resize (AppKit resets it).
pub fn position_traffic_lights(window: &tauri::WebviewWindow) {
    #[cfg(target_os = "macos")]
    {
        let Ok(ptr) = window.ns_window() else { return };
        let ptr = ptr as usize;
        let _ = window.run_on_main_thread(move || {
            use objc2_app_kit::{NSWindow, NSWindowButton};

            // SAFETY: Tauri hands out a valid NSWindow pointer for the window's
            // lifetime, and we're on the main thread.
            let window = unsafe { &*(ptr as *const NSWindow) };
            let (x, y) = TRAFFIC_LIGHTS;
            let buttons = [
                NSWindowButton::CloseButton,
                NSWindowButton::MiniaturizeButton,
                NSWindowButton::ZoomButton,
            ]
            .map(|kind| window.standardWindowButton(kind));
            let [Some(close), Some(minimize), Some(zoom)] = buttons else {
                return;
            };
            // SAFETY: the buttons live in the title bar's view hierarchy on the main thread.
            let container = unsafe { close.superview().and_then(|v| v.superview()) };
            let Some(container) = container else {
                return;
            };

            let close_frame = close.frame();
            let mut bar = container.frame();
            bar.size.height = close_frame.size.height + y;
            bar.origin.y = window.frame().size.height - bar.size.height;
            container.setFrame(bar);

            let spacing = minimize.frame().origin.x - close_frame.origin.x;
            for (i, button) in [close, minimize, zoom].iter().enumerate() {
                let mut origin = button.frame().origin;
                origin.x = x + i as f64 * spacing;
                button.setFrameOrigin(origin);
            }
        });
    }
    #[cfg(not(target_os = "macos"))]
    let _ = window;
}

/// Puts text on the general pasteboard.
pub fn copy_to_clipboard(text: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new("pbcopy").stdin(Stdio::piped()).spawn()?;
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(text.as_bytes())?;
    child.wait()?;
    Ok(())
}
