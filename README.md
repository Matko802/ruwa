# Ruwa

Lightweight WhatsApp desktop client for Linux: a site-specific Firefox
bundle (Neutron) pointed at https://web.whatsapp.com/ — no bundled
Chromium, no Firefox source compilation. See `ATTRIBUTION.md`.

The build starts from the official Datcord 0.6.2 Linux release and
rebrands it (`neutron/rebrand.py`): WhatsApp homepage pref, external
links opened in the default browser, Ruwa branding/icons, own profile.
`neutron/launch-ruwa.sh` wraps the engine with single-instance locking
(no tray host required, unlike Datcord's C++ launcher).

## Install (Nix)

```sh
nix build .#ruwa
./result/bin/ruwa
```

Ships a `ruwa.desktop` entry and icons. On first launch, scan the QR
code with your phone (or link with your phone number); the session
persists in `~/.local/share/ruwa/profile/`.

`RUWA_PROFILE` overrides the profile dir, `RUWA_DIR` overrides the
engine tree location. Any extra arguments (e.g. a URL) are passed to
the engine.

## Notes

- X11/XWayland is used even on Wayland desktops (the wrapper unsets
  `WAYLAND_DISPLAY`); the engine runs in an FHS env with system
  Firefox-runtime libs.
- External links (videos, docs) open in your default browser via the
  bundled `open-in-default-browser` helper; only WhatsApp hosts stay
  in-app.
- Voice/video calls need mic/camera permission plus
  xdg-desktop-portal + PipeWire on Wayland.
- Closing the window quits the app (no tray icon in this build).
