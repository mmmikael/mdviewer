//! Document windows: which window shows which file, and how files get opened.
//!
//! Every document gets its own window (label `doc-N`). macOS groups them into
//! native tabs according to the user's "Prefer tabs" setting because they share
//! a tabbing identifier.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_dialog::DialogExt;

use crate::settings;

pub const DOC_PREFIX: &str = "doc-";
pub const SETTINGS_LABEL: &str = "settings";
pub const MARKDOWN_EXTENSIONS: &[&str] = &[
    "md", "markdown", "mdown", "mkd", "mkdn", "mdwn", "mdx", "txt",
];
const MAX_FILE_BYTES: u64 = 20 * 1024 * 1024;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub path: PathBuf,
    pub name: String,
    pub dir: PathBuf,
    pub content: String,
    /// True when the file wasn't valid UTF-8 and some bytes were replaced.
    pub lossy: bool,
}

#[derive(Default)]
pub struct Documents(Mutex<Inner>);

#[derive(Default)]
struct Inner {
    /// Window label → the file it shows (or is about to show).
    open: HashMap<String, PathBuf>,
    /// Window label → file to load once its web view has started.
    pending: HashMap<String, PathBuf>,
    next_id: u32,
}

impl Documents {
    pub fn path_for(&self, label: &str) -> Option<PathBuf> {
        self.0.lock().unwrap().open.get(label).cloned()
    }

    pub fn label_for(&self, path: &Path) -> Option<String> {
        let inner = self.0.lock().unwrap();
        // Prefer the oldest window when several show the same file.
        let mut labels: Vec<_> = inner
            .open
            .iter()
            .filter(|(_, p)| p.as_path() == path)
            .map(|(l, _)| l.clone())
            .collect();
        labels.sort_by_key(|l| window_number(l));
        labels.into_iter().next()
    }

    pub fn set(&self, label: &str, path: PathBuf) {
        self.0.lock().unwrap().open.insert(label.to_string(), path);
    }

    pub fn remove(&self, label: &str) {
        let mut inner = self.0.lock().unwrap();
        inner.open.remove(label);
        inner.pending.remove(label);
    }

    /// Assigns `path` to an existing window that will load it via `take_pending`.
    pub fn claim(&self, label: &str, path: PathBuf) {
        let mut inner = self.0.lock().unwrap();
        inner.open.insert(label.to_string(), path.clone());
        inner.pending.insert(label.to_string(), path);
    }

    pub fn take_pending(&self, label: &str) -> Option<PathBuf> {
        self.0.lock().unwrap().pending.remove(label)
    }

    fn reserve_window(&self, path: Option<PathBuf>) -> String {
        let mut inner = self.0.lock().unwrap();
        inner.next_id += 1;
        let label = format!("{DOC_PREFIX}{}", inner.next_id);
        if let Some(path) = path {
            inner.open.insert(label.clone(), path.clone());
            inner.pending.insert(label.clone(), path);
        }
        label
    }
}

fn window_number(label: &str) -> u32 {
    label
        .strip_prefix(DOC_PREFIX)
        .and_then(|n| n.parse().ok())
        .unwrap_or(u32::MAX)
}

pub fn is_doc_window(label: &str) -> bool {
    label.starts_with(DOC_PREFIX)
}

pub fn read_document(path: &Path) -> Result<Document, String> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned());
    let meta = fs::metadata(path).map_err(|e| format!("Couldn’t open “{name}”: {e}"))?;
    if !meta.is_file() {
        return Err(format!("“{name}” is not a file."));
    }
    if meta.len() > MAX_FILE_BYTES {
        return Err(format!(
            "“{name}” is too large to display ({} MB).",
            meta.len() / 1024 / 1024
        ));
    }
    let bytes = fs::read(path).map_err(|e| format!("Couldn’t read “{name}”: {e}"))?;
    let (mut content, lossy) = match String::from_utf8(bytes) {
        Ok(text) => (text, false),
        Err(err) => (String::from_utf8_lossy(err.as_bytes()).into_owned(), true),
    };
    if content.starts_with('\u{feff}') {
        content.remove(0);
    }
    Ok(Document {
        path: path.to_path_buf(),
        name,
        dir: path.parent().map(Path::to_path_buf).unwrap_or_default(),
        content,
        lossy,
    })
}

