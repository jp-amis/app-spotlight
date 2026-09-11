//! Thin macOS/objc2 layer: icon extraction, app launch, and launcher-window tuning.
//! All public APIs (NSWorkspace) — the 0005 spike proved these work under App Sandbox.

#![cfg(target_os = "macos")]

use std::ffi::c_void;

use objc2::rc::Retained;
use objc2::AllocAnyThread;
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSBitmapImageFileType, NSBitmapImageRep, NSGraphicsContext, NSImage, NSWindow,
    NSWindowCollectionBehavior, NSWorkspace,
};
use window_vibrancy::NSVisualEffectMaterial;
use objc2_foundation::{NSDictionary, NSPoint, NSRect, NSSize, NSString, NSURL};

/// Extract an app icon and render it to a `size`x`size` PNG.
///
/// MUST run on the main thread — AppKit icon + drawing APIs are main-thread-only
/// (the `app_icon` command dispatches here). We render via `NSBitmapImageRep`
/// rather than decoding `TIFFRepresentation`, because modern macOS icons are
/// 16-bit float TIFF that Rust image decoders reject (and are ~74 MB besides).
pub fn icon_png(path: &str, size: u32) -> Option<Vec<u8>> {
    let ws = NSWorkspace::sharedWorkspace();
    let image: Retained<NSImage> = ws.iconForFile(&NSString::from_str(path));

    let px = size as isize;
    // Empty 8-bit RGBA bitmap at exactly size×size.
    let rep = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            std::ptr::null_mut(),
            px,
            px,
            8,
            4,
            true,
            false,
            &NSString::from_str("NSDeviceRGBColorSpace"),
            0,
            0,
        )
    }?;

    // Draw the icon (best representation, scaled) into the bitmap.
    let ctx = NSGraphicsContext::graphicsContextWithBitmapImageRep(&rep)?;
    NSGraphicsContext::saveGraphicsState_class();
    NSGraphicsContext::setCurrentContext(Some(&ctx));
    let rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(size as f64, size as f64));
    image.drawInRect(rect);
    NSGraphicsContext::restoreGraphicsState_class();

    let props = NSDictionary::new();
    let data = unsafe { rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &props) }?;
    Some(data.to_vec())
}

/// Launch an app by its bundle path via NSWorkspace. Returns true on success.
pub fn launch(path: &str) -> bool {
    let ws = NSWorkspace::sharedWorkspace();
    let url = NSURL::fileURLWithPath(&NSString::from_str(path));
    #[allow(deprecated)]
    ws.openURL(&url)
}

/// Raise the launcher window into a Spotlight-like float: above normal windows,
/// visible on every Space, and present over fullscreen apps.
///
/// `ns_window` is the raw `*mut NSWindow` from `WebviewWindow::ns_window()`.
pub fn tune_launcher_window(ns_window: *mut c_void) {
    if ns_window.is_null() {
        return;
    }
    let window: &NSWindow = unsafe { &*(ns_window as *const NSWindow) };

    // NSStatusWindowLevel (25): sits above normal + floating windows like Spotlight.
    window.setLevel(25);

    let behavior = NSWindowCollectionBehavior::CanJoinAllSpaces
        | NSWindowCollectionBehavior::FullScreenAuxiliary
        | NSWindowCollectionBehavior::Stationary;
    window.setCollectionBehavior(behavior);
}

/// Force a window's appearance (light/dark), or clear it to follow the system.
/// Setting the NSWindow appearance also cascades to the WKWebView inside, so the
/// web content's `prefers-color-scheme` (and thus our `dark:` styles) follows suit.
///
/// `dark`: `Some(true)` = dark, `Some(false)` = light, `None` = follow system.
/// MUST run on the main thread.
pub fn set_appearance(ns_window: *mut c_void, dark: Option<bool>) {
    if ns_window.is_null() {
        return;
    }
    let window: &NSWindow = unsafe { &*(ns_window as *const NSWindow) };
    let appearance = match dark {
        None => None,
        Some(true) => NSAppearance::appearanceNamed(unsafe { NSAppearanceNameDarkAqua }),
        Some(false) => NSAppearance::appearanceNamed(unsafe { NSAppearanceNameAqua }),
    };
    window.setAppearance(appearance.as_deref());
}

/// Is the window's *effective* appearance dark? (Accounts for "follow system".)
/// MUST run on the main thread.
pub fn is_dark(ns_window: *mut c_void) -> bool {
    if ns_window.is_null() {
        return true;
    }
    let window: &NSWindow = unsafe { &*(ns_window as *const NSWindow) };
    window
        .effectiveAppearance()
        .name()
        .to_string()
        .contains("Dark")
}

/// The vibrancy material the launcher should use for a given effective appearance:
/// the dark HUD glass in dark mode, an adaptive light material in light mode.
pub fn launcher_material(dark: bool) -> NSVisualEffectMaterial {
    if dark {
        NSVisualEffectMaterial::HudWindow
    } else {
        NSVisualEffectMaterial::Sidebar
    }
}

