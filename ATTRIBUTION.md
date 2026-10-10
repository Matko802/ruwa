# Ruwa — attribution

Ruwa is built directly on
[Gelectron](https://github.com/mileswolfallen2/gelectron) by
mileswolfallen2 (MIT License): the Rust runtime (`crates/`), the
Electron-compatible JS layer (`src/electron/`), the CLI (`cli/`) and the
packager (`packager/`) are upstream Gelectron, with the full upstream
history preserved in this repo.

Local patches on top of upstream (both required to run at all on Linux):

1. `crates/gelectron-app/src/main.rs` — the tao event loop uses
   `ControlFlow::Poll` plus an explicit sleep instead of `WaitUntil`.
   `WaitUntil` never wakes the loop on some Linux/X11 setups, which
   silently drops every JS→native message (no window is ever created).
2. `vendor/wry/src/webkitgtk/mod.rs` — accept `RawWindowHandle::Xcb`
   as the WebView parent window. XCB and Xlib share the X11 XID
   namespace, so tao/x11rb window ids work directly. Upstream only
   accepted `Xlib`, which tao 0.30 never produces, so every WebView
   failed with "the window handle kind is not supported".

Ruwa-specific additions: `app/` (the WhatsApp client), `flake.nix`
(Nix package with X11 wrapper + dev shell), `data/` (desktop entry,
icons), `neutron/` (Datcord rebrand recipe), this file.

## Neutron variant (`packages.neutron`)

The Neutron build derives from [Datcord](https://github.com/gamingdoom/datcord)
by gamingdoom (GPL-3.0), itself built on
[Neutron](https://github.com/gamingdoom/neutron) (MIT). At package build
time the official Datcord 0.6.2 Linux release tarball is downloaded and
`neutron/rebrand.py` converts it to WhatsApp (homepage pref, external-link
regex, branding strings/icons, app/profile names); no Firefox source
compilation is needed. The shipped tree still contains Datcord's GPL
`launch-app` helper binary (unused at runtime — `neutron/launch-ruwa.sh`
is used instead), so the Neutron variant as a whole is distributed under
GPL-3.0-only (see the flake `meta.license`). The Gelectron variant above
remains MIT OR Apache-2.0.
