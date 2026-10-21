//! Watches open documents and pushes new content to their window.
//!
//! We watch the document's *folder* rather than the file itself: many editors
//! save by writing a temporary file and renaming it over the original, which
//! would silently end a watch on the old file.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};
use tauri::{AppHandle, Emitter, Manager};

use crate::documents::read_document;

const DEBOUNCE: Duration = Duration::from_millis(120);

#[derive(Default)]
pub struct Watchers(Mutex<HashMap<String, Debouncer<RecommendedWatcher, RecommendedCache>>>);

/// Starts watching `path` for the window `label`, replacing any previous watch.
pub fn watch(app: &AppHandle, label: &str, path: &Path) -> notify::Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| notify::Error::generic("document has no parent folder"))?;
    let target = path.to_path_buf();
    let handler_app = app.clone();
    let handler_label = label.to_string();

    let mut debouncer = new_debouncer(DEBOUNCE, None, move |result: DebounceEventResult| {
        let Ok(events) = result else { return };
        let touches_target = events
            .iter()
            .any(|event| event.paths.iter().any(|p| is_same_file(p, &target)));
        if touches_target {
            on_change(&handler_app, &handler_label, &target);
        }
    })?;
    debouncer.watch(dir, RecursiveMode::NonRecursive)?;

    app.state::<Watchers>()
        .0
        .lock()
        .unwrap()
        .insert(label.to_string(), debouncer);
    Ok(())
}

pub fn unwatch(app: &AppHandle, label: &str) {
    app.state::<Watchers>().0.lock().unwrap().remove(label);
}

/// Only the target's folder is watched (non-recursively), so the name is enough.
/// Comparing full paths would trip over `/var` vs `/private/var` style aliases.
fn is_same_file(candidate: &Path, target: &Path) -> bool {
    candidate.file_name() == target.file_name()
}

fn on_change(app: &AppHandle, label: &str, path: &PathBuf) {
    if path.is_file() {
        match read_document(path) {
            Ok(doc) => {
                let _ = app.emit_to(label, "doc:changed", doc);
            }
            Err(message) => log_error(&message),
        }
    } else {
        let _ = app.emit_to(label, "doc:missing", path);
    }
}

fn log_error(message: &str) {
    eprintln!("[mdviewer] watcher: {message}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_file_matches_by_name_only() {
        assert!(is_same_file(
            Path::new("/private/var/x/README.md"),
            Path::new("/var/x/README.md")
        ));
        assert!(!is_same_file(
            Path::new("/var/x/.README.md.swp"),
            Path::new("/var/x/README.md")
        ));
    }
}
