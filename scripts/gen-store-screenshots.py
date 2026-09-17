#!/usr/bin/env python3
"""Generate Mac App Store marketing screenshots (2560x1600) as SVG -> PNG.

Beautiful, ASO-driven compositions: benefit headline + a faithful vector mockup
of the launcher UI, on the brand gradient. Edit the SHOTS list and re-run.
Outputs to assets/store/NN-slug.{svg,png}.
"""
import os, subprocess, html, struct, base64

W, H = 2560, 1600
ACCENT = "#5B4BE0"
OUT = os.path.join(os.path.dirname(__file__), "..", "assets", "store")

FONT = "SF Pro Display, Helvetica Neue, Helvetica, Arial, sans-serif"
MONO = "SF Mono, Menlo, Apple Symbols, Helvetica, sans-serif"  # has the cmd glyph

# Optional mascot overlay — composites onto any shot that opts in with mascot=<pos>.
# Every shot uses the main mascot (assets/store/mascot.png) unless it overrides with
# mascot_img="mascot-cta.png". Both PNGs come from extras/*.svg via `mise run mascots`.
# If the file is missing, a dashed placeholder shows where it will land.
MASCOT_IMG = os.path.join(OUT, "mascot.png")
MASCOT = dict(x=90, y=1140, w=440)   # default; per-shot presets below
# Per-shot mascot placements (tweak freely, or set mascot=dict(x=,y=,w=) inline).
M_BL = dict(x=90, y=1150, w=440)     # bottom-left
M_BR = dict(x=2030, y=1170, w=420)   # bottom-right
M_BL_HI = dict(x=110, y=1090, w=410) # bottom-left, a touch higher/smaller
M_CENTER = dict(x=980, y=1010, w=600) # centered + bigger (for sparse shots)

# name, tile color (icon-ish), glyph letter
APPS = [
    ("Terminal", "#26262B", "›_"),
    ("Calendar", "#FF3B30", "31"),
    ("Notes", "#FEC431", "≡"),
    ("Music", "#FB2C55", "♪"),
    ("Photos", "#3C9BFF", "◈"),
    ("Maps", "#34C759", "▲"),
    ("Messages", "#1BD760", "●"),
    ("Reminders", "#FF9500", "✓"),
    ("Preview", "#8E8E93", "▢"),
    ("System Settings", "#6E6E73", "⚙"),
]

def esc(s): return html.escape(str(s), quote=True)

def tile(x, y, s, color, glyph, glyph_color="#ffffff"):
    return f'''
      <rect x="{x}" y="{y}" width="{s}" height="{s}" rx="{s*0.26:.0f}" fill="{color}"/>
      <rect x="{x}" y="{y}" width="{s}" height="{s*0.5:.0f}" rx="{s*0.26:.0f}" fill="#ffffff" opacity="0.12"/>
      <text x="{x+s/2:.0f}" y="{y+s*0.66:.0f}" font-family="{FONT}" font-size="{s*0.42:.0f}"
            font-weight="700" fill="{glyph_color}" text-anchor="middle">{esc(glyph)}</text>'''

def row(px, y, pw, app, badge, fav, selected):
    name, color, glyph = app
    pad = 34
    parts = []
    if selected:
        parts.append(f'<rect x="{px+16}" y="{y}" width="{pw-32}" height="92" rx="20" fill="{ACCENT}"/>')
    ts = 60
    parts.append(tile(px+pad, y+16, ts, color, glyph))
    tc = "#ffffff" if selected else "#1c1c22"
    parts.append(f'<text x="{px+pad+ts+28}" y="{y+60}" font-family="{FONT}" font-size="34" '
                 f'font-weight="500" fill="{tc}">{esc(name)}</text>')
    rx = px + pw - pad
    # cmd badge
    bw = 74
    bg = "#ffffff" if selected else "#00000010"
    bfg = "#ffffff" if selected else "#8a8a99"
    bop = "0.22" if selected else "1"
    parts.append(f'<rect x="{rx-bw}" y="{y+30}" width="{bw}" height="34" rx="9" fill="{bg}" opacity="{bop}"/>')
    parts.append(f'<text x="{rx-bw/2:.0f}" y="{y+54}" font-family="{MONO}" font-size="24" '
                 f'fill="{bfg}" text-anchor="middle">{esc(badge)}</text>')
    if fav:
        star = "#ffffff" if selected else "#c9b8ff"
        parts.append(f'<text x="{rx-bw-34}" y="{y+56}" font-family="{FONT}" font-size="30" '
                     f'fill="{star}" text-anchor="middle">★</text>')
    return "".join(parts)

