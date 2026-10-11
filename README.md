# Ruwa

Lightweight WhatsApp desktop client for Linux: a site-specific Firefox
bundle (Neutron) pointed at https://web.whatsapp.com/ — no bundled
Chromium, no Firefox source compilation. See `ATTRIBUTION.md`.

The window is chromeless — just the page. Control it through the
tray menu (Show / Reload / Settings / Quit), `Ctrl+R` to reload, `F11`
for fullscreen.

Site styling comes from `userContent.css`, rebuilt on every launch from
the Ruwa base plus your own tweaks: Settings has a Custom CSS editor
writing `~/.config/ruwa/custom.css`. The window chrome file
(`chrome/userChrome.css`) is managed by Ruwa; your rules belong in
custom CSS, which is never overwritten.

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
`~/.local/share/ruwa/profile/` across builds (a `user.js.ruwa-backup`
is kept from before Ruwa management).

`RUWA_PROFILE` overrides the profile dir, `RUWA_ENGINE` overrides the
engine binary, `RUWA_BACKEND` (`auto`|`wayland`|`x11`) overrides the
Settings display backend, `RUWA_VERBOSE=1` enables extra logging.
CLI: `ruwa --show | --quit | --minimized | --help`; any extra arguments
(e.g. a URL) are passed to the engine; a second `ruwa` invocation
focuses the running window via Firefox remoting.

## Notes

- Wayland is preferred when available; set Display backend to
  “X11 / XWayland” in Settings (or `RUWA_BACKEND=x11`) if calls or
  screen-share misbehave. The engine runs in an FHS env with system
  Firefox-runtime libs, host `/run` bound in for PipeWire/PulseAudio
  and the D-Bus session bus.
- External links open in your default browser via the
  bundled `open-in-default-browser` helper; only WhatsApp hosts
  (`whatsapp.com`, `whatsapp.net`, `wa.me`, incl. subdomains) stay
  in-app.
- Downloads use Neutron's default handling (Ruwa no longer overrides
  any `browser.download.*` prefs; stale Ruwa-managed entries are removed
  from `user.js` on next Settings save).
- Voice/video calls need mic/camera permission (pre-granted for
  WhatsApp origins) plus xdg-desktop-portal + PipeWire on Wayland.
- The tray icon needs a StatusNotifier host (KDE natively, GNOME via
  the AppIndicator extension, or a Wayland bar with a tray). Without
  one the app still works, minus the icon. The tray shows
  NeedsAttention when the window is closed but the tray is kept alive.
- Closing the window keeps the app in the tray by default (Settings →
  Behavior → “Keep running in tray”); turn it off to quit on close.
  “Start minimized to tray” + “Start on login” gives login-to-tray.
