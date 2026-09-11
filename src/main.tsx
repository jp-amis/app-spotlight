/* @refresh reload */
import { render } from "solid-js/web";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./app.css";
import Launcher from "./launcher/Launcher";
import Settings from "./settings/Settings";

// One frontend bundle, two windows. Route by the Tauri window label.
const label = getCurrentWindow().label;
const root = document.getElementById("root")!;

// The launcher is transparent (native vibrancy shows through); the settings window
// must be opaque so it never exposes the white window backing during resize.
if (label === "settings") document.body.classList.add("settings-window");

// Suppress the webview's right-click menu (incl. "Inspect Element") everywhere
// except inside editable fields, where it's useful for cut/copy/paste.
document.addEventListener("contextmenu", (e) => {
  const t = e.target as HTMLElement | null;
  if (!t?.closest("input, textarea, [contenteditable='true']")) e.preventDefault();
});

render(() => (label === "settings" ? <Settings /> : <Launcher />), root);
