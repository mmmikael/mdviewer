//! File ▸ Export as PDF/HTML and Edit ▸ Copy as Rich Text.
//!
//! PDF export prints the page with WebKit's own print engine (so it gets the
//! print stylesheet and real page breaks) into a file instead of a printer.
//! HTML is assembled by the page; this module only asks where to save it.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

use crate::documents::Documents;

/// Suggested export name: the document's name with a new extension.
pub fn default_name(app: &AppHandle, window: &WebviewWindow, ext: &str) -> String {
    let stem = app
        .state::<Documents>()
        .path_for(window.label())
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "Untitled".into());
    format!("{stem}.{ext}")
}

/// Shows a save sheet on `window`. Blocks until the user answers, so it must
/// not run on the main thread.
pub fn pick_save_path(
    app: &AppHandle,
    window: &WebviewWindow,
    name: &str,
    filter: &str,
    ext: &str,
) -> Option<PathBuf> {
    // Lets exports be tested without clicking through the save panel.
    #[cfg(debug_assertions)]
    if let Ok(dir) = std::env::var("MDV_DEBUG_EXPORT_DIR") {
        return Some(Path::new(&dir).join(name));
    }
    app.dialog()
        .file()
        .set_parent(window)
        .set_file_name(name)
        .add_filter(filter, &[ext])
        .blocking_save_file()
        .and_then(|f| f.into_path().ok())
}

/// File ▸ Export as PDF…: asks for a file name, then prints the page into it.
pub fn export_pdf(app: &AppHandle, window: WebviewWindow) {
    let app = app.clone();
    std::thread::spawn(move || {
        let name = default_name(&app, &window, "pdf");
        let Some(path) = pick_save_path(&app, &window, &name, "PDF", "pdf") else {
            return;
        };
        // The save panel already confirmed replacing an existing file; removing
        // it first lets us notice when the new one has been written.
        let _ = std::fs::remove_file(&path);
        if let Err(err) = print_to_pdf(&window, &path) {
            let _ = window.emit_to(window.label(), "export:failed", err);
            return;
        }
        let label = window.label().to_string();
        match wait_for_file(&path, Duration::from_secs(60)) {
            true => app.emit_to(label.as_str(), "export:done", &path),
            false => app.emit_to(
                label.as_str(),
                "export:failed",
                "The PDF couldn’t be created.",
            ),
        }
        .ok();
    });
}

fn print_to_pdf(window: &WebviewWindow, path: &Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let path = path.to_string_lossy().into_owned();
        window
            .with_webview(move |platform| {
                use objc2_app_kit::{
                    NSPrintInfo, NSPrintJobSavingURL, NSPrintSaveJob, NSPrintingPaginationMode,
                };
                use objc2_foundation::{NSCopying, NSString, NSURL};
                use objc2_web_kit::WKWebView;

                // SAFETY: on macOS the platform web view is a WKWebView, and this
                // closure runs on the main thread.
                unsafe {
                    let webview = &*(platform.inner() as *const WKWebView);
                    // A copy, so the user's own print settings aren't changed.
                    let info: objc2::rc::Retained<NSPrintInfo> =
                        NSPrintInfo::sharedPrintInfo().copy();
                    info.setJobDisposition(NSPrintSaveJob);
                    let url = NSURL::fileURLWithPath(&NSString::from_str(&path));
                    info.dictionary().insert(NSPrintJobSavingURL, &url);
                    for set in [
                        NSPrintInfo::setTopMargin,
                        NSPrintInfo::setBottomMargin,
                        NSPrintInfo::setLeftMargin,
                        NSPrintInfo::setRightMargin,
                    ] {
                        set(&info, 54.0);
                    }
                    info.setHorizontalPagination(NSPrintingPaginationMode::Fit);
                    info.setVerticallyCentered(false);

                    let operation = webview.printOperationWithPrintInfo(&info);
                    operation.setShowsPrintPanel(false);
                    operation.setShowsProgressPanel(false);
                    // WebKit renders a blank page unless the view has a size.
                    if let Some(view) = operation.view() {
                        view.setFrame(webview.frame());
                    }
                    if let Some(ns_window) = webview.window() {
                        operation.runOperationModalForWindow_delegate_didRunSelector_contextInfo(
                            &ns_window,
                            None,
                            None,
                            std::ptr::null_mut(),
                        );
                    }
                }
            })
            .map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, path);
        Err("Only supported on macOS.".into())
    }
}

/// Waits until `path` exists and has stopped growing.
fn wait_for_file(path: &Path, timeout: Duration) -> bool {
    let start = Instant::now();
    let mut last_size = None;
    while start.elapsed() < timeout {
        std::thread::sleep(Duration::from_millis(250));
        let size = std::fs::metadata(path).ok().map(|m| m.len());
        if size.is_some_and(|s| s > 0) && size == last_size {
            return true;
        }
        last_size = size;
    }
    false
}

/// Puts HTML (for rich text editors) and plain text on the pasteboard.
pub fn copy_rich(app: &AppHandle, html: String, text: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        app.run_on_main_thread(move || {
            use objc2_app_kit::{NSPasteboard, NSPasteboardTypeHTML, NSPasteboardTypeString};
            use objc2_foundation::NSString;

            let pasteboard = NSPasteboard::generalPasteboard();
            pasteboard.clearContents();
            // SAFETY: the pasteboard type constants are immutable AppKit statics.
            unsafe {
                pasteboard.setString_forType(&NSString::from_str(&html), NSPasteboardTypeHTML);
                pasteboard.setString_forType(&NSString::from_str(&text), NSPasteboardTypeString);
            }
        })
        .map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, html, text);
        Err("Only supported on macOS.".into())
    }
}