def launcher(cx, top, query, rows, fav_count, sel, width=1180):
    px = cx - width/2
    parts = [f'<g filter="url(#drop)">',
             f'<rect x="{px}" y="{top}" width="{width}" height="{launcher_height(rows)}" rx="40" '
             f'fill="#ffffff" opacity="0.94"/>',
             f'<rect x="{px}" y="{top}" width="{width}" height="{launcher_height(rows)}" rx="40" '
             f'fill="none" stroke="#ffffff" stroke-opacity="0.6" stroke-width="1.5"/></g>']
    # search header
    hy = top + 46
    parts.append(f'<circle cx="{px+70}" cy="{hy+34}" r="20" fill="none" stroke="#9a9aa8" stroke-width="6"/>')
    parts.append(f'<line x1="{px+86}" y1="{hy+50}" x2="{px+104}" y2="{hy+68}" stroke="#9a9aa8" stroke-width="6" stroke-linecap="round"/>')
    qcolor = "#1c1c22" if query else "#9a9aa8"
    qtext = query if query else "Search apps..."
    parts.append(f'<text x="{px+130}" y="{hy+52}" font-family="{FONT}" font-size="46" '
                 f'font-weight="300" fill="{qcolor}">{esc(qtext)}</text>')
    if not query:
        parts.append(f'<rect x="{px+width-150}" y="{hy+18}" width="104" height="42" rx="12" fill="#00000008"/>')
        parts.append(f'<text x="{px+width-98}" y="{hy+47}" font-family="{MONO}" font-size="24" '
                     f'fill="#8a8a99" text-anchor="middle">⌘ ;</text>')
    # rows
    ry = top + 140
    rh = 92
    for i, (app, badge, fav) in enumerate(rows):
        parts.append(row(px, ry, width, app, badge, fav, i == sel))
        ry += rh
        if fav_count and i == fav_count - 1:
            parts.append(f'<line x1="{px+40}" y1="{ry+2}" x2="{px+width-40}" y2="{ry+2}" '
                         f'stroke="#0000000f" stroke-width="2"/>')
            ry += 18
    return "".join(parts)

def launcher_height(rows):
    return 140 + len(rows)*92 + 60

def png_size(path):
    with open(path, "rb") as f:
        f.read(16)
        return struct.unpack(">II", f.read(8))

def data_uri(path):
    # Embed PNGs inline. librsvg only loads xlink:href files from the SVG's own
    # directory subtree, so absolute/parent paths fail for locale subdirs — data
    # URIs make rendering location-independent.
    with open(path, "rb") as f:
        return "data:image/png;base64," + base64.b64encode(f.read()).decode()

