//! `mdvres://localhost/<name>` serves a few large files that ship once in the
//! bundle instead of being built into the program, which a universal app
//! would store twice. Only the files listed in `path_for` are served.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use tauri::http::{Request, Response, StatusCode, header};

pub const SCHEME: &str = "mdvres";

/// Where a served file lives, relative to the bundle's `Contents` folder.
fn relative_path(name: &str) -> Option<&'static str> {
    match name {
        // Mermaid's single-file build lives in the Quick Look extension, which
        // needs it too, so the app and Quick Look share one copy.
        "mermaid.min.js" => {
            Some("PlugIns/MDViewerQuickLook.appex/Contents/Resources/web/mermaid.min.js")
        }
        _ => None,
    }
}

fn path_for(contents: &Path, name: &str) -> Option<PathBuf> {
    relative_path(name).map(|relative| contents.join(relative))
}

/// The bundle's `Contents` folder (the executable is in `Contents/MacOS`).
fn contents_dir() -> Option<PathBuf> {
    Some(
        std::env::current_exe()
            .ok()?
            .parent()?
            .parent()?
            .to_path_buf(),
    )
}

pub fn handle(request: &Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let name = request.uri().path().trim_start_matches('/');
    let file = contents_dir()
        .and_then(|contents| path_for(&contents, name))
        .and_then(|path| std::fs::read(path).ok());
    match file {
        Some(bytes) => Response::builder()
            .header(header::CONTENT_TYPE, "text/javascript; charset=utf-8")
            .body(Cow::Owned(bytes)),
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Cow::Borrowed(&[][..])),
    }
    .expect("static response parts are valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serves_only_listed_files() {
        let contents = Path::new("/Applications/MDViewer.app/Contents");
        assert_eq!(
            path_for(contents, "mermaid.min.js"),
            Some(
                contents
                    .join("PlugIns/MDViewerQuickLook.appex/Contents/Resources/web/mermaid.min.js")
            )
        );
        assert_eq!(path_for(contents, "../../etc/passwd"), None);
        assert_eq!(path_for(contents, "Info.plist"), None);
    }
}