/// Opens files coming from Finder, the Dock, the Open dialog, drag and drop or
/// the command line. A file that is already open just brings its window forward.
/// `prefer` names a window that may be reused if it has no document yet.
pub fn open_paths(app: &AppHandle, paths: Vec<PathBuf>, prefer: Option<&str>) {
    let docs = app.state::<Documents>();
    for path in paths {
        let Ok(path) = dunce::canonicalize(&path) else {
            settings::remove_recent(app, &path);
            continue;
        };
        if !path.is_file() {
            continue;
        }
        if let Some(label) = docs.label_for(&path)
            && let Some(window) = app.get_webview_window(&label)
        {
            focus(&window);
            continue;
        }
        if let Some(window) = empty_window(app, prefer) {
            // Claim the window now so the next path in this batch doesn't reuse it.
            // The page may still be starting up and not listening yet, so leave the
            // file as pending too; the page takes it on start or on `doc:open`.
            docs.claim(window.label(), path);
            let _ = app.emit_to(window.label(), "doc:open", ());
            focus(&window);
            continue;
        }
        if let Err(err) = create_window(app, Some(path)) {
            eprintln!("[mdviewer] couldn’t create window: {err}");
        }
    }
}

/// A document window that hasn't loaded a file yet (e.g. the welcome window).
fn empty_window(app: &AppHandle, prefer: Option<&str>) -> Option<WebviewWindow> {
    let docs = app.state::<Documents>();
    let is_empty =
        |w: &WebviewWindow| is_doc_window(w.label()) && docs.path_for(w.label()).is_none();
    if let Some(window) = prefer.and_then(|l| app.get_webview_window(l))
        && is_empty(&window)
    {
        return Some(window);
    }
    let mut windows: Vec<_> = app
        .webview_windows()
        .into_values()
        .filter(is_empty)
        .collect();
    windows.sort_by_key(|w| window_number(w.label()));
    windows.into_iter().next()
}

pub fn has_doc_windows(app: &AppHandle) -> bool {
    app.webview_windows().keys().any(|l| is_doc_window(l))
}

pub fn focused_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.webview_windows()
        .into_values()
        .find(|w| w.is_focused().unwrap_or(false))
}

fn focus(window: &WebviewWindow) {
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

pub fn create_window(app: &AppHandle, path: Option<PathBuf>) -> tauri::Result<WebviewWindow> {
    let docs = app.state::<Documents>();
    let label = docs.reserve_window(path);
    let theme = settings::current(app).window_theme();
    let anchor = focused_window(app)
        .filter(|w| is_doc_window(w.label()))
        .and_then(|w| w.outer_position().ok());

    let builder = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("index.html".into()))
        .title("MDViewer")
        .inner_size(880.0, 960.0)
        .min_inner_size(420.0, 320.0)
        .theme(theme)
        .on_navigation(is_app_url)
        // Shown by the page once it has rendered, to avoid a blank flash.
        .visible(false);
    #[cfg(target_os = "macos")]
    let builder = {
        use tauri::TitleBarStyle;
        use tauri::utils::config::WindowEffectsConfig;
        use tauri::window::{Effect, EffectState};
        builder
            .title_bar_style(TitleBarStyle::Overlay)
            .hidden_title(true)
            .transparent(true)
            .effects(WindowEffectsConfig {
                effects: vec![Effect::Sidebar],
                state: Some(EffectState::FollowsWindowActiveState),
                radius: None,
                color: None,
            })
            .tabbing_identifier("com.mmmikael.mdviewer.document")
    };
    let window = builder.build()?;
    crate::platform::install_unified_toolbar(&window);

    // Cascade instead of stacking exactly on top of the current window.
    if let (Some(anchor), Ok(position)) = (anchor, window.outer_position())
        && anchor == position
    {
        let _ = window.set_position(tauri::PhysicalPosition::new(
            position.x + 56,
            position.y + 56,
        ));
    }

    // Safety net in case the page fails before it can show itself.
    let fallback = window.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(1500));
        if !fallback.is_visible().unwrap_or(true) {
            let _ = fallback.show();
        }
    });
    Ok(window)
}

/// Keeps web views on the app's own pages: links in documents are handled by
/// the page, and nothing in a document may navigate the window away.
fn is_app_url(url: &tauri::Url) -> bool {
    match url.scheme() {
        "tauri" | "about" => true,
        "http" | "https" => {
            url.host_str() == Some("tauri.localhost")
                || (cfg!(debug_assertions)
                    && matches!(url.host_str(), Some("localhost" | "127.0.0.1")))
        }
        _ => false,
    }
}

