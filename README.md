# Ruwa

Lightweight WhatsApp desktop client for Linux: a Tauri 2 app pointed at
https://web.whatsapp.com/ — no bundled Chromium, no Firefox, no engine
downloads. It uses your system's WebKitGTK webview (WKWebView on macOS).
See `ATTRIBUTION.md`.

The window is chromeless — just the page. Control it through the
tray menu (Show / Reload / Settings / Quit); clicking the tray icon
shows the window.

Site styling comes from `assets/base.css` plus your own tweaks: Settings
has a Custom CSS editor writing `~/.config/ruwa/custom.css` (plain CSS,
no wrapper needed; old `@-moz-document` blocks are unwrapped on inject).
Reload to apply edits.

Premade mods ship with Ruwa and run inside WhatsApp Web — toggle them
in Settings → Mods (takes effect on next launch). Ships with **UAAA
Decryptor**: messages containing `UAAA…` cipher blobs get inline
Decrypt buttons, and an Encrypt button appears while you type in the
chat box. Made for fun, not secure — see
[`mods/uaaa-decryptor/`](mods/uaaa-decryptor/). To add a mod, drop a
folder with a `mod.json` + JS files into `mods/` (see the UAAA mod for
the format) and rebuild.

## Install (Nix)

```sh
nix build .#ruwa
./result/bin/ruwa
```

Ships `ruwa.desktop`, `ruwa-settings` (no desktop entry — open it from
the tray menu), and icons. On first launch, scan the QR code with your
phone (or link with your phone number); the session persists in
`~/.local/share/ruwa/webview/` across builds.

Upgrading from Ruwa ≤ 0.3 (Firefox/Neutron engine)? Link your device
again once — the old profile is a different engine and can't be reused.
Your behavior/display/mods preferences migrate automatically; your old
`custom.css` keeps working (`@-moz-document` wrappers are unwrapped).

`RUWA_BACKEND` (`auto`|`wayland`|`x11`) overrides the Settings display
backend, `RUWA_VERBOSE=1` enables extra logging.
CLI: `ruwa --show | --quit | --minimized | --reload | --help`; a second
`ruwa` invocation focuses the running window (or quits/reloads it with
`--quit` / `--reload`).

## Notes

- Wayland is preferred when available; set Display backend to
  “X11 / XWayland” in Settings (or `RUWA_BACKEND=x11`) if calls or
  screen-share misbehave.
- The webview reports a Chrome-on-Linux user agent so WhatsApp Web
  serves the full client, including calls.
- Voice/video calls need mic/camera permission (auto-granted for
  WhatsApp origins) plus xdg-desktop-portal + PipeWire on Wayland.
  Screen-share uses the system portal picker.
- External links open in your default browser; only WhatsApp hosts
  (`whatsapp.com`, `whatsapp.net`, `wa.me`, incl. subdomains) stay
  in-app. Same-host popups are folded back into the main window.
- Downloads land in `~/Downloads`.
- The tray icon needs a StatusNotifier host (KDE natively, GNOME via
  the AppIndicator extension, or a Wayland bar with a tray). Without
  one the app still works, minus the icon. The tooltip mirrors the
  page title, so unread counts show up there.
- Closing the window keeps the app in the tray by default (Settings →
  General → “Keep running in tray”); turn it off to quit on close.
  “Start minimized to tray” + “Start on login” gives login-to-tray.
