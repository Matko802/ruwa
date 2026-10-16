/* UAAA Decryptor for Ruwa (WhatsApp Web).
   Adapted from UAAADecryptor.user.js (upstream MIT, Matko802).
   Changes vs upstream: userscript metadata block removed (Ruwa loads
   this as a bundled mod, scoped to web.whatsapp.com); GM_* APIs are
   shimmed onto window.localStorage below; the Google Fonts @import was
   dropped so the mod works fully offline (monospace fallback kept).
   Upstream: https://github.com/Matko802/UAAAEncryption */

function GM_getValue(key, def) {
    try {
        var raw = window.localStorage.getItem('ruwa:uaaa:' + key);
        if (raw === null || raw === undefined) return def;
        return JSON.parse(raw);
    } catch (e) { return def; }
}
function GM_setValue(key, value) {
    try { window.localStorage.setItem('ruwa:uaaa:' + key, JSON.stringify(value)); } catch (e) {}
}
function GM_registerMenuCommand() {
}

(function() {
    'use strict';

    // Blocklist: do not run on the official hosted site to avoid self-interference
    try {
        const BLOCK_HOST = 'https://matko802.github.io/UAAAEncryption/';
        if (window.location.href && window.location.href.indexOf(BLOCK_HOST) === 0) {
            // Abort early
            return;
        }
    } catch (e) {
        // If anything goes wrong, continue execution (fail-open)
    }

    // --- Persisted settings ---
    const MODE_KEY = 'uaaa_default_mode';

    let defaultMode = GM_getValue(MODE_KEY, 'encrypted');

    function toggleDefaultMode() {
        defaultMode = defaultMode === 'encrypted' ? 'decrypted' : 'encrypted';
        GM_setValue(MODE_KEY, defaultMode);
        registerMenu();
        reprocessAllExisting();
    }

    function registerMenu() {
        GM_registerMenuCommand(
            `UAAA: Default page mode -> ${defaultMode === 'encrypted' ? 'Encrypted' : 'Decrypted'}`,
            toggleDefaultMode
        );
    }
    registerMenu();

    // --- Configuration ---
    const CONFIG = {
        url: 'https://matko802.github.io/UAAAEncryption/',
        regex: /(UAAA[UA]+)/gi,
        key: '\x55\x41\x41\x41', // "UAAA"
        ignoreTags: new Set(['SCRIPT', 'STYLE', 'TEXTAREA', 'INPUT', 'NOSCRIPT', 'CODE', 'PRE', 'CANVAS', 'SVG']),
    };

    const LOCK_PATH = 'M18 8h-1V6c0-2.76-2.24-5-5-5S7 3.24 7 6v2H6c-1.1 0-2 .9-2 2v10c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V10c0-1.1-.9-2-2-2zm-6 9c-1.1 0-2-.9-2-2s.9-2 2-2 2 .9 2 2-.9 2-2 2zm3.1-9H8.9V6c0-1.71 1.39-3.1 3.1-3.1 1.71 0 3.1 1.39 3.1 3.1v2z';
    const UNLOCK_PATH = 'M18 8h-1V6c0-2.76-2.24-5-5-5S7 3.24 7 6h2c0-1.66 1.34-3 3-3s3 1.34 3 3v2H6c-1.1 0-2 .9-2 2v10c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V10c0-1.1-.9-2-2-2zm-6 9c-1.1 0-2-.9-2-2s.9-2 2-2 2 .9 2 2-.9 2-2 2z';
    const INFO_PATH = 'M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-6h2v6zm0-8h-2V7h2v2z';

    function svgIcon(pathData, size) {
        const s = size || 18;
        return '<svg viewBox="0 0 24 24" width="' + s + '" height="' + s + '" fill="currentColor" aria-hidden="true"><path d="' + pathData + '"/></svg>';
    }

    const nodeQueue = new Set();
    let isProcessingQueue = false;
    const instances = new Set();
    const scannedNodes = new WeakSet();

    // Tracks text mutations caused by our own script so we don't trigger infinite loops
    const ourInternalMutations = new WeakSet();

    // --- UI & Styling (WhatsApp-style icon pills) ---
    function injectStyles() {
        const style = document.createElement('style');
        style.textContent = `
            .uaaa-btn-group {
                display: inline-flex;
                align-items: center;
                gap: 4px;
                margin: 0 6px;
                vertical-align: middle;
                user-select: none;
            }
            .uaaa-btn {
                display: inline-flex;
                align-items: center;
                justify-content: center;
                gap: 6px;
                padding: 5px 12px 5px 9px;
                border-radius: 999px;
                cursor: pointer;
                outline: none;
                text-decoration: none;
                font-size: 0.78em;
                font-weight: 500;
                font-family: inherit;
                white-space: nowrap;
                background: transparent;
                color: inherit;
                border: 1px solid currentColor;
                opacity: 0.9;
            }
            .uaaa-btn:hover { opacity: 1; }
            .uaaa-btn svg {
                width: 16px;
                height: 16px;
                fill: currentColor;
                display: block;
                flex: none;
            }
            .uaaa-compose-pill {
                display: inline-flex;
                align-items: center;
                gap: 8px;
                height: 40px;
                padding: 0 18px 0 14px;
                margin-inline-start: 8px;
                border-radius: 999px;
                border: 0;
                cursor: pointer;
                background: #00a884;
                color: #ffffff;
                font-size: 14px;
                font-weight: 500;
                font-family: inherit;
                white-space: nowrap;
                flex: none;
            }
            .uaaa-compose-pill:hover { filter: brightness(1.08); }
            .uaaa-compose-pill svg {
                width: 20px;
                height: 20px;
                fill: currentColor;
                display: block;
                flex: none;
            }
            .uaaa-compose-btn {
                display: inline-flex;
                align-items: center;
                justify-content: center;
                width: 40px;
                height: 40px;
                border-radius: 50%;
                border: 0;
                cursor: pointer;
                padding: 0;
                margin-inline-start: 8px;
                background: #00a884;
                color: #ffffff;
                flex: none;
                vertical-align: middle;
            }
            .uaaa-compose-btn:hover { filter: brightness(1.08); }
            .uaaa-compose-btn svg {
                width: 24px;
                height: 24px;
                fill: currentColor;
                display: block;
            }
            .uaaa-btn.success { color: #00e676; }
            .uaaa-btn.error { color: #ff5252; }
            [data-uaaa-encrypt] {
                background-color: #00a884 !important;
                color: #ffffff !important;
            }
            [data-uaaa-encrypt] svg {
                width: 24px !important;
                height: 24px !important;
                fill: currentColor;
                display: block;
            }
        `;
        document.head.appendChild(style);
    }

    function reprocessAllExisting() {
        for (const instance of instances) {
            instance.applyMode(defaultMode);
        }
    }

    // --- Core Processing Logic ---
    function processTextNode(node) {
      try {
        processTextNodeInner(node);
      } catch (e) {
        console.warn('[UAAA] node skipped:', e && e.message);
      }
    }

    function processTextNodeInner(node) {
        if (!node.nodeValue) return;
        if (!document.body.contains(node)) return;

        const parent = node.parentNode;
        if (!parent || parent.isContentEditable || CONFIG.ignoreTags.has(parent.tagName)) {
            return;
        }

        if (scannedNodes.has(node)) return;

        if (!node.nodeValue.includes('UAAA')) {
            scannedNodes.add(node);
            return;
        }

        CONFIG.regex.lastIndex = 0;
        const match = CONFIG.regex.exec(node.nodeValue);

        if (!match) {
            scannedNodes.add(node);
            return;
        }

        // FIX: Clean up any old UAAA buttons in this bubble to prevent duplicates
        // when React rerenders or expands the text.
        const oldUIs = parent.querySelectorAll('.uaaa-message-ui');
        oldUIs.forEach(ui => ui.remove());

        const cipherText = match[0];
        const cipherBlob = cipherText.replace(/\s/g, '').toUpperCase();
        let decryptedCache = null;
        let isDecrypted = false;

        const container = document.createElement('span');
        container.className = 'uaaa-btn-group uaaa-message-ui';
        container.contentEditable = "false";

        const toggleBtn = document.createElement('button');
        toggleBtn.className = 'uaaa-btn';
        toggleBtn.type = 'button';
        toggleBtn.innerHTML = svgIcon(UNLOCK_PATH, 16) + '<span>Decrypt</span>';
        toggleBtn.title = 'Decrypt cipher text';
        toggleBtn.setAttribute('aria-label', 'Decrypt cipher text');

        const infoBtn = document.createElement('a');
        infoBtn.className = 'uaaa-btn link';
        infoBtn.innerHTML = svgIcon(INFO_PATH, 16) + '<span>Info</span>';
        infoBtn.href = `${CONFIG.url}?m=dec&d=${encodeURIComponent(cipherBlob)}`;
        infoBtn.target = '_blank';
        infoBtn.rel = 'noopener noreferrer';
        infoBtn.title = 'Open in UAAAEncryption';
        infoBtn.setAttribute('aria-label', 'Open in UAAAEncryption');

        container.appendChild(toggleBtn);
        container.appendChild(infoBtn);

        // FIX: Append to the parent container instead of splitting the text node
        parent.appendChild(container);

        function setToggleIcon(locked, errMsg) {
            const label = errMsg ? 'Error' : (locked ? 'Decrypt' : 'Encrypt');
            toggleBtn.innerHTML = svgIcon(locked ? UNLOCK_PATH : LOCK_PATH, 16) + '<span>' + label + '</span>';
            toggleBtn.className = 'uaaa-btn' + (errMsg ? ' error' : (locked ? '' : ' success'));
            toggleBtn.title = errMsg || (locked ? 'Decrypt cipher text' : 'Re-encrypt text');
            toggleBtn.setAttribute('aria-label', toggleBtn.title);
        }

        function doDecrypt() {
            if (!decryptedCache) {
                try {
                    if (typeof _C !== 'function') throw new Error("Core API (_C) missing");
                    decryptedCache = _C(cipherBlob, CONFIG.key, false);
                } catch (err) {
                    setToggleIcon(true, err.message);
                    return false;
                }
            }

            ourInternalMutations.add(node);
            // Seamlessly replace just the cipher string, leaving React's node intact
            node.nodeValue = node.nodeValue.replace(cipherText, decryptedCache);

            setToggleIcon(false);
            isDecrypted = true;
            return true;
        }

        function doEncrypt() {
            ourInternalMutations.add(node);
            // Restore the exact cipher string
            if (decryptedCache) {
                node.nodeValue = node.nodeValue.replace(decryptedCache, cipherText);
            }

            setToggleIcon(true);
            isDecrypted = false;
        }

        toggleBtn.addEventListener('click', (e) => {
            e.preventDefault();
            e.stopPropagation();
            if (!isDecrypted) doDecrypt();
            else doEncrypt();
        });

        if (defaultMode === 'decrypted') {
            doDecrypt();
        }

        const instance = {
            get isDecrypted() { return isDecrypted; },
            applyMode(mode) {
                if (mode === 'decrypted' && !isDecrypted) doDecrypt();
                else if (mode === 'encrypted' && isDecrypted) doEncrypt();
            }
        };
        instances.add(instance);
        if (instances.size > 5000) instances.clear();
        scannedNodes.add(node);
    }

    // --- Performance Queuing ---
    function processQueue() {
        const startTime = performance.now();

        for (const node of nodeQueue) {
            if (performance.now() - startTime > 15) break;

            nodeQueue.delete(node);
            processTextNode(node);
        }

        if (nodeQueue.size > 0) {
            window.requestAnimationFrame(processQueue);
        } else {
            isProcessingQueue = false;
        }
    }

    function enqueueNode(node) {
        if (node.nodeType === Node.TEXT_NODE) {
            nodeQueue.add(node);
        } else if (node.nodeType === Node.ELEMENT_NODE && !CONFIG.ignoreTags.has(node.tagName)) {
            const walker = document.createTreeWalker(node, NodeFilter.SHOW_TEXT, null, false);
            let currentNode;
            while ((currentNode = walker.nextNode())) {
                nodeQueue.add(currentNode);
            }
        }

        if (!isProcessingQueue && nodeQueue.size > 0) {
            isProcessingQueue = true;
            window.requestAnimationFrame(processQueue);
        }
    }

    // --- Composer encrypt button: clone of WhatsApp's own voice button ---
    const composerCrypt = { plaintext: '', isEncrypted: false };
    let suppressInputReset = false;

    function getEditableHost(el) {
        if (!el) return null;
        if (el.isContentEditable) return el;
        return el.closest ? el.closest('[contenteditable="true"]') : null;
    }

    // Reads the current text out of either a native form field or a contenteditable box.
    function getElText(el) {
        return el.isContentEditable ? el.innerText : el.value;
    }

    // Writes text back in a way that framework-controlled fields (React, etc.) actually notice.
    function setElText(el, text) {
        el.focus();
        if (el.isContentEditable) {
            const range = document.createRange();
            range.selectNodeContents(el);
            const sel = window.getSelection();
            sel.removeAllRanges();
            sel.addRange(range);
            const inserted = document.execCommand && document.execCommand('insertText', false, text);
            if (!inserted) {
                el.textContent = text;
                el.dispatchEvent(new Event('input', { bubbles: true }));
            }
        } else {
            const proto = el.tagName === 'TEXTAREA' ? window.HTMLTextAreaElement.prototype : window.HTMLInputElement.prototype;
            const nativeSetter = Object.getOwnPropertyDescriptor(proto, 'value').set;
            nativeSetter.call(el, text);
            el.dispatchEvent(new Event('input', { bubbles: true }));
        }
    }

    function setComposerIcon(btn, locked) {
        try {
            btn.innerHTML = svgIcon(locked ? LOCK_PATH : UNLOCK_PATH, 24);
            btn.setAttribute('aria-label', locked ? 'Encrypt with UAAA' : 'Restore original text');
            btn.setAttribute('title', locked ? 'Encrypt with UAAA' : 'Restore original text');
        } catch (e) {}
    }

    function resetComposerCrypt() {
        composerCrypt.plaintext = '';
        composerCrypt.isEncrypted = false;
        try {
            const b = document.querySelector('[data-uaaa-encrypt]');
            if (b) setComposerIcon(b, true);
        } catch (e) {}
    }

    function findComposer() {
        try {
            const editable = document.querySelector('footer div[contenteditable="true"], footer [role="textbox"][contenteditable="true"]');
            if (!editable) return null;
            const footer = editable.closest ? editable.closest('footer') : null;
            if (!footer) return null;
            return { editable, footer };
        } catch (e) {
            return null;
        }
    }

    function validateEncryptPlacement() {
        try {
            if (!document.querySelector('[data-uaaa-encrypt]')) {
                ensureEncryptButton();
            }
        } catch (e) {}
    }

    function toggleComposerCrypt(editable, btn) {
        try {
            const host = getEditableHost(editable) || editable;
            if (!composerCrypt.isEncrypted) {
                const plaintext = getElText(host);
                if (!plaintext || !plaintext.trim()) return;
                if (typeof _C !== 'function') throw new Error('Core API (_C) missing');
                const cipher = _C(plaintext, CONFIG.key, true);
                composerCrypt.plaintext = plaintext;
                suppressInputReset = true;
                try {
                    setElText(host, cipher);
                } finally {
                    suppressInputReset = false;
                }
                composerCrypt.isEncrypted = true;
                setComposerIcon(btn, false);
            } else {
                suppressInputReset = true;
                try {
                    setElText(host, composerCrypt.plaintext);
                } finally {
                    suppressInputReset = false;
                }
                resetComposerCrypt();
            }
            try { host.focus(); } catch (e) {}
        } catch (err) {
            console.warn('[UAAA] composer crypt skipped:', err && err.message);
        }
    }

    function ensureEncryptButton() {
        try {
            if (document.querySelector('[data-uaaa-encrypt]')) return;
            const found = findComposer();
            if (!found) return;
            const host = getEditableHost(found.editable) || found.editable;
            const btn = document.createElement('button');
            btn.type = 'button';
            btn.className = 'uaaa-compose-btn';
            btn.setAttribute('data-uaaa-encrypt', '1');
            setComposerIcon(btn, true);
            btn.addEventListener('click', function (e) {
                e.preventDefault();
                e.stopPropagation();
                toggleComposerCrypt(host, btn);
            }, true);
            if (!host.parentNode) return;
            if (host.nextSibling) host.parentNode.insertBefore(btn, host.nextSibling);
            else host.parentNode.appendChild(btn);
        } catch (err) {
            console.warn('[UAAA] encrypt button skipped:', err && err.message);
        }
    }

    document.addEventListener('input', function (e) {
        try {
            if (suppressInputReset) return;
            if (getEditableHost(e.target)) resetComposerCrypt();
        } catch (err) {}
    }, true);

    // --- Initialization & Observation ---
    injectStyles();
    enqueueNode(document.body);
    ensureEncryptButton();
    try {
        setInterval(validateEncryptPlacement, 2500);
    } catch (e) {}

    const observer = new MutationObserver((mutations) => {
      try {
        for (const mutation of mutations) {
            if (mutation.type === 'characterData') {
                const target = mutation.target;

                // 1. Ignore mutations caused by our script replacing the text
                if (ourInternalMutations.has(target)) {
                    ourInternalMutations.delete(target);
                    continue;
                }

                // 2. WhatsApp dynamically changed the text (e.g. user clicked Read More):
                // content changed, so allow a rescan of this node.
                scannedNodes.delete(target);
                enqueueNode(target);

            } else if (mutation.type === 'childList') {
                for (const addedNode of mutation.addedNodes) {
                    enqueueNode(addedNode);
                }
            }
        }
        ensureEncryptButton();
      } catch (e) {
        console.warn('[UAAA] observer skipped batch:', e && e.message);
      }
    });

    observer.observe(document.body, {
        childList: true,
        subtree: true,
        characterData: true
    });
})();
