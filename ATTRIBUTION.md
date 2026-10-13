# Ruwa — attribution

Ruwa 0.5.0+ is an [Electron](https://www.electronjs.org/) app
(MIT): it bundles Electron's Chromium for rendering WhatsApp Web,
with no browser source compilation. The app code in this repo
(`main.js`, topbar/panel frontend) is Ruwa's own.

The bundled `uaaa-decryptor` mod is adapted from
[UAAAEncryption](https://github.com/Matko802/UAAAEncryption) by Matko802
(MIT, © 2026 Matko802): `mods/uaaa-decryptor/uaaa-core.js` and `LICENSE`
are vendored verbatim, `uaaa-decryptor.js` is the upstream userscript
minus its metadata block, with `GM_*` shimmed onto `localStorage` and
remote `@require`/font `@import` inlined or dropped for offline use.

History: Ruwa 0.4.x was built with [Tauri 2](https://v2.tauri.app/)
(MIT/Apache-2.0) on system WebKitGTK; Ruwa ≤ 0.3 was a rebranded build
of [Datcord](https://github.com/gamingdoom/datcord) by gamingdoom
(GPL-3.0), itself built with [Neutron](https://github.com/gamingdoom/neutron)
(MIT) on Firefox/Gecko. Those engines are gone; if you upgrade, link
your device again once (each engine keeps its own login) — your
`config.json` settings and `custom.css` carry over automatically.
