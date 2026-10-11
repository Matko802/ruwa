'use strict';
/* Ruwa mods loader: runs bundled premade mods inside WhatsApp Web.
   Which mods run is decided at Ruwa launch time and passed through a
   private CSS variable (--ruwa-mods) in userContent.css; this keeps the
   loader dependency-free (plain content script, no WebExtension APIs).
   Rebrand assembles the registry below from mods/<id>/ at package
   build time by replacing the __RUWA_MOD_REGISTRY__ marker. */

/*__RUWA_MOD_REGISTRY__*/

(function () {
    function readVar(name) {
        try {
            var root = document.documentElement;
            if (!root) return null;
            var v = getComputedStyle(root).getPropertyValue(name);
            if (!v) return null;
            return v.trim().replace(/^["']|["']$/g, '').trim();
        } catch (e) {
            return null;
        }
    }

    function boot(enabled) {
        var seen = {};
        for (var i = 0; i < enabled.length; i++) {
            var id = enabled[i];
            if (!id || seen[id]) continue;
            seen[id] = true;
            var fn = RUWA_MODS[id];
            if (typeof fn !== 'function') continue;
            try {
                fn();
            } catch (e) {
                console.warn('[ruwa-mods] mod failed: ' + id, e);
            }
        }
    }

    var attempts = 0;
    function tick() {
        if (readVar('--ruwa-present') !== '1') {
            attempts++;
            if (attempts < 20) {
                setTimeout(tick, 250);
                return;
            }
            console.warn('[ruwa-mods] user styles not detected, mods stay off');
            return;
        }
        var raw = readVar('--ruwa-mods') || '';
        var enabled = raw.split(/\s+/).filter(function (s) { return !!s; });
        boot(enabled);
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', tick);
    } else {
        tick();
    }
})();
