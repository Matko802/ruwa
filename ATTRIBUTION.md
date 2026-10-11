# Ruwa — attribution

Ruwa is a rebranded build of [Datcord](https://github.com/gamingdoom/datcord)
by gamingdoom (GPL-3.0), itself built with
[Neutron](https://github.com/gamingdoom/neutron) (MIT) on Firefox/Gecko.

At package build time the official Datcord 0.6.2 Linux release tarball
is downloaded and `neutron/rebrand.py` converts it to WhatsApp
(homepage pref, external-link regex, branding strings/icons,
app/profile names); no Firefox source compilation is needed. The
shipped tree still contains Datcord's GPL `launch-app` helper binary
(unused at runtime — `neutron/launch-ruwa.sh` is used instead), so this
project as distributed is under GPL-3.0-only.

The bundled `uaaa-decryptor` mod is adapted from
[UAAAEncryption](https://github.com/Matko802/UAAAEncryption) by Matko802
(MIT, © 2026 Matko802): `mods/uaaa-decryptor/uaaa-core.js` and `LICENSE`
are vendored verbatim, `uaaa-decryptor.js` is the upstream userscript
minus its metadata block, with `GM_*` shimmed onto `localStorage` and
remote `@require`/font `@import` inlined or dropped for offline use.
