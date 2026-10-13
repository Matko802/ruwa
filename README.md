# Ruwa

Lightweight WhatsApp desktop client for Linux: an Electron app pointed
at https://web.whatsapp.com/ — real Chromium inside, so voice/video
calls, screen-share and notifications work like Chrome. See
`ATTRIBUTION.md`.

One window, three views: a slim custom topbar (Reload, Settings,
window controls), the WhatsApp page, and fullscreen Settings (topbar,
menu, `Ctrl+,`, or the tray menu). The tray menu has Show / Reload /
Settings / Quit; clicking the tray icon shows the window. The window
title and tray tooltip mirror the page title, so unread counts show
up there.

Site styling is the Ruwa base plus your own tweaks: Settings has a
Custom CSS editor writing `~/.config/ruwa/custom.css` (plain CSS).
Reload to apply edits.

Premade mods ship with Ruwa and run inside WhatsApp Web — toggle them
in Settings (applies immediately — the page reloads). Ships with **UAAA
Decryptor**: messages containing `UAAA…` cipher blobs get inline
Decrypt buttons, and an Encrypt button appears while you type in the
chat box. Made for fun, not secure — see
[`mods/uaaa-decryptor/`](mods/uaaa-decryptor/). To add a mod, drop a
folder with a `mod.json` + JS files into `mods/` (see the UAAA mod for
the format) and rebuild (mods are loaded from disk at startup, no
build step).

## Install (Nix)

```sh
nix build .#ruwa
./result/bin/ruwa
```

Ships `ruwa.desktop` and icons. On first launch, scan the QR code with
your phone (or link with your phone number); the session persists in
`~/.local/share/ruwa/electron/` across builds.

Upgrading from Ruwa ≤ 0.4? Link your device again once — each engine
keeps its own login. Your `config.json` settings and `custom.css`
carry over untouched.

`RUWA_BACKEND` (`auto`|`wayland`|`x11`) overrides the Settings display
backend, `RUWA_VERBOSE=1` enables extra logging.
CLI: `ruwa --show | --quit | --minimized | --reload | --help`; a second
`ruwa` invocation focuses the running window (or quits/reloads it with
`--quit` / `--reload`).

Dev: `nix develop`, then `electron .` to run, `node --check main.js`
for a syntax check. No npm dependencies — vanilla JS only.

## Notes

- Voice/video calls need mic/camera permission (auto-granted for
  WhatsApp origins). Ruwa reports the bundled Chromium version as its
  user agent, set before the window is created so page checks see
  plain Chrome. On Wayland, screen-share goes through
  xdg-desktop-portal + PipeWire; set Display backend to “X11 /
  XWayland” in Settings (or `RUWA_BACKEND=x11`) if calls or
  screen-share misbehave.
- External links open in your default browser; only WhatsApp hosts
  (`whatsapp.com`, `whatsapp.net`, `wa.me`, incl. subdomains) stay
  in-app. Same-host popups are folded back into the main view.
- Downloads land in `~/Downloads`.
- The tray icon needs a StatusNotifier host (KDE natively, GNOME via
  the AppIndicator extension, or a Wayland bar with a tray). Without
  one the app still works, minus the icon.
- Closing the window keeps the app in the tray by default (Settings →
  “Keep running in tray”); turn it off to quit on close.
  “Start minimized to tray” + “Start on login” gives login-to-tray.
