// StoreKit 2 helper for the "unlock favorites" non-consumable IAP.
//
// Compiled + linked ONLY when the Rust `storekit` feature is enabled (see build.rs).
// Exposes a tiny C ABI consumed by src-tauri/src/iap.rs.
//
// NOTE: this is scaffolding — it compiles against StoreKit but has NOT been tested
// against a live App Store product here. Finalize/verify on a real MAS or TestFlight
// build with a sandbox Apple ID. The product id must match App Store Connect.

import Foundation
import StoreKit

private let PRODUCT_ID = "co.amis.myappspot.favorites"

@available(macOS 12.0, *)
private enum IAP {
    // Cache the loaded product so price/purchase don't re-fetch every call.
    static var product: Product?

    static func loadProduct() async -> Product? {
        if let p = product { return p }
        product = try? await Product.products(for: [PRODUCT_ID]).first
        return product
    }

    /// Is the non-consumable currently owned? (verified current entitlement)
    static func isPurchased() async -> Bool {
        for await result in Transaction.currentEntitlements {
            if case .verified(let t) = result, t.productID == PRODUCT_ID, t.revocationDate == nil {
                return true
            }
        }
        return false
    }
}

/// Run an async block to completion synchronously (for the C ABI). Safe here because
/// the calls are short and StoreKit presents its own UI on the main actor.
@available(macOS 12.0, *)
private func blocking<T>(_ op: @escaping () async -> T) -> T {
    let sem = DispatchSemaphore(value: 0)
    var out: T!
    Task.detached { out = await op(); sem.signal() }
    sem.wait()
    return out
}

// MARK: - C ABI (consumed by iap.rs)

/// Owned forever? 1 = yes, 0 = no.
@_cdecl("myappspot_iap_is_purchased")
public func myappspot_iap_is_purchased() -> Bool {
    guard #available(macOS 12.0, *) else { return false }
    return blocking { await IAP.isPurchased() }
}

/// Copy the localized price string (e.g. "$2.99") into `buf`; returns bytes written
/// (0 if the product isn't available — treated as "not purchasable" by Rust).
@_cdecl("myappspot_iap_price")
public func myappspot_iap_price(_ buf: UnsafeMutablePointer<UInt8>, _ cap: Int) -> Int {
    guard #available(macOS 12.0, *) else { return 0 }
    let price = blocking { () -> String? in
        await IAP.loadProduct()?.displayPrice
    }
    guard let price, let bytes = price.data(using: .utf8), bytes.count <= cap else { return 0 }
    bytes.copyBytes(to: buf, count: bytes.count)
    return bytes.count
}

/// Purchase: 0 ok, 1 cancelled, 2 error, 3 unavailable.
@_cdecl("myappspot_iap_purchase")
public func myappspot_iap_purchase() -> Int32 {
    guard #available(macOS 12.0, *) else { return 3 }
    return blocking { () -> Int32 in
        guard let product = await IAP.loadProduct() else { return 3 }
        do {
            switch try await product.purchase() {
            case .success(let verification):
                if case .verified(let t) = verification { await t.finish(); return 0 }
                return 2
            case .userCancelled: return 1
            case .pending: return 2
            @unknown default: return 2
            }
        } catch {
            return 2
        }
    }
}

/// Restore: 0 restored, 1 nothing to restore, 2 error, 3 unavailable.
@_cdecl("myappspot_iap_restore")
public func myappspot_iap_restore() -> Int32 {
    guard #available(macOS 12.0, *) else { return 3 }
    return blocking { () -> Int32 in
        do { try await AppStore.sync() } catch { return 2 }
        return await IAP.isPurchased() ? 0 : 1
    }
}
