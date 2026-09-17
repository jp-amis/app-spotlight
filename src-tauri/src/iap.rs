//! In-app purchase entitlement for the Favorites feature.
//!
//! Two ways to unlock favorites:
//!   * BUY the non-consumable IAP  -> unlocked forever (persisted; StoreKit is the
//!     source of truth on App Store builds).
//!   * FREE "unlock for this session" -> unlocked until the app quits (in-memory).
//!
//! Real StoreKit lives behind the off-by-default `storekit` cargo feature (Mac App
//! Store builds — see src-tauri/swift/iap.swift). The DEFAULT build (incl.
//! `mise run sandbox`) uses the stub below: purchasing is reported unavailable, so
//! the Buy button is disabled, but the free session unlock still works.

use crate::i18n::Key;

/// A localizable IAP failure. Kept language-agnostic here; the command layer maps
/// it to a localized string via [`IapError::key`] + `crate::i18n`. Most variants are
/// only constructed under the `storekit` feature (the stub build only ever returns
/// `StoreOnly`), hence the allow.
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub enum IapError {
    Cancelled,
    Unavailable,
    PurchaseFailed,
    RestoreFailed,
    StoreOnly,
}

impl IapError {
    /// The i18n key for this error's user-facing message.
    pub fn key(self) -> Key {
        match self {
            IapError::Cancelled => Key::IapCancelled,
            IapError::Unavailable => Key::IapUnavailable,
            IapError::PurchaseFailed => Key::IapPurchaseFailed,
            IapError::RestoreFailed => Key::IapRestoreFailed,
            IapError::StoreOnly => Key::IapStoreOnly,
        }
    }
}

/// StoreKit-backed implementation (App Store builds only).
#[cfg(feature = "storekit")]
mod backend {
    use super::IapError;

    extern "C" {
        fn myappspot_iap_is_purchased() -> bool;
        fn myappspot_iap_price(buf: *mut u8, cap: usize) -> usize;
        fn myappspot_iap_purchase() -> i32; // 0 ok, 1 cancelled, 2 error, 3 unavailable
        fn myappspot_iap_restore() -> i32; // 0 restored, 1 nothing, 3 unavailable, 2 error
    }

    pub fn price() -> Option<String> {
        let mut buf = [0u8; 64];
        let n = unsafe { myappspot_iap_price(buf.as_mut_ptr(), buf.len()) };
        if n == 0 {
            None
        } else {
            String::from_utf8(buf[..n.min(buf.len())].to_vec()).ok()
        }
    }
    pub fn purchasable() -> bool {
        price().is_some()
    }
    pub fn is_purchased() -> bool {
        unsafe { myappspot_iap_is_purchased() }
    }
    pub fn purchase() -> Result<(), IapError> {
        match unsafe { myappspot_iap_purchase() } {
            0 => Ok(()),
            1 => Err(IapError::Cancelled),
            3 => Err(IapError::Unavailable),
            _ => Err(IapError::PurchaseFailed),
        }
    }
    pub fn restore() -> Result<bool, IapError> {
        match unsafe { myappspot_iap_restore() } {
            0 => Ok(true),
            1 => Ok(false),
            3 => Err(IapError::Unavailable),
            _ => Err(IapError::RestoreFailed),
        }
    }
}

/// Stub used by non-App-Store builds (default, and `mise run sandbox`).
#[cfg(not(feature = "storekit"))]
mod backend {
    use super::IapError;

    pub fn price() -> Option<String> {
        None
    }
    pub fn purchasable() -> bool {
        false
    }
    pub fn is_purchased() -> bool {
        false
    }
    pub fn purchase() -> Result<(), IapError> {
        Err(IapError::StoreOnly)
    }
    pub fn restore() -> Result<bool, IapError> {
        Err(IapError::StoreOnly)
    }
}

pub use backend::*;
