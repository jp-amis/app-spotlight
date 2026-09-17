# App Store Connect listing - fill-in guide (ASO-optimized)

Everything you need to create the app in App Store Connect, with ready-to-paste
copy. Values are written for **ASO**: the App Store search index only reads the
**App Name**, **Subtitle**, and **Keywords** - so those three carry distinct,
high-intent terms and never repeat each other. The Description is for *conversion*
(humans), not search.

Avoid trademarks in indexed fields: **do not** use "Spotlight" (Apple), "Alfred",
"Raycast", etc. - Apple can reject the listing.

---

## 0. Quick reference

| Field | Value |
|---|---|
| Bundle ID | `co.amis.myappspot` (fixed) |
| Apple ID | `6811473829` |
| App Store URL | https://apps.apple.com/us/app/my-app-spot-app-launcher/id6811473829 |
| SKU | `myappspot-mac-001` (any unique string) |
| Primary category | **Productivity** |
| Secondary category | **Utilities** |
| Price | **Free** (with one optional In-App Purchase) |
| Age rating | **4+** |
| Platform | macOS 13.0+ (Apple Silicon + Intel, universal) |
| Primary language | **English (U.S.)** |
| Localizations | English (U.S.), **Português (Brasil)** — see §13 |
| Copyright | `© 2026 amis.co` |

---

## 1. Name & ASO strategy

**Recommended app name (30 char max):**
> `My App Spot: App Launcher`  *(25 chars)*

Keeps the brand ("My App Spot" - ties to the spotlight-buddy icon) **and** puts the
highest-intent keyword phrase, "App Launcher", into the indexed name. This is the
single biggest ASO lever, so spend it on a real search term.

**Subtitle (30 char max) - different words from the name:**
> `Launch apps by keyboard`  *(23 chars)*

**Keywords field (100 char max, comma-separated, NO spaces after commas, no
repeats of name/subtitle words):**
> `quick,search,fuzzy,shortcut,hotkey,productivity,menu bar,switcher,open,run,finder,dock,fast`

> Tip: every character counts - don't waste them on spaces, plurals the store
> already stems, or words already in the name/subtitle.

**If you'd rather rename the app** (bundle id stays `co.amis.myappspot` either way),
pick a short, brandable, trademark-free name and still append a keyword, e.g.:
- `Zip: App Launcher`
- `Dash Launcher`
- `Flick - App Launcher`
- `Beam: Quick App Launcher`

Recommendation: **keep "My App Spot"** - it's memorable, matches the mascot, and the
`: App Launcher` suffix already captures the ASO value. Only rebrand if you dislike
the current name.

---

## 2. Promotional Text (170 char max - editable anytime, no review)

> `The fastest way to open any Mac app: one shortcut, a few letters, launch. Pin your favorites to Cmd 1-0. Beautiful, private, and out of your way.`

---

## 3. Description (4000 char max)

```
My App Spot is a fast, keyboard-first app launcher for your Mac. Press one shortcut, type a couple of letters, and launch any app instantly. No more digging through the Dock, Launchpad, or Finder.

It's built to feel at home on your Mac: a beautiful Liquid Glass window that adapts to light and dark, lives quietly in your menu bar, and opens on whatever screen your cursor is on.

WHY YOU'LL LOVE IT
• Launch in a keystroke. Open any app the moment you think of it.
• Fuzzy search that keeps up. Type a few letters and the best match is already selected.
• Keyboard-first. Arrow keys to move, Cmd 1-0 to jump straight to a result, Return to launch.
• Pin your favorites. Keep your most-used apps at the top, each on its own Cmd-number shortcut. (Optional one-time unlock, or unlock free for the session.)
• Liquid Glass design. A gorgeous, glassy look in light or dark.
• Menu-bar app. No Dock clutter. It's there when you need it, invisible when you don't.

PRIVATE BY DESIGN
My App Spot collects no data and makes no network connections, ever. It's fully sandboxed, so your apps and activity stay entirely on your Mac.

FAST AND LIGHT
Instant indexing, instant search, a tiny footprint. It gets out of your way and stays out of it.

Set your global shortcut and you're off. Welcome to a faster Mac.
```

---

## 4. What's New (release notes - required each version)

v1.0:
> `First release. A fast, private, keyboard-first app launcher for your Mac.`

---

## 5. URLs (all required-ish)

| Field | Value |
|---|---|
| Support URL | **Required.** `https://myappspot.app/support.html` |
| Marketing URL | `https://myappspot.app/` |
| Privacy Policy URL | **Required.** `https://myappspot.app/privacy.html` (hosts §9) |

