# Distribution

Two independent release paths. They use different certificates, entitlements, and
outputs — keep both config files (`.env.dist`, `.env.mas`) side by side.

| | Developer ID (direct) | Mac App Store |
|---|---|---|
| Use for | Sending a `.dmg` to friends/testers | Public release via the App Store |
| Command | `mise run dist` | `mise run release` |
| App signing cert | **Developer ID Application** | **Apple Distribution** |
| Installer cert | — | **3rd Party Mac Developer Installer** |
| Provisioning profile | none | required (`src-tauri/embedded.provisionprofile`) |
| Entitlements | `entitlements.plist` | `entitlements.template.plist` (adds `application-identifier` + `team-identifier`) |
| Notarization | yes (notarytool, automatic) | no — App Review handles it |
| Output | `.dmg` (double-click to install) | `.pkg` (upload to App Store Connect) |
| Config file | `.env.dist` | `.env.mas` |

Bundle id: `co.amis.myappspot`. Both builds are sandboxed and universal
(Intel + Apple Silicon), min macOS 13.

> The `network.client` entitlement is kept in both — WKWebView needs it to load the
> local frontend under the sandbox (verified: removing it renders a blank window).
> The app makes no network requests itself.

---

## Developer ID — send to friends (`mise run dist`)

Produces a signed, notarized, stapled universal `.dmg`. Friends double-click, drag to
Applications, and launch with no Gatekeeper warnings on any Mac ≥ 13.

**One-time setup**

1. Install a **Developer ID Application** certificate (Xcode → Settings → Accounts →
   Manage Certificates → +, or developer.apple.com → Certificates). Confirm:
   ```
   security find-identity -v -p codesigning
   ```
2. Create an app-specific password: appleid.apple.com → Sign-In & Security →
   App-Specific Passwords.
3. Configure:
   ```
   cp .env.dist.example .env.dist
   ```
   Fill in `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD` (the app-specific
   one), `APPLE_TEAM_ID`. (`.env.dist` is git-ignored.)

**Build**

```
mise run dist
```

Output: `src-tauri/target/universal-apple-darwin/release/bundle/dmg/My App Spot_*_universal.dmg`.
Tauri signs (hardened runtime + our entitlements), notarizes, and staples automatically
once the `APPLE_*` vars are present.

**Verify (optional)**

```
spctl -a -vvv "…/My App Spot.app"     # should say: accepted, Notarized Developer ID
xcrun stapler validate "…/*.dmg"
```

---

## Mac App Store (`mise run release`)

Runs `scripts/build.sh` → `sign.sh` → `pkg.sh`, producing a signed `My App Spot.pkg`
for App Store Connect.

**One-time setup**

1. Register the App ID `co.amis.myappspot` (with App Sandbox) at developer.apple.com →
   Identifiers, and create the app record in App Store Connect.
2. Create a **Mac App Store** provisioning profile for that App ID, download it, and
   save it as `src-tauri/embedded.provisionprofile` (`build.sh` embeds it if present).
3. Configure:
   ```
   cp .env.mas.example .env.mas
   ```
   Fill in `TEAM_ID`, `BUNDLE_ID` (`co.amis.myappspot`),
   `APPLE_DIST_IDENTITY="Apple Distribution: … (TEAMID)"`,
   `APPLE_INSTALLER_IDENTITY="3rd Party Mac Developer Installer: … (TEAMID)"`.
   Get exact strings from `security find-identity -v -p codesigning`.

**Build**

```
mise run release      # → My App Spot.pkg
```

**Upload**

- Easiest: open the **Transporter** app (free, on the Mac App Store) and drag in the
  `.pkg`, or
  ```
  xcrun altool --upload-app -f "My App Spot.pkg" -t macos \
    -u "$APPLE_ID" -p "$APP_SPECIFIC_PASSWORD"
  ```
- Then submit for review in App Store Connect (screenshots, description, privacy — the
  app collects no data and makes no network requests).

**TestFlight:** once a build is processed in App Store Connect you can distribute it to
testers via TestFlight (managed installs + auto-updates) without a full public release.

---

## Notes

- `mise run sandbox` is the **local dev** build: universal debug, ad-hoc signed, run in
  place. Good for testing sandbox behavior, but `SMAppService` ("Open at login") is
  strict about signature and may not register under ad-hoc — it works under Developer ID
  and MAS builds.
- Related plan: [`docs/plans/0008_sandbox_and_mas_distribution.md`](plans/0008_sandbox_and_mas_distribution.md).
