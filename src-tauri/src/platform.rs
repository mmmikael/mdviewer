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

/// The content type macOS gives `.md` files.
#[cfg(target_os = "macos")]
const MARKDOWN_TYPE: &str = "net.daringfireball.markdown";

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DefaultViewer {
    /// MDViewer already opens Markdown files on double-click.
    pub is_default: bool,
    /// The app that currently does, e.g. "Xcode".
    pub current_app: Option<String>,
    /// False when running unbundled (`tauri dev`), where macOS can't register us.
    pub can_change: bool,
}

pub fn default_viewer() -> DefaultViewer {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::NSWorkspace;
        use objc2_foundation::NSBundle;

        let ours = NSBundle::mainBundle().bundlePath().to_string();
        let current = markdown_type()
            .and_then(|t| NSWorkspace::sharedWorkspace().URLForApplicationToOpenContentType(&t))
            .and_then(|url| url.path())
            .map(|path| path.to_string());
        DefaultViewer {
            is_default: current.as_deref() == Some(ours.as_str()),
            current_app: current.as_deref().map(app_name),
            can_change: ours.ends_with(".app"),
        }
    }
    #[cfg(not(target_os = "macos"))]
    DefaultViewer {
        is_default: false,
        current_app: None,
        can_change: false,
    }
}

#[cfg(target_os = "macos")]
fn markdown_type() -> Option<objc2::rc::Retained<objc2_uniform_type_identifiers::UTType>> {
    use objc2_foundation::NSString;
    objc2_uniform_type_identifiers::UTType::typeWithIdentifier(&NSString::from_str(MARKDOWN_TYPE))
}

fn app_name(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

/// Asks macOS to open Markdown files with MDViewer from now on. macOS may ask
/// the user to confirm. Blocks until macOS answers, so call it off the main thread.
pub fn make_default_viewer(app: &AppHandle) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use std::sync::mpsc;
        use std::time::Duration;

        let (tx, rx) = mpsc::channel::<Result<(), String>>();
        app.run_on_main_thread(move || {
            use block2::RcBlock;
            use objc2_app_kit::NSWorkspace;
            use objc2_foundation::{NSBundle, NSError};

            let Some(content_type) = markdown_type() else {
                let _ = tx.send(Err("macOS doesn’t know the Markdown file type.".into()));
                return;
            };
            let done = tx.clone();
            let handler = RcBlock::new(move |error: *mut NSError| {
                let result = if error.is_null() {
                    Ok(())
                } else {
                    // SAFETY: a non-null error passed to the handler is a valid NSError.
                    Err(unsafe { &*error }.localizedDescription().to_string())
                };
                let _ = done.send(result);
            });
            NSWorkspace::sharedWorkspace()
                .setDefaultApplicationAtURL_toOpenContentType_completionHandler(
                    &NSBundle::mainBundle().bundleURL(),
                    &content_type,
                    Some(&handler),
                );
        })
        .map_err(|e| e.to_string())?;
        rx.recv_timeout(Duration::from_secs(120))
            .map_err(|_| "macOS didn’t respond.".to_string())?
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Err("Only supported on macOS.".into())
    }
}
