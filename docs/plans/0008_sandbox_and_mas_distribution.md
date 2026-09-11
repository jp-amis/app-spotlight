# 0008 — Sandbox & MAS distribution

## Goal

Ship a sandboxed, signed, App-Store-acceptable build.

## Entitlements (`entitlements.plist`)

Start minimal, add only what the 0005 spike proves necessary:

```
com.apple.security.app-sandbox            = true
com.apple.application-identifier          = <TEAMID>.<bundle-id>
com.apple.developer.team-identifier       = <TEAMID>
```

**App logic needs only `app-sandbox`** (0005 spike: enumeration, icon extraction, launch). No temporary-exception, no file entitlements.

**BUT the WKWebView needs `com.apple.security.network.client`** to load the (local) frontend under the sandbox — without it the webview stays blank and the app shows an empty window. Discovered 2026-08-29 when the sandboxed build rendered nothing; the reference app kept this entitlement for the same reason. The app makes no actual network requests. So the required set is:

```
com.apple.security.app-sandbox                   = true
com.apple.security.network.client                = true   # WKWebView content loading
com.apple.security.files.user-selected.read-only = true   # user-granted folders (0013)
com.apple.security.files.bookmarks.app-scope     = true   # persist those grants
com.apple.application-identifier                  = <TEAMID>.<bundle-id>   # (signing)
com.apple.developer.team-identifier               = <TEAMID>              # (signing)
```

Reference `entitlements.plist` also had iCloud/CloudKit — NOT needed here; omit.

## Provisioning

- Create App ID + Mac App Store provisioning profile in Apple Developer portal.
- Embed as `embedded.provisionprofile` via `tauri.conf.json` `bundle.macOS.files` (as reference does).

## Signing & packaging (from reference `build.sh`)

```
cargo tauri build --target universal-apple-darwin
codesign --sign "Apple Distribution: <NAME> (<TEAMID>)" \
  --entitlements ./entitlements.plist \
  ./target/universal-apple-darwin/release/bundle/macos/<App>.app
xcrun productbuild --sign "3rd Party Mac Developer Installer: <NAME> (<TEAMID>)" \
  --component ./.../<App>.app /Applications/ <App>.pkg
```

Then upload `.pkg` via Transporter / `xcrun altool`/`notarytool` path for MAS.

## Info.plist

- `ITSAppUsesNonExemptEncryption = false` (as reference), assuming no custom crypto.
- `LSUIElement = true` if shipping as menu-bar/accessory app (0007) — or set ActivationPolicy in code.
- Usage-description strings only if an API demands one.

## Tasks

- [ ] Minimal `entitlements.plist`.
- [ ] Register App ID + MAS provisioning profile; embed it.
- [ ] Port `build.sh` sign+pkg to `mise run build`/`sign`/`pkg`.
- [ ] First TestFlight/App Store Connect upload dry-run.
- [ ] Confirm sandbox container doesn't break app enum/launch (ties to 0005 spike).

## Risks

- App enumeration/launch: **de-risked** — 0005 spike proved it works sandboxed with public `NSWorkspace` + directory scan only. `LSApplicationWorkspace` avoided.
- Universal build signing pitfalls (per-arch, hardened runtime not needed for MAS but entitlements must match profile).

## Unresolved questions

- Apple Team ID + signing identities available? (needed to build/sign)
- Final bundle identifier?
- ~~Is `LSApplicationWorkspace` acceptable~~ — RESOLVED: not needed, staying on public NSWorkspace.
