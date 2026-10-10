# Ruwa

Lightweight WhatsApp desktop client for Linux, built on
[Gelectron](https://github.com/mileswolfallen2/gelectron) — an
Electron-compatible runtime using native web views (WebKitGTK on Linux,
no bundled Chromium). See `ATTRIBUTION.md`.

`app/` holds the WhatsApp client (`package.json` + `main.js`): a single
window pointed at https://web.whatsapp.com/ with login persistence,
external links opened in the default browser (`shell.openExternal`),
unread-count tracking from the page title, and native notifications for
new messages. Everything else in this repo is the Gelectron runtime.

## Run from source

```sh
nix develop            # system WebKitGTK, mold linker, node
cargo build -p gelectron
node cli/gelectron.js app/
```

Without Nix you need Rust stable, Node 18+, `webkit2gtk-4.1`,
`libsoup-3`, GTK3 dev files, `pkg-config`, and `xdg-utils`, then the
same three commands.

## Install (Nix)

```sh
nix build .#ruwa
./result/bin/ruwa
```

`ruwa` also ships a `ruwa.desktop` entry and icons. On first launch,
scan the QR code with your phone; the session persists in
`~/.local/share/ruwa/`.

## Notes

- X11/XWayland is used even on Wayland desktops: the bundled `wry`
  fork only supports Xlib/XCB parent windows (see `ATTRIBUTION.md`).
  The wrapper unsets `WAYLAND_DISPLAY` for reliability.
- Tray icon: Gelectron's `Tray` is currently a stub, so Ruwa has no
  tray icon yet and closing the window quits the app.
- Downloads from chats use the WebKitGTK default handling; inline
  media (images, voice notes, video) works.
- Voice/video calls need mic/camera permission plus
  xdg-desktop-portal + PipeWire on Wayland.
- `RUWA_URL` overrides the start URL (handy for testing),
  `RUWA_TEST_LOG` makes the app log `unread:` / `open-external:` /
  `report:` events to a file instead of acting on them.