Contact email (support page + privacy policy): **`jp@amis.co`**.

---

## 6. App Privacy (the "nutrition label")

Answer the App Privacy questionnaire as **Data Not Collected**:
- "Do you or your third-party partners collect data from this app?" > **No**.
- This is truthful and verified: no HTTP client in the app, no analytics/telemetry,
  and the CSP blocks all remote origins (see `docs/distribution.md`). The only
  network entitlement exists because WKWebView needs it to load the local UI under
  the sandbox - it initiates zero requests.

---

## 7. Pricing & In-App Purchase

- **App price:** Free.
- **In-App Purchase:** one **Non-Consumable** - the favorites unlock.
  - Reference name (internal): `Unlock Favorites`
  - Product ID: **`co.amis.myappspot.favorites`** (must match `swift/iap.swift`)
  - Price tier: your call (a low "tip" tier, e.g. Tier 2-3 / ~$1.99-$2.99).
  - Display Name (localized): `Unlock Favorites`
  - Description: `Pin up to 10 apps to the top of the launcher, each with a Cmd 1-0 shortcut. A one-time unlock that supports development.`
  - Review screenshot: a capture of the Favorites tab / paywall (1024×... any size).
  - Note for reviewer: favorites can also be unlocked **free for the session** - the
    purchase is an optional, permanent "support the developer" unlock.
  - Full setup + testing steps: **`docs/iap.md`**.

---

## 8. Media

- **App icon:** 1024×1024 PNG (no alpha, no rounded corners - the store rounds it).
  Export from `assets/app-icon.svg`: `rsvg-convert -w 1024 -h 1024 assets/app-icon.svg -o icon-1024.png` (or use `src-tauri/icons/icon.png`).
- **Screenshots (required, macOS):** use `assets/store/*.png` (2560×1600). Order them
  by ASO impact - hero first (already numbered `01...05`). Regenerate/tweak with
  `mise run screenshots` (see `assets/store/README.md`; you can drop in real
  Liquid-Glass captures). Localized pt-BR headlines render to `assets/store/pt-BR/`
  in the same run (see §13.7).
- **App Preview (optional video):** a 15-30s screen recording (launch, search, favorites), if you want one later.

---

## 9. Privacy Policy (template to host)

```
Privacy Policy for My App Spot

My App Spot does not collect, store, transmit, or share any personal data.

The app runs entirely on your Mac. It does not connect to the internet, contains no
analytics or tracking, and sends no information to us or any third party. Your list
of applications, searches, favorites, and settings never leave your device.

In-app purchases are handled by Apple; we never see your payment details.

Because we collect no data, there is nothing to access, correct, or delete on our
side. If you have questions, contact jp@amis.co.

Last updated: September 17, 2026.
```

> This is already hosted at https://myappspot.app/privacy.html — keep the two in sync.

---

## 10. App Review notes (to reviewers)

```
- No account or login required; the app works immediately.
- It is a menu-bar (accessory) app. After launch, use the menu-bar icon or press
  the global shortcut (default Cmd Shift Space) to open the launcher.
- The single In-App Purchase ("Unlock Favorites") is optional. Favorites can also be
  unlocked free for the current session via the "free" link in Settings > Favorites,
  so the feature is testable without purchasing.
- The app makes no network requests and collects no data.
```

---

## 11. Export compliance / encryption

- Uses no encryption beyond what Apple's OS provides and makes no network calls. In
  App Store Connect, answer the encryption question so it's **exempt** ("None of
  the algorithms mentioned above" / standard OS encryption only). You can also add
  `ITSAppUsesNonExemptEncryption = false` to `Info.plist` to skip the prompt.

---

## 12. Pre-submit checklist

