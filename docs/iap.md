# In-app purchase — "Unlock Favorites"

Favorites/pins are a paid, optional feature framed as *supporting the developer*.
There are two ways to unlock them:

- **Buy** the non-consumable IAP → unlocked **forever**.
- **Free** → "Unlock for this session" → unlocked until the app quits (the menu-bar
  app runs continuously, so this lasts your whole session).

Everything works in the normal / `mise run sandbox` build **except** the real
purchase — there the Buy button is disabled ("Available in the App Store version")
and the free session unlock is used instead.

## How it's wired

- **Entitlement**: `favorites_unlocked = purchased_forever || session_unlocked`
  (`src-tauri/src/commands.rs`). `session_unlocked` is an in-memory `AppState.fav_session`
  (resets on restart). `purchased_forever` = StoreKit entitlement, mirrored into the
  `favorites_purchased` key in `settings.json`.
- **Commands**: `favorites_status`, `unlock_favorites_session`, `purchase_favorites`,
  `restore_favorites`. `set_favorites` refuses when locked (defense in depth).
- **Gating**: the launcher only applies pins when unlocked (`Launcher.tsx`
  `refreshFavorites`); the Settings → Favorites tab shows a paywall until unlocked.
- **StoreKit** lives in `src-tauri/src/iap.rs`:
  - default build → stub (`purchasable()=false`, `purchase()=Err`).
  - `--features storekit` → calls the Swift helper `src-tauri/swift/iap.swift`
    (StoreKit 2), compiled + linked by `src-tauri/build.rs`.

## App Store Connect setup

1. App Store Connect → your app → **In-App Purchases** → **+** → **Non-Consumable**.
2. Product ID: **`co.amis.myappspot.favorites`** (must match `PRODUCT_ID` in
   `swift/iap.swift`). Set a price tier, display name, and description; add the
   required localization and review screenshot.
3. Submit the IAP with the app build (a non-consumable must ship alongside a build).

## Building with StoreKit

```
pnpm tauri build --target universal-apple-darwin --features storekit
```

For the MAS pipeline, add `--features storekit` to `scripts/build.sh`'s
`pnpm tauri build` invocation (or your `.env.mas` flow) so store builds include it.
Requires the Swift toolchain (`swiftc`, bundled with Xcode / Command Line Tools).

## Testing purchases

StoreKit can't be exercised in `mise run sandbox` (not an App Store build). To test:

- Upload a `--features storekit` build to **TestFlight** / App Store Connect and buy
  with a **sandbox Apple ID** (App Store Connect → Users and Access → Sandbox), **or**
- Add a StoreKit configuration file in Xcode for local testing (requires wrapping the
  app in an Xcode scheme — heavier; TestFlight is simpler here).

Verify: buy → favorites unlock and persist across relaunch; **Restore purchase**
re-grants on a fresh install; "Unlock for this session" works without buying and
resets after quitting.

> `swift/iap.swift` is scaffolding that compiles + links against StoreKit but has not
> been validated against a live product. Confirm the purchase/restore flows on a real
> build before shipping.
