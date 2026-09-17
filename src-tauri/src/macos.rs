//! Thin macOS/objc2 layer: icon extraction, app launch, and launcher-window tuning.
//! All public APIs (NSWorkspace) — the 0005 spike proved these work under App Sandbox.

#![cfg(target_os = "macos")]

use std::ffi::c_void;

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, NSObjectProtocol};
use objc2::{msg_send, AllocAnyThread, ClassType, MainThreadMarker};
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSApplication, NSAutoresizingMaskOptions, NSBitmapImageFileType, NSBitmapImageRep,
    NSGlassEffectView, NSGlassEffectViewStyle, NSGraphicsContext, NSImage, NSWindow,
    NSWindowCollectionBehavior, NSWindowOrderingMode, NSWorkspace,
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

/// Bring the app to the front. Menu-bar (accessory) apps don't activate when a
/// window is shown, so a settings window can open behind other apps unless we
/// explicitly activate. MUST run on the main thread.
pub fn activate_app() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    #[allow(deprecated)]
    app.activateIgnoringOtherApps(true);
}

/// Open a URL (e.g. an `x-apple.systempreferences:` deep link) via NSWorkspace.
pub fn open_url(url: &str) -> bool {
    let ws = NSWorkspace::sharedWorkspace();
    match NSURL::URLWithString(&NSString::from_str(url)) {
        Some(nsurl) => {
            #[allow(deprecated)]
            ws.openURL(&nsurl)
        }
        None => false,
    }
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

/// Is Liquid Glass (`NSGlassEffectView`, macOS 26+) available at runtime?
pub fn has_liquid_glass() -> bool {
    AnyClass::get(c"NSGlassEffectView").is_some()
}

/// Add a Liquid Glass backing (`NSGlassEffectView`) behind the transparent webview.
/// `clear` picks the more-transparent Clear style over Regular. Returns false if
/// unsupported (pre-macOS 26) so callers can fall back to vibrancy.
/// MUST run on the main thread.
pub fn apply_liquid_glass(ns_window: *mut c_void, radius: f64, clear: bool) -> bool {
    if ns_window.is_null() || !has_liquid_glass() {
        return false;
    }
    let Some(mtm) = MainThreadMarker::new() else {
        return false;
    };
    let window: &NSWindow = unsafe { &*(ns_window as *const NSWindow) };
    let Some(content) = window.contentView() else {
        return false;
    };
    clear_glass(ns_window); // never stack glass views
    let bounds = content.bounds();
    let glass = NSGlassEffectView::initWithFrame(mtm.alloc(), bounds);
    glass.setCornerRadius(radius);
    glass.setStyle(if clear {
        NSGlassEffectViewStyle::Clear
    } else {
        NSGlassEffectViewStyle::Regular
    });
    glass.setAutoresizingMask(
        NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable,
    );
    // Behind the webview (which is transparent), so the glass shows through.
    content.addSubview_positioned_relativeTo(&glass, NSWindowOrderingMode::Below, None);
    true
}

/// Clip the launcher's content view to a rounded rectangle. Regular Liquid Glass
/// draws a bright specular rim at its edge that reads as a hard border; clipping
/// the container to the same radius trims that rim. MUST run on the main thread.
pub fn round_launcher_content(ns_window: *mut c_void, radius: f64) {
    if ns_window.is_null() {
        return;
    }
    let window: &NSWindow = unsafe { &*(ns_window as *const NSWindow) };
    let Some(content) = window.contentView() else {
        return;
    };
    unsafe {
        let _: () = msg_send![&*content, setWantsLayer: true];
        let layer: *mut AnyObject = msg_send![&*content, layer];
        if !layer.is_null() {
            let _: () = msg_send![layer, setCornerRadius: radius];
            let _: () = msg_send![layer, setMasksToBounds: true];
        }
    }
}

/// Remove any Liquid Glass backing views we previously added (for switching styles).
/// MUST run on the main thread.
pub fn clear_glass(ns_window: *mut c_void) {
    if ns_window.is_null() || !has_liquid_glass() {
        return;
    }
    let window: &NSWindow = unsafe { &*(ns_window as *const NSWindow) };
    let Some(content) = window.contentView() else {
        return;
    };
    let glass_cls = NSGlassEffectView::class();
    for v in content.subviews().iter() {
        if v.isKindOfClass(glass_cls) {
            v.removeFromSuperview();
        }
    }
}

// ---------- launch at login (SMAppService, macOS 13+) ----------

// Link the framework so the SMAppService class is registered at runtime.
#[link(name = "ServiceManagement", kind = "framework")]
extern "C" {}

/// Is the app registered to open at login? (`SMAppServiceStatusEnabled` == 1.)
pub fn login_item_enabled() -> bool {
    let Some(cls) = AnyClass::get(c"SMAppService") else {
        return false;
    };
    unsafe {
        let service: *mut AnyObject = msg_send![cls, mainAppService];
        if service.is_null() {
            return false;
        }
        let status: isize = msg_send![service, status];
        status == 1
    }
}

/// A localizable "open at login" failure. The `Os` variant already carries the
/// system's localized description; the others map to i18n keys at the command layer.
pub enum LoginError {
    NeedsMacos13,
    ServiceUnavailable,
    Unknown,
    Os(String),
}

impl LoginError {
    /// The i18n key for the non-OS variants (unused for `Os`, which is pre-localized).
    pub fn key(&self) -> crate::i18n::Key {
        use crate::i18n::Key;
        match self {
            LoginError::NeedsMacos13 => Key::LoginNeedsMacos13,
            LoginError::ServiceUnavailable => Key::LoginServiceUnavailable,
            _ => Key::LoginUnknownError,
        }
    }
}

/// Enable/disable "open at login" via `SMAppService.mainApp`. Requires macOS 13+.
pub fn set_login_item(enabled: bool) -> Result<(), LoginError> {
    let Some(cls) = AnyClass::get(c"SMAppService") else {
        return Err(LoginError::NeedsMacos13);
    };
    unsafe {
        let service: *mut AnyObject = msg_send![cls, mainAppService];
        if service.is_null() {
            return Err(LoginError::ServiceUnavailable);
        }
        let mut err: *mut AnyObject = std::ptr::null_mut();
        let ok: bool = if enabled {
            msg_send![service, registerAndReturnError: &mut err]
        } else {
            msg_send![service, unregisterAndReturnError: &mut err]
        };
        if ok {
            return Ok(());
        }
        if err.is_null() {
            return Err(LoginError::Unknown);
        }
        let desc: *mut NSString = msg_send![err, localizedDescription];
        if desc.is_null() {
            Err(LoginError::Unknown)
        } else {
            Err(LoginError::Os((*desc).to_string()))
        }
    }
}