def screenshot(cfg):
    icon_uri = data_uri(os.path.abspath(os.path.join(OUT, "..", "app-icon-1024.png")))
    # Use a real capture if you dropped one at assets/store/captures/<slug>.png,
    # otherwise fall back to the vector mock.
    cap = cfg.get("capture") or os.path.join(OUT, "captures", cfg["slug"] + ".png")
    if cap and os.path.exists(cap):
        # Composite a REAL window capture (real Liquid Glass) instead of the mock.
        pw, ph = png_size(cap)
        tw = cfg.get("cap_w", 1240)
        th = tw * ph / pw
        x, y = W/2 - tw/2, cfg["top"]
        body = (f'<image xlink:href="{data_uri(cap)}" x="{x:.0f}" y="{y:.0f}" '
                f'width="{tw:.0f}" height="{th:.0f}" filter="url(#drop)"/>')
    else:
        body = launcher(W/2, cfg["top"], cfg.get("query", ""), cfg["rows"],
                        cfg.get("fav_count", 0), cfg.get("sel", 0))
    badge = ""
    if cfg.get("badge"):
        bt = cfg["badge"]
        text_w = len(bt) * 24
        has_icon = cfg.get("badge_icon") == "shield"
        icon_w, gap = (48, 16) if has_icon else (0, 0)
        pad = 46
        bw = pad * 2 + icon_w + gap + text_w
        bx = W / 2 - bw / 2
        by = H - 215
        icon = ""
        if has_icon:
            icon = (f'<g transform="translate({pad},21) scale(2)" fill="none" '
                    f'stroke="{ACCENT}" stroke-width="2" stroke-linecap="round" '
                    f'stroke-linejoin="round"><path d="M12 3l7 3v5c0 4.6-3 7.7-7 9'
                    f'-4-1.3-7-4.4-7-9V6l7-3z"/><path d="M9 12l2 2 4-4"/></g>')
        tx = pad + icon_w + gap + text_w / 2
        badge = (f'<g transform="translate({bx:.0f},{by})">'
                 f'<rect x="0" y="0" width="{bw:.0f}" height="90" rx="45" fill="#ffffff"/>'
                 f'{icon}'
                 f'<text x="{tx:.0f}" y="59" font-family="{FONT}" font-size="40" '
                 f'font-weight="800" fill="{ACCENT}" text-anchor="middle">{esc(bt)}</text></g>')
    # Optional mascot overlay (opt in per shot with mascot=True). Uses the drop-in
    # image at MASCOT_IMG, or a dashed placeholder if it's not there yet.
    mascot = ""
    mconf = cfg.get("mascot")
    if mconf:
        pos = mconf if isinstance(mconf, dict) else MASCOT
        mx, my, mw = pos["x"], pos["y"], pos["w"]
        # Main mascot everywhere; a shot may opt onto the alternate with mascot_img=.
        mimg = os.path.join(OUT, cfg.get("mascot_img", "mascot.png"))
        if os.path.exists(mimg):
            pw, ph = png_size(mimg)
            mh = mw * ph / pw
            mascot = (f'<image xlink:href="{data_uri(mimg)}" x="{mx}" y="{my}" '
                      f'width="{mw:.0f}" height="{mh:.0f}"/>')
        else:
            mh = mw * 0.78
            mascot = (f'<g><rect x="{mx}" y="{my}" width="{mw}" height="{mh:.0f}" rx="28" '
                      f'fill="#ffffff" fill-opacity="0.08" stroke="#ffffff" stroke-opacity="0.5" '
                      f'stroke-width="2" stroke-dasharray="10 8"/>'
                      f'<text x="{mx+mw/2:.0f}" y="{my+mh/2:.0f}" font-family="{FONT}" '
                      f'font-size="30" font-weight="600" fill="#ffffff" fill-opacity="0.85" '
                      f'text-anchor="middle">Mascot</text></g>')

    # App name is redundant with the App Store page, so show it only where a shot
    # opts in with brand=True (the hero). Keeps the other shots clean.
    brand = ""
    if cfg.get("brand"):
        brand = (f'<g transform="translate({W/2-210},{H-120})">'
                 f'<image xlink:href="{icon_uri}" x="0" y="-58" width="88" height="88"/>'
                 f'<text x="112" y="6" font-family="{FONT}" font-size="44" '
                 f'font-weight="700" fill="#ffffff">My App Spot</text></g>')
    return f'''<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"
     width="{W}" height="{H}" viewBox="0 0 {W} {H}">
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="0.4" y2="1">
      <stop offset="0" stop-color="#6E5EF8"/><stop offset="1" stop-color="#3E2ABF"/>
    </linearGradient>
    <radialGradient id="b1" cx="0.5" cy="0.5" r="0.5">
      <stop offset="0" stop-color="#9C8BFF" stop-opacity="0.55"/>
      <stop offset="1" stop-color="#9C8BFF" stop-opacity="0"/>
    </radialGradient>
    <radialGradient id="b2" cx="0.5" cy="0.5" r="0.5">
      <stop offset="0" stop-color="#F26FB2" stop-opacity="0.35"/>
      <stop offset="1" stop-color="#F26FB2" stop-opacity="0"/>
    </radialGradient>
    <filter id="drop" x="-40%" y="-30%" width="180%" height="230%">
      <feDropShadow dx="0" dy="40" stdDeviation="50" flood-color="#150a4a" flood-opacity="0.45"/>
    </filter>
  </defs>
  <rect width="{W}" height="{H}" fill="url(#bg)"/>
  <circle cx="380" cy="300" r="620" fill="url(#b1)"/>
  <circle cx="{W-300}" cy="{H-200}" r="560" fill="url(#b2)"/>
  <!-- headline -->
  <text x="{W/2}" y="200" font-family="{FONT}" font-size="96" font-weight="800"
        fill="#ffffff" text-anchor="middle">{esc(cfg["title"])}</text>
  <text x="{W/2}" y="288" font-family="{FONT}" font-size="46" font-weight="400"
        fill="#ffffff" fill-opacity="0.82" text-anchor="middle">{esc(cfg["sub"])}</text>
  {badge}
  {body}
  {mascot}
  {brand}
</svg>'''

# ---- the screenshot set (ASO: benefit headlines, keywords, ordered by impact) ----
def apps(*names):
    d = {n: (n, c, g) for (n, c, g) in APPS}
    return [d[n] for n in names]

