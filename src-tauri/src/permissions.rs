//! Sandbox folder-access grants (plan 0013 issue 4).
//!
//! Under the App Sandbox, `$HOME` is redirected to the app container, so the user's
//! real `~/Applications` (and other folders) are invisible. We let the user grant a
//! folder via `NSOpenPanel` and persist access across launches with an app-scoped
//! security bookmark.

#![cfg(target_os = "macos")]

use std::path::PathBuf;

use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSOpenPanel};
use objc2_foundation::{
    NSData, NSString, NSURLBookmarkCreationOptions, NSURLBookmarkResolutionOptions, NSURL,
};

/// The user's real home directory, even under the sandbox (where `$HOME` points at
/// the container). Derived by stripping the `/Library/Containers/<id>/Data` suffix.
pub fn real_home() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let s = home.to_string_lossy();
    if let Some(idx) = s.find("/Library/Containers/") {
        return Some(PathBuf::from(&s[..idx]));
    }
    Some(PathBuf::from(home))
}

/// Show a folder picker (must run on the main thread) and, on selection, return the
/// chosen path plus a security-scoped bookmark blob to persist. `prompt` and
/// `message` are the (already-localized) panel button + description text.
pub fn pick_folder(prompt: &str, message: &str) -> Option<(String, Vec<u8>)> {
    let mtm = MainThreadMarker::new()?;

    // Accessory app: bring it forward so the panel is visible + frontmost.
    let app = NSApplication::sharedApplication(mtm);
    #[allow(deprecated)]
    app.activateIgnoringOtherApps(true);

    let panel = NSOpenPanel::openPanel(mtm);
    panel.setCanChooseDirectories(true);
    panel.setCanChooseFiles(false);
    panel.setAllowsMultipleSelection(false);
    panel.setPrompt(Some(&NSString::from_str(prompt)));
    panel.setMessage(Some(&NSString::from_str(message)));

    // Default the panel to the real ~/Applications if we can name it.
    if let Some(home) = real_home() {
        let apps = home.join("Applications");
        let url = NSURL::fileURLWithPath(&NSString::from_str(&apps.to_string_lossy()));
        panel.setDirectoryURL(Some(&url));
    }

    let response = panel.runModal();
    if response != 1 {
        return None; // not NSModalResponseOK
    }
    let url: Retained<NSURL> = panel.URL()?;

    let bookmark: Retained<NSData> = url
        .bookmarkDataWithOptions_includingResourceValuesForKeys_relativeToURL_error(
            NSURLBookmarkCreationOptions::WithSecurityScope,
            None,
            None,
        )
        .ok()?;

    let path = url.path()?.to_string();
    Some((path, bookmark.to_vec()))
}

/// Resolve a stored bookmark, begin accessing the security-scoped resource, and return
/// its path. The resolved URL is leaked so access persists for the app's lifetime
/// (the sandbox releases it on exit).
pub fn resolve_and_access(bookmark: &[u8]) -> Option<String> {
    let data = NSData::with_bytes(bookmark);
    let mut stale = objc2::runtime::Bool::NO;
    let url: Retained<NSURL> = unsafe {
        NSURL::URLByResolvingBookmarkData_options_relativeToURL_bookmarkDataIsStale_error(
            &data,
            NSURLBookmarkResolutionOptions::WithSecurityScope,
            None,
            &mut stale,
        )
    }
    .ok()?;

    if !unsafe { url.startAccessingSecurityScopedResource() } {
        return None;
    }
    let path = url.path()?.to_string();
    std::mem::forget(url); // keep the security scope open for the session
    Some(path)
}
