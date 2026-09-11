# 0011 — App Store build guide (runbook)

Step-by-step to produce a signed `.pkg` and ship Spotlight to the Mac App Store.
Implements the concepts in [0008](0008_sandbox_and_mas_distribution.md); this doc is the
actionable checklist. Do the sections in order.

---

## 0. What you'll produce

`tauri build` → universal `.app` → codesign with **Apple Distribution** → wrap in a
signed `.pkg` (**3rd Party Mac Developer Installer**) → upload to App Store Connect →
submit for review. No notarization step (that's for direct download, not MAS).

## 1. Prerequisites (one-time)

- **Apple Developer Program** membership ($99/yr), with Admin or App Manager role.
- **Xcode** installed and signed in: Xcode → Settings → Accounts → add your Apple ID.
- Command-line tools: `xcrun`, `codesign`, `productbuild` (come with Xcode).

## 2. Values you must fill in

Collect these first, then paste them where the table says. Replace every `<...>`.

| Value | What it looks like | Where to get it | Where it goes |
|-------|--------------------|-----------------|---------------|
| **Team ID** | `Q3SG3DR3CC` (10 chars) | [developer.apple.com](https://developer.apple.com/account) → Membership details | `entitlements.plist` (step 5), signing identity strings |
| **Bundle identifier** | `com.yourname.spotlight` | You choose it; must be globally unique | `src-tauri/tauri.conf.json` → `identifier` (currently `com.example.spotlight`), and the App ID (step 3) |
| **App name** | `Spotlight` (or your unique name) | You choose; store display name can differ | `src-tauri/tauri.conf.json` → `productName`. Note: "Spotlight" may collide on the store — pick a unique marketing name in App Store Connect |
| **Apple Distribution identity** | `Apple Distribution: YOUR NAME (Q3SG3DR3CC)` | Keychain Access → My Certificates, or `security find-identity -v -p codesigning` | env `APPLE_DIST_IDENTITY` (step 6) |
| **Installer identity** | `3rd Party Mac Developer Installer: YOUR NAME (Q3SG3DR3CC)` | Same as above | env `APPLE_INSTALLER_IDENTITY` (step 6) |
| **Provisioning profile** | `Spotlight_MAS.provisionprofile` | developer.apple.com → Profiles (step 4) | `src-tauri/embedded.provisionprofile` + `tauri.conf.json` (step 5) |

Tip: list the signing identities already in your keychain:

```sh
security find-identity -v -p codesigning
```

## 3. Register the App ID

developer.apple.com → **Certificates, IDs & Profiles** → **Identifiers** → **+**:

- Type: **App IDs** → **App**.
- Description: `Spotlight`.
- Bundle ID: **Explicit** = your `<bundle identifier>` (e.g. `com.yourname.spotlight`).
  Must be explicit (not wildcard) for MAS, and match `BUNDLE_ID` in `.env.mas`.
- Capabilities: leave **all unchecked** — this app needs none. (Note: App Sandbox is
  NOT an App ID capability; it's an entitlement applied at signing, already handled by
  `entitlements.template.plist`.) → **Register**.

## 4. Certificates + provisioning profile

**Certificates** (skip any you already have — check Keychain first):

developer.apple.com → **Certificates** → **+**:
- **Apple Distribution** → follow prompts (needs a CSR; Xcode can auto-manage, or
  Keychain Access → Certificate Assistant → Request a Certificate from a CA).
- **Mac Installer Distribution** (gives the "3rd Party Mac Developer Installer" identity).

Download and double-click each to install into the login keychain.

**Provisioning profile:**

developer.apple.com → **Profiles** → **+** → **Mac App Store** (Distribution):
- App ID: the one from step 3.
- Certificate: your Apple Distribution cert.
- Name it (e.g. `Spotlight MAS`), **Generate**, **Download**.
- Move it into the repo as the embedded profile:

```sh
mv ~/Downloads/Spotlight_MAS.provisionprofile \
   src-tauri/embedded.provisionprofile
```

(`*.provisionprofile` is already git-ignored.)

## 5. Fill in ONE file: `.env.mas`

All your values live in a single git-ignored file. The build scripts read it and
generate the identifier override + entitlements automatically — you don't edit
`tauri.conf.json` or `entitlements.plist` by hand.

```sh
cp .env.mas.example .env.mas
```

Then edit `.env.mas` (keep the quotes — it's shell-sourced):

```sh
TEAM_ID="Q3SG3DR3CC"
BUNDLE_ID="com.yourname.spotlight"
APPLE_DIST_IDENTITY="Apple Distribution: YOUR NAME (Q3SG3DR3CC)"
APPLE_INSTALLER_IDENTITY="3rd Party Mac Developer Installer: YOUR NAME (Q3SG3DR3CC)"
```

What each script does with it:
- `scripts/build.sh` writes `src-tauri/tauri.mas.conf.json` (sets `identifier`, embeds
  `embedded.provisionprofile` if present) and passes it to `tauri build --config`.
- `scripts/sign.sh` fills `src-tauri/entitlements.template.plist` → `entitlements.generated.plist`
  (adds `application-identifier` + `team-identifier`) and codesigns with it.
- `scripts/pkg.sh` uses your installer identity to build the `.pkg`.

(The committed `tauri.conf.json`/`entitlements.plist` keep their `com.example.spotlight`
placeholder + app-sandbox-only defaults for local dev — the override only applies to
release builds. Both generated files are git-ignored.)

## 6. Build → sign → package

Make sure `src-tauri/embedded.provisionprofile` is in place (step 4), then one command:

```sh
mise run release   # build.sh → sign.sh → pkg.sh, all reading .env.mas
```

Or step by step: `mise run build`, then `mise run sign`, then `mise run pkg`.
Output: `Spotlight.pkg` in the repo root.

Verify signing before uploading:

```sh
codesign --verify --strict --verbose=2 \
  src-tauri/target/universal-apple-darwin/release/bundle/macos/Spotlight.app
codesign -d --entitlements - \
  src-tauri/target/universal-apple-darwin/release/bundle/macos/Spotlight.app
```

The entitlements dump should show `app-sandbox`, `application-identifier`, and
`team-identifier` matching your profile.

## 7. Create the app record in App Store Connect

[appstoreconnect.apple.com](https://appstoreconnect.apple.com) → **Apps** → **+** →
**New App**:
- Platform: **macOS**.
- Name: your unique store name.
- Bundle ID: pick the one from step 3.
- SKU: any internal string (e.g. `spotlight-001`).
- Fill required metadata (category: Productivity, screenshots, description, privacy).

## 8. Upload the build

**Option A — Transporter (easiest):** install **Transporter** from the Mac App Store,
sign in, drag `Spotlight.pkg` in, **Deliver**.

**Option B — CLI:** create an app-specific password at
[account.apple.com](https://account.apple.com) → Sign-In & Security → App-Specific
Passwords, then:

```sh
xcrun altool --upload-app -t macos -f Spotlight.pkg \
  -u "your-apple-id@example.com" -p "<app-specific-password>"
```

After processing (a few minutes), the build appears under your app's **TestFlight** /
**App Store** build list.

## 9. Submit for review

App Store Connect → your app → create a version → attach the uploaded build → answer
export-compliance (`ITSAppUsesNonExemptEncryption=false` is already set, so "No") →
**Submit for Review**.

---

## Troubleshooting

- **`errSecInternalComponent` / no identity found** — the cert isn't in the login
  keychain, or the name in `APPLE_DIST_IDENTITY` doesn't match exactly. Re-check with
  `security find-identity -v -p codesigning` and copy the string verbatim.
- **"Provisioning profile doesn't match entitlements"** — `application-identifier` /
  `team-identifier` in `entitlements.plist` must equal `<TEAM_ID>.<BUNDLE_ID>` and the
  profile's App ID. Rebuild after fixing.
- **"Invalid signature" on upload** — you signed with Developer ID instead of **Apple
  Distribution**, or nested code is unsigned. This app is a single binary, but if you add
  frameworks/helpers, sign them inside-out before the outer `.app`.
- **Rejected: bundle identifier mismatch** — `identifier` in `tauri.conf.json`, the App
  ID, the provisioning profile, and the App Store Connect record must all be identical.
- **App enumeration/launch questioned in review** — we use only public `NSWorkspace` +
  directory scan with the base sandbox (see 0005). No private APIs, so this should pass.

## Fill-in checklist

- [ ] Team ID collected
- [ ] Bundle identifier chosen
- [ ] App ID registered (App Sandbox enabled)
- [ ] Apple Distribution + Mac Installer Distribution certs installed
- [ ] MAS provisioning profile downloaded → `src-tauri/embedded.provisionprofile`
- [ ] `.env.mas` created + filled (TEAM_ID, BUNDLE_ID, both identities)
- [ ] `mise run release` succeeds → `Spotlight.pkg`
- [ ] `codesign --verify` clean, entitlements dump correct
- [ ] App record created in App Store Connect
- [ ] `Spotlight.pkg` uploaded (Transporter or altool)
- [ ] Build attached to a version + submitted for review

## Unresolved questions

- Store display name? ("Spotlight" likely taken/confusable — need a unique marketing name.)
- Final bundle identifier + Team ID? (blocks steps 3–6.)
