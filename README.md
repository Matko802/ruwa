# Ruwa

Lightweight WhatsApp desktop client for Linux: a site-specific Firefox
bundle (Neutron) pointed at https://web.whatsapp.com/ — no bundled
Chromium, no Firefox source compilation. See `ATTRIBUTION.md`.

The build starts from the official Datcord 0.6.2 Linux release and
rebrands it (`neutron/rebrand.py`): WhatsApp homepage pref, permission
pre-grants (notifications, mic, camera), external links opened in the
default browser, GPU/WebRender/VA-API prefs, Ruwa branding/icons, own
profile. `neutron/launch-ruwa.sh` runs the engine; the `ruwa` supervisor
(`src/`, Rust) adds single-instance locking, a StatusNotifier tray icon
(Show / Settings / Quit, when a tray host exists), and falls back to
direct engine launch otherwise. `ruwa-settings` is a GTK settings app
with a header bar: GPU acceleration, VA-API decoding, audio backend,
download folder, autostart, log-out (clears site data), and launch.

## Install (Nix)

```sh
nix build .#ruwa
./result/bin/ruwa
```

Ships `ruwa.desktop`, `ruwa-settings` (no desktop entry — open it from
the tray menu), and icons. On first launch, scan the QR code with your
phone (or link with your phone number); the session persists in
`~/.local/share/ruwa/profile/`. Each major build keeps its own
profile, so expect one fresh login after switching builds.

`RUWA_PROFILE` overrides the profile dir, `RUWA_ENGINE` overrides the
engine binary. Any extra arguments (e.g. a URL) are passed to the
engine; a second `ruwa` invocation focuses the running window via
Firefox remoting.

## Notes

- X11/XWayland is used even on Wayland desktops (the wrapper unsets
  `WAYLAND_DISPLAY`); the engine runs in an FHS env with system
  Firefox-runtime libs, host `/run` bound in for PipeWire/PulseAudio
  and the D-Bus session bus.
- External links (videos, docs) open in your default browser via the
  bundled `open-in-default-browser` helper; only WhatsApp hosts stay
  in-app.
- Voice/video calls need mic/camera permission (pre-granted for
  WhatsApp) plus xdg-desktop-portal + PipeWire on Wayland.
- The tray icon needs a StatusNotifier host (KDE natively, GNOME via
  the AppIndicator extension, or a Wayland bar with a tray). Without
  one the app still works, minus the icon.
- Closing the window quits the app.