pub fn show_open_dialog(app: &AppHandle, prefer: Option<String>) {
    let handle = app.clone();
    app.dialog()
        .file()
        .add_filter("Markdown", MARKDOWN_EXTENSIONS)
        .pick_files(move |files| {
            let paths: Vec<PathBuf> = files
                .unwrap_or_default()
                .into_iter()
                .filter_map(|f| f.into_path().ok())
                .collect();
            if !paths.is_empty() {
                open_paths(&handle, paths, prefer.as_deref());
            }
        });
}

pub fn show_settings_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(SETTINGS_LABEL) {
        focus(&window);
        return;
    }
    let theme = settings::current(app).window_theme();
    let builder =
        WebviewWindowBuilder::new(app, SETTINGS_LABEL, WebviewUrl::App("index.html".into()))
            .title("Settings")
            .inner_size(520.0, 400.0)
            .resizable(false)
            .minimizable(false)
            .maximizable(false)
            .theme(theme)
            .on_navigation(is_app_url)
            .visible(false)
            .center();
    #[cfg(target_os = "macos")]
    let builder = builder.tabbing_identifier("com.mmmikael.mdviewer.settings");
    if let Err(err) = builder.build() {
        eprintln!("[mdviewer] couldn’t open settings: {err}");
    }
}

/// Lets the page load local images that sit next to (or below) the document.
/// Inside a git repository the whole repository is allowed, so `../assets/x.png`
/// style references work too.
pub fn allow_assets(app: &AppHandle, path: &Path) {
    let Some(dir) = path.parent() else { return };
    let root = asset_root(dir, app.path().home_dir().ok().as_deref());
    let _ = app.asset_protocol_scope().allow_directory(root, true);
}

fn asset_root<'a>(dir: &'a Path, home: Option<&Path>) -> &'a Path {
    dir.ancestors()
        .find(|d| d.join(".git").exists())
        // Never widen to the home folder or a top-level folder (e.g. a dotfiles repo).
        .filter(|root| Some(*root) != home && root.components().count() > 3)
        .unwrap_or(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_dedupes_and_prefers_oldest_window() {
        let docs = Documents::default();
        let a = docs.reserve_window(Some("/tmp/a.md".into()));
        let b = docs.reserve_window(Some("/tmp/a.md".into()));
        assert_eq!(a, "doc-1");
        assert_eq!(b, "doc-2");
        assert_eq!(
            docs.label_for(Path::new("/tmp/a.md")).as_deref(),
            Some("doc-1")
        );
        assert_eq!(docs.take_pending("doc-2"), Some("/tmp/a.md".into()));
        assert_eq!(docs.take_pending("doc-2"), None);
        docs.remove("doc-1");
        assert_eq!(
            docs.label_for(Path::new("/tmp/a.md")).as_deref(),
            Some("doc-2")
        );
    }

    #[test]
    fn claimed_window_is_open_and_pending_once() {
        let docs = Documents::default();
        let label = docs.reserve_window(None);
        docs.claim(&label, "/tmp/b.md".into());
        assert_eq!(docs.label_for(Path::new("/tmp/b.md")), Some(label.clone()));
        assert_eq!(docs.take_pending(&label), Some("/tmp/b.md".into()));
        assert_eq!(docs.take_pending(&label), None);
    }

    #[test]
    fn empty_window_reservation_has_no_path() {
        let docs = Documents::default();
        let label = docs.reserve_window(None);
        assert_eq!(docs.path_for(&label), None);
        assert_eq!(docs.take_pending(&label), None);
    }

    #[test]
    fn reads_utf8_and_strips_bom() {
        let dir = std::env::temp_dir().join(format!("mdv-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("bom.md");
        fs::write(&file, "\u{feff}# Hi").unwrap();
        let doc = read_document(&file).unwrap();
        assert_eq!(doc.content, "# Hi");
        assert_eq!(doc.name, "bom.md");
        assert!(!doc.lossy);

        fs::write(&file, b"# caf\xe9").unwrap();
        let doc = read_document(&file).unwrap();
        assert!(doc.lossy);
        assert!(doc.content.starts_with("# caf"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn asset_root_uses_repo_root_but_never_home() {
        let base = std::env::temp_dir().join(format!("mdv-root-{}", std::process::id()));
        let docs_dir = base.join("repo/docs/guide");
        fs::create_dir_all(&docs_dir).unwrap();
        fs::create_dir_all(base.join("repo/.git")).unwrap();
        let base = dunce::canonicalize(&base).unwrap();
        let docs_dir = base.join("repo/docs/guide");
        assert_eq!(asset_root(&docs_dir, None), base.join("repo"));
        assert_eq!(asset_root(&docs_dir, Some(&base.join("repo"))), docs_dir);
        fs::remove_dir_all(&base).unwrap();
    }
}
