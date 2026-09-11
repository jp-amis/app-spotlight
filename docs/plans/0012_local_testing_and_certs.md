# 0012 — Local testing & signing certificates

How to run/test the app locally (including sandboxed), where to persist your signing
identity, and how to list + issue certificates. For the full Mac App Store release flow
see [0011](0011_app_store_build_guide.md).

---

## Three tiers of local testing

| Tier | Command | Signed? | Sandboxed? | Apple account? |
|------|---------|---------|------------|----------------|
| **1. Fast dev** | `mise run dev` | no | no | none |
| **2. Sandboxed (ad-hoc)** | `mise run sandbox` | ad-hoc | ✅ yes | none |
| **3. Sandboxed (your cert)** | `mise run sandbox` + `.env.local` | your dev cert | ✅ yes | free Apple ID ok |

- **Tier 1** — hot-reloading, best for UI/logic iteration. The global shortcut works with
  no permission grant (Carbon hotkeys, not Accessibility).
- **Tier 2/3** — build a real `.app`, codesign it with the `app-sandbox` entitlement, and
  launch it. This is the truest local test of shipping behavior. The sandbox is enforced
  by the entitlement in the signature regardless of ad-hoc vs a real cert, so you can
  verify sandbox behavior with **no account at all**. Container lands in
  `~/Library/Containers/<bundle-id>/`.

`mise run sandbox` builds a **native debug** bundle (seconds) — much faster than the
universal release build used for the store.

## Persist your signing identity (no `export`)

Put local dev vars in a git-ignored `.env.local` — `mise run sandbox` loads it automatically:

```sh
cp .env.local.example .env.local
```

```sh
# .env.local  (keep the quotes)
APPLE_DEV_IDENTITY="Apple Development: YOUR NAME (TEAMID)"
```

Leave `APPLE_DEV_IDENTITY` unset to sign **ad-hoc** (Tier 2). Set it to sign with your
development cert (Tier 3). Both enforce the sandbox.

> Files: `.env.local` (local dev) is separate from `.env.mas` (release secrets, see 0011).
> Both are git-ignored; `.env.local.example` / `.env.mas.example` are committed templates.

## Listing certificates

```sh
security find-identity -v -p codesigning   # code-signing certs (Apple Development/Distribution)
security find-identity -v                   # ALL identities (incl. the installer cert)
```

The installer cert ("3rd Party Mac Developer Installer") does **not** appear under
`-p codesigning` — use the second command to see it. Copy the quoted string verbatim into
`.env.local` / `.env.mas`.

## Issuing certificates

### Option A — Xcode-managed (easiest)

Xcode → **Settings → Accounts** → add your Apple ID → select the team →
**Manage Certificates…** → **＋** → pick the type. Xcode generates the CSR, creates the
cert, and installs it into your keychain.

### Option B — Developer portal (manual)

1. Keychain Access → Certificate Assistant → **Request a Certificate from a Certificate
   Authority** → save the `.certSigningRequest` to disk.
2. developer.apple.com → **Certificates** → **＋** → choose type → upload the CSR →
   download the `.cer` → double-click to install.

### Which cert for what

| Cert type | Used for | mise task |
|-----------|----------|-----------|
| **Apple Development** | local dev / sandboxed testing | `mise run sandbox` |
| **Apple Distribution** | MAS app signing | `mise run sign` |
| **Mac Installer Distribution** → "3rd Party Mac Developer Installer" | MAS `.pkg` | `mise run pkg` |
| Developer ID Application/Installer | direct (non-store) distribution only | — |

Notes:
- A **free "Personal Team"** Apple ID can only issue **Apple Development** — enough for
  Tier 3 local testing. The two **Distribution** certs require the paid Developer Program.
- Create the Distribution certs under whichever team will own the app, and keep that Team
  ID consistent in `.env.mas`.
- No provisioning profile is needed for local signing because the only entitlement is
  `app-sandbox` (nothing restricted).

## Quick reference

```sh
mise run dev        # tier 1: unsigned, hot reload
mise run sandbox    # tier 2/3: sandboxed local .app (uses .env.local if present)
mise run release    # store: build → sign → pkg (uses .env.mas)  — see 0011
```