SHOTS = [
    dict(slug="hero-launch", top=420, brand=True, mascot=M_BL,
         title="Open any app in a keystroke",
         sub="A fast, private, keyboard-first app launcher for your Mac.",
         query="", fav_count=3, sel=0,
         rows=[(a, f"⌘{i+1 if i<9 else 0}", i < 3)
               for i, a in enumerate(apps("Terminal","Calendar","Notes","Music","Photos","Messages"))]),
    dict(slug="fuzzy-search", top=470, mascot=M_CENTER,
         title="Fuzzy search that keeps up",
         sub="Type a few letters. The best match is already selected.",
         query="min", fav_count=0, sel=0,
         rows=[(a, f"⌘{i+1}", False)
               for i, a in enumerate(apps("Terminal","Reminders"))]),
    dict(slug="favorites", top=420, mascot=M_BR, mascot_img="mascot-cta.png",
         title="Pin your favorite apps",
         sub="Your most-used apps stay on top, each on its own number-key shortcut.",
         query="", fav_count=4, sel=1,
         rows=[(a, f"⌘{i+1 if i<9 else 0}", i < 4)
               for i, a in enumerate(apps("Music","Terminal","Photos","Messages","Maps","Preview"))]),
    dict(slug="keyboard", top=440, mascot=M_BL,
         title="Keyboard-first, mouse-optional",
         sub="Arrows to move, Cmd-digits to jump, Return to launch.",
         query="", fav_count=2, sel=2,
         rows=[(a, f"⌘{i+1}", i < 2)
               for i, a in enumerate(apps("Calendar","Notes","Maps","Reminders","Preview"))]),
    dict(slug="private", top=410, cap_w=1080, mascot=M_BR,
         title="Private. No network. Ever.",
         sub="Your apps and data never leave your Mac. Sandboxed for the App Store.",
         query="", fav_count=3, sel=0, badge="Offline", badge_icon="shield",
         rows=[(a, f"⌘{i+1}", i < 3)
               for i, a in enumerate(apps("Photos","Music","Messages","Calendar","Notes"))]),
]

# ---- localization -------------------------------------------------------------
# English lives inline in SHOTS (the default set). Each extra locale maps a shot
# slug to its translated headline/sub (and optional badge). The brand ("My App
# Spot") stays; captures are language-neutral, so every locale composites the same
# captures/<slug>.png. English renders to the store root; each locale to a subdir.
LOCALES = {
    "pt-BR": {
        "hero-launch": dict(title="Abra qualquer app num toque",
                            sub="Um launcher de apps rápido, privado e voltado ao teclado para o Mac."),
        "fuzzy-search": dict(title="Busca aproximada que acompanha você",
                             sub="Digite algumas letras. A melhor correspondência já vem selecionada."),
        "favorites": dict(title="Fixe seus apps favoritos",
                          sub="Seus apps mais usados no topo, cada um com seu atalho numérico."),
        "keyboard": dict(title="Voltado ao teclado, mouse opcional",
                         sub="Setas para navegar, Cmd-números para pular, Return para abrir."),
        "private": dict(title="Privado. Sem rede. Nunca.",
                        sub="Seus apps e dados nunca saem do seu Mac. Em sandbox para a App Store.",
                        badge="Offline"),
    },
}

def localize(cfg, loc):
    """Return cfg with title/sub/badge swapped for the locale (loc=None → English)."""
    if not loc:
        return cfg
    return {**cfg, **LOCALES[loc].get(cfg["slug"], {})}

def render_set(loc, outdir):
    os.makedirs(outdir, exist_ok=True)
    for i, cfg in enumerate(SHOTS, 1):
        base = os.path.join(outdir, f"{i:02d}-{cfg['slug']}")
        svg = screenshot(localize(cfg, loc))
        with open(base + ".svg", "w") as f:
            f.write(svg)
        subprocess.run(["rsvg-convert", "-w", str(W), "-h", str(H),
                        base + ".svg", "-o", base + ".png"], check=True)
        print("wrote", base + ".png")

def gen_wallpaper():
    """A plain brand-gradient wallpaper (matches the SVG background behind the
    launcher). Set it as your desktop so the Liquid Glass samples these colours,
    then the captured window blends seamlessly into the marketing frame."""
    ww, wh = 2880, 1800
    svg = (f'<svg xmlns="http://www.w3.org/2000/svg" width="{ww}" height="{wh}" '
           f'viewBox="0 0 {ww} {wh}"><defs><linearGradient id="bg" x1="0" y1="0" '
           f'x2="0.4" y2="1"><stop offset="0" stop-color="#6E5EF8"/>'
           f'<stop offset="1" stop-color="#3E2ABF"/></linearGradient></defs>'
           f'<rect width="{ww}" height="{wh}" fill="url(#bg)"/></svg>')
    base = os.path.join(OUT, "wallpaper")
    with open(base + ".svg", "w") as f:
        f.write(svg)
    subprocess.run(["rsvg-convert", "-w", str(ww), "-h", str(wh),
                    base + ".svg", "-o", base + ".png"], check=True)
    print("wrote", base + ".png  (set this as your desktop before capturing)")

def main():
    os.makedirs(OUT, exist_ok=True)
    os.makedirs(os.path.join(OUT, "captures"), exist_ok=True)
    gen_wallpaper()
    render_set(None, OUT)                       # English → store root
    for loc in LOCALES:                         # each locale → its own subdir
        render_set(loc, os.path.join(OUT, loc))

if __name__ == "__main__":
    main()
