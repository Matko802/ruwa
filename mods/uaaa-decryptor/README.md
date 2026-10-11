# uaaa-decryptor (bundled Ruwa mod)

Encrypt/decrypt [UAAA](https://github.com/Matko802/UAAAEncryption)-style
cipher text directly in WhatsApp Web: messages containing `UAAA…` blobs
get inline Decrypt buttons, and an Encrypt button appears when you type
in the chat box.

- `uaaa-core.js` — upstream cipher core, vendored verbatim.
- `uaaa-decryptor.js` — upstream Violentmonkey userscript, adapted for
  Ruwa: metadata block removed, `GM_*` shimmed onto `localStorage`,
  remote `@require`/font `@import` inlined or dropped (fully offline).
- `LICENSE` — upstream MIT license text, vendored verbatim.

Upstream is the author's own project, made for fun — the cipher hides
text from casual readers, it is not real security.
