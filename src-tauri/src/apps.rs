//! App enumeration via filesystem scan + Info.plist parsing.
//! Pure-Rust and unit-testable (the 0005 spike proved dir reads work under sandbox).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AppEntry {
    pub name: String,
    pub path: String,
    pub bundle_id: Option<String>,
}

/// Standard macOS application locations. `~/Applications` is appended at runtime.
const APP_DIRS: &[&str] = &[
    "/Applications",
    "/Applications/Utilities",
    "/System/Applications",
    "/System/Applications/Utilities",
];

/// The full set of directories scanned: standard system locations + `~/Applications`
/// + user-granted folders. Shared by `enumerate` and the folder watcher (0014).
pub fn dirs_to_scan(extra_dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = APP_DIRS.iter().map(PathBuf::from).collect();
    if let Some(home) = std::env::var_os("HOME") {
        // Under the sandbox, $HOME is the app container, so `~/Applications` points at
        // a useless `<container>/Data/Applications`. Only scan it when not sandboxed;
        // the real `~/Applications` is reached via a user grant (plan 0013/0016).
        if !home.to_string_lossy().contains("/Library/Containers/") {
            dirs.push(Path::new(&home).join("Applications"));
        }
    }
    dirs.extend(extra_dirs.iter().cloned());
    dirs
}

/// Enumerate installed apps, de-duplicated by path and sorted by display name.
/// `extra_dirs` are user-granted folders (plan 0013) scanned in addition to the
/// standard system locations.
pub fn enumerate(extra_dirs: &[PathBuf]) -> Vec<AppEntry> {
    enumerate_with(extra_dirs, |_, _| {})
}

/// Like `enumerate`, but calls `on_dir(dir, total_found_so_far)` after each scanned
/// directory — used to emit indexing progress (plan 0015).
pub fn enumerate_with<F: FnMut(&Path, usize)>(
    extra_dirs: &[PathBuf],
    mut on_dir: F,
) -> Vec<AppEntry> {
    let dirs = dirs_to_scan(extra_dirs);

    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for dir in dirs {
        scan_dir(&dir, &mut out, &mut seen, 0);
        on_dir(&dir, out.len());
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

/// Recurse into plain folders up to `max_depth`, but never descend into a `.app`.
fn scan_dir(dir: &Path, out: &mut Vec<AppEntry>, seen: &mut HashSet<String>, depth: u8) {
    const MAX_DEPTH: u8 = 1;
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_app = path.extension().map(|e| e == "app").unwrap_or(false);
        if is_app {
            if let Some(app) = make_entry(&path) {
                if seen.insert(app.path.clone()) {
                    out.push(app);
                }
            }
        } else if depth < MAX_DEPTH && path.is_dir() {
            scan_dir(&path, out, seen, depth + 1);
        }
    }
}

/// Build an entry from a `.app` bundle, reading its Info.plist for name + bundle id.
fn make_entry(app_path: &Path) -> Option<AppEntry> {
    let path = app_path.to_str()?.to_string();
    let file_stem = app_path.file_stem()?.to_string_lossy().to_string();

    // Prefer the bundle's display name, but fall back to the file name whenever it's
    // missing OR blank (plan 0016 issue 1).
    let (name, bundle_id) = read_info_plist(app_path)
        .map(|(n, b)| (n, b))
        .unwrap_or((None, None));
    let name = name
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| file_stem.clone());

    Some(AppEntry {
        name,
        path,
        bundle_id,
    })
}

/// Returns (display_name, bundle_id) from Contents/Info.plist.
fn read_info_plist(app_path: &Path) -> Option<(Option<String>, Option<String>)> {
    let plist_path = app_path.join("Contents/Info.plist");
    let value = plist::Value::from_file(&plist_path).ok()?;
    let dict = value.as_dictionary()?;
    let name = dict
        .get("CFBundleDisplayName")
        .or_else(|| dict.get("CFBundleName"))
        .and_then(|v| v.as_string())
        .map(str::to_string);
    let bundle_id = dict
        .get("CFBundleIdentifier")
        .and_then(|v| v.as_string())
        .map(str::to_string);
    Some((name, bundle_id))
}

/// Load a previously-saved index from disk (0014). Returns None if missing/corrupt.
pub fn load_cache(file: &Path) -> Option<Vec<AppEntry>> {
    let bytes = std::fs::read(file).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Persist the index to disk so the next launch can paint instantly.
pub fn save_cache(file: &Path, apps: &[AppEntry]) {
    if let Some(parent) = file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(bytes) = serde_json::to_vec(apps) {
        let _ = std::fs::write(file, bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enumerates_system_apps() {
        let apps = enumerate(&[]);
        // Every macOS install has these; if enumeration works at all, they appear.
        assert!(!apps.is_empty(), "expected some apps");
        let names: Vec<_> = apps.iter().map(|a| a.name.to_lowercase()).collect();
        assert!(
            names.iter().any(|n| n.contains("calculator")),
            "expected Calculator in {names:?}"
        );
    }

    #[test]
    fn cache_round_trips() {
        let apps = vec![AppEntry {
            name: "Calculator".into(),
            path: "/System/Applications/Calculator.app".into(),
            bundle_id: Some("com.apple.calculator".into()),
        }];
        let file = std::env::temp_dir().join("mas-index-cache-test/index.json");
        save_cache(&file, &apps);
        let loaded = load_cache(&file).expect("cache should load");
        assert_eq!(loaded, apps);
        let _ = std::fs::remove_dir_all(file.parent().unwrap());
    }

    #[test]
    fn entries_are_sorted_and_unique() {
        let apps = enumerate(&[]);
        let mut paths = HashSet::new();
        for a in &apps {
            assert!(paths.insert(a.path.clone()), "duplicate path {}", a.path);
            assert!(a.path.ends_with(".app"));
        }
        let mut sorted = apps.clone();
        sorted.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        let a: Vec<_> = apps.iter().map(|x| &x.name).collect();
        let b: Vec<_> = sorted.iter().map(|x| &x.name).collect();
        assert_eq!(a, b, "enumerate() should return sorted entries");
    }
}