- [ ] App record created (name, subtitle, category, price = Free).
- [ ] Keywords, promo text, description filled from above.
- [ ] Screenshots (2560×1600) + 1024 icon uploaded.
- [ ] Support & Privacy Policy URLs live.
- [ ] App Privacy = Data Not Collected.
- [ ] IAP `co.amis.myappspot.favorites` created + submitted **with** the build.
- [ ] Age rating questionnaire > 4+.
- [ ] Export compliance answered (exempt).
- [ ] Build uploaded: `mise run release` (MAS pipeline, includes `--features storekit`) > upload the `.pkg` via Transporter (see `docs/distribution.md`).
- [ ] Reviewer notes added.
- [ ] pt-BR localization filled (Name, Subtitle, Keywords, Promo, Description, What's New, IAP) — §13.
- [ ] Submit for review.

---

## 13. Português (Brasil) — pt-BR localization

Add **Português (Brasil)** as a localization in App Store Connect (App
Information → Localizable Information, and each version's metadata), then paste the
copy below. Same ASO rules as §1: the **Name**, **Subtitle**, and **Keywords** are the
only indexed fields, so they carry distinct terms and don't repeat each other.
Terminology follows Apple's official pt-BR (keeps brand terms untranslated: Mac, Dock,
Launchpad, Finder, Liquid Glass, App Store, Return). "Launcher" is kept as-is — it's
the common loanword in Brazilian tech usage and reads better than "lançador".

### 13.1 Name & Subtitle

**App Name (30 char max):**
> `My App Spot: Launcher de Apps`  *(29 chars)*

**Subtitle (30 char max) — different words from the name:**
> `Abra apps pelo teclado`  *(22 chars)*

**Keywords (100 char max, comma-separated, NO spaces after commas, no repeats of
name/subtitle words):**
> `rápido,busca,fuzzy,atalho,tecla,produtividade,alternar,executar,dock,iniciar,pesquisa,barra de menus`

### 13.2 Promotional Text (170 char max)

> `A forma mais rápida de abrir qualquer app do Mac: um atalho, algumas letras e pronto. Fixe seus favoritos em Cmd 1-0. Bonito, privado e sem atrapalhar.`

### 13.3 Description (4000 char max)

```
O My App Spot é um launcher de apps rápido e voltado ao teclado para o seu Mac. Pressione um atalho, digite algumas letras e abra qualquer app na hora. Chega de vasculhar o Dock, o Launchpad ou o Finder.

Ele foi feito para parecer nativo no seu Mac: uma bela janela Liquid Glass que se adapta aos modos claro e escuro, fica discreta na barra de menus e abre na tela onde está o seu cursor.

POR QUE VOCÊ VAI ADORAR
• Abra apps num toque. Abra qualquer app no instante em que pensar nele.
• Busca aproximada (fuzzy) que acompanha você. Digite algumas letras e a melhor correspondência já vem selecionada.
• Voltado ao teclado. Setas para navegar, Cmd 1-0 para ir direto a um resultado, Return para abrir.
• Fixe seus favoritos. Mantenha os apps mais usados no topo, cada um com seu próprio atalho Cmd-número. (Desbloqueio único opcional, ou libere grátis pela sessão.)
• Design Liquid Glass. Um visual elegante e translúcido, no modo claro ou escuro.
• App na barra de menus. Sem bagunçar o Dock. Está lá quando você precisa e some quando não precisa.

PRIVADO POR DESIGN
O My App Spot não coleta nenhum dado e nunca faz conexões de rede. Ele roda totalmente em sandbox, então seus apps e sua atividade ficam inteiramente no seu Mac.

RÁPIDO E LEVE
Indexação instantânea, busca instantânea e um consumo mínimo. Ele sai do seu caminho e continua fora dele.

Defina seu atalho global e pronto. Boas-vindas a um Mac mais rápido.
```

### 13.4 What's New (release notes)

v1.0:
> `Primeira versão. Um launcher de apps rápido, privado e voltado ao teclado para o seu Mac.`

### 13.5 In-App Purchase (localized display)

- **Display Name (pt-BR):** `Desbloquear Favoritos`
- **Description (pt-BR):** `Fixe até 10 apps no topo do launcher, cada um com um atalho Cmd 1-0. Um desbloqueio único que apoia o desenvolvimento.`

### 13.6 Privacy Policy (pt-BR)

```
Política de Privacidade do My App Spot

O My App Spot não coleta, armazena, transmite nem compartilha nenhum dado pessoal.

O app funciona inteiramente no seu Mac. Ele não se conecta à internet, não contém
análises nem rastreamento e não envia nenhuma informação para nós ou para terceiros.
Sua lista de aplicativos, buscas, favoritos e ajustes nunca saem do seu dispositivo.

As compras dentro do app são processadas pela Apple; nunca vemos os seus dados de
pagamento.

Como não coletamos nenhum dado, não há nada para acessar, corrigir ou excluir do nosso
lado. Em caso de dúvidas, entre em contato pelo e-mail jp@amis.co.

Última atualização: 17 de setembro de 2026.
```

### 13.7 Screenshots

Localized screenshots (pt-BR headlines, same app captures) are generated at
`assets/store/pt-BR/01-05*.png` by `mise run screenshots`. Upload these under the
Português (Brasil) localization in App Store Connect; English stays at
`assets/store/01-05*.png`.

> **Note:** App Review notes (§10) can stay in English — reviewers read English.
