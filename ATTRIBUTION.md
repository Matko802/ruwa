# Ruwa — attribution

Ruwa 0.4.0+ is built with [Tauri 2](https://v2.tauri.app/)
(MIT/Apache-2.0) on the system's WebKitGTK webview
(WebKitGTK, LGPL-2.1) — on macOS the same codebase uses WKWebView.
No browser is bundled and no browser source is compiled.

The bundled `uaaa-decryptor` mod is adapted from
[UAAAEncryption](https://github.com/Matko802/UAAAEncryption) by Matko802
(MIT, © 2026 Matko802): `mods/uaaa-decryptor/uaaa-core.js` and `LICENSE`
are vendored verbatim, `uaaa-decryptor.js` is the upstream userscript
minus its metadata block, with `GM_*` shimmed onto `localStorage` and
remote `@require`/font `@import` inlined or dropped for offline use.

Pre-0.4.0 Ruwa releases were a rebranded build of
[Datcord](https://github.com/gamingdoom/datcord) by gamingdoom (GPL-3.0),
itself built with [Neutron](https://github.com/gamingdoom/neutron) (MIT)
on Firefox/Gecko. That engine is gone; if you upgrade from Ruwa ≤ 0.3,
link your device again once (the old Firefox profile is left untouched)
and your behavior/display/mods preferences are migrated automatically.
