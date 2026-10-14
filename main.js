'use strict';

const path = require('path');
const fs = require('fs');
const os = require('os');
const {
  app,
  BrowserWindow,
  WebContentsView,
  Tray,
  Menu,
  nativeImage,
  nativeTheme,
  session,
  shell,
  ipcMain,
} = require('electron');

const WHATSAPP_HOME = 'https://web.whatsapp.com/';

function chromeUA() {
  return (
    'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) ' +
    `Chrome/${process.versions.chrome} Safari/537.36`
  );
}

function verbose() {
  const v = process.env.RUWA_VERBOSE || process.env.RUWA_LOG || '';
  return v === '1' || v === 'debug' || v === 'verbose' || v === 'trace';
}

function vlog(msg) {
  if (verbose()) console.error(`ruwa: ${msg}`);
}

function homeDir() {
  return process.env.HOME || '/tmp';
}

function configDir() {
  const x = process.env.XDG_CONFIG_HOME;
  if (x && x.trim()) return path.join(x, 'ruwa');
  return path.join(homeDir(), '.config', 'ruwa');
}

function dataRoot() {
  const override = process.env.RUWA_DATA_DIR;
  if (override && override.trim()) return override;
  const x = process.env.XDG_DATA_HOME;
  const base = x && x.trim() ? x : path.join(homeDir(), '.local', 'share');
  return path.join(base, 'ruwa');
}

function configPath() {
  return path.join(configDir(), 'config.json');
}

function customCssPath() {
  return path.join(configDir(), 'custom.css');
}

function downloadsDir() {
  return path.join(homeDir(), 'Downloads');
}

const CUSTOM_CSS_SEED = `/* Ruwa custom CSS: applied to WhatsApp Web on every launch.
   Plain CSS, no wrapper needed. Reload the page to apply edits. */

#app {
}
`;

const DISPLAYS = ['auto', 'wayland', 'x11'];

function validModId(id) {
  return (
    typeof id === 'string' &&
    id.length > 0 &&
    id.length <= 64 &&
    /^[a-z0-9-]+$/.test(id) &&
    !id.startsWith('-') &&
    !id.endsWith('-')
  );
}

function sanitizeModIds(ids) {
  return [...new Set((ids || []).filter(validModId))].sort();
}

function defaultConfig() {
  return {
    close_to_tray: true,
    start_minimized: false,
    display: 'auto',
    mods_enabled: [],
    engine: defaultEngine(),
  };
}

function defaultEngine() {
  return {
    diskCacheMB: 0,
    persistentCookies: true,
    gpuSameProcess: true,
    disableGpu: false,
    disableVsync: false,
    forceSoftwareRendering: false,
    singleProcess: false,
    processPerSite: true,
    jsMemoryMB: 0,
    animatedScrolling: true,
    backgroundOptimizations: true,
    disablePageAnimations: false,
    disablePinchZoom: false,
  };
}

function sanitizeEngine(raw) {
  const d = defaultEngine();
  if (!raw || typeof raw !== 'object') return d;
  const num = (v) => {
    const n = parseInt(v, 10);
    return Number.isFinite(n) && n >= 0 ? Math.min(n, 65536) : 0;
  };
  const flag = (v, fb) => (typeof v === 'boolean' ? v : fb);
  return {
    diskCacheMB: num(raw.diskCacheMB),
    persistentCookies: flag(raw.persistentCookies, d.persistentCookies),
    gpuSameProcess: flag(raw.gpuSameProcess, d.gpuSameProcess),
    disableGpu: flag(raw.disableGpu, d.disableGpu),
    disableVsync: flag(raw.disableVsync, d.disableVsync),
    forceSoftwareRendering: flag(raw.forceSoftwareRendering, d.forceSoftwareRendering),
    singleProcess: flag(raw.singleProcess, d.singleProcess),
    processPerSite: flag(raw.processPerSite, d.processPerSite),
    jsMemoryMB: num(raw.jsMemoryMB),
    animatedScrolling: flag(raw.animatedScrolling, d.animatedScrolling),
    backgroundOptimizations: flag(raw.backgroundOptimizations, d.backgroundOptimizations),
    disablePageAnimations: flag(raw.disablePageAnimations, d.disablePageAnimations),
    disablePinchZoom: flag(raw.disablePinchZoom, d.disablePinchZoom),
  };
}

function legacyConfigFromUserJs(content) {
  const cfg = defaultConfig();
  let found = false;
  const boolPref = (key) => {
    const m = content.match(new RegExp(`user_pref\\("${key}",\\s*(true|false)\\)`));
    return m ? m[1] === 'true' : undefined;
  };
  const strPref = (key) => {
    const m = content.match(new RegExp(`user_pref\\("${key}",\\s*"([^"]*)"\\)`));
    return m ? m[1] : undefined;
  };
  let v = boolPref('ruwa.close_to_tray');
  if (v !== undefined) { cfg.close_to_tray = v; found = true; }
  v = boolPref('ruwa.start_minimized');
  if (v !== undefined) { cfg.start_minimized = v; found = true; }
  const d = strPref('ruwa.display.backend');
  if (d !== undefined) { cfg.display = DISPLAYS.includes(d) ? d : 'auto'; found = true; }
  const m = strPref('ruwa.mods.enabled');
  if (m !== undefined) { cfg.mods_enabled = sanitizeModIds(m.split(/[\s,]+/)); found = true; }
  return found ? cfg : null;
}

function readConfig() {
  try {
    const raw = fs.readFileSync(configPath(), 'utf8');
    const parsed = JSON.parse(raw);
    const cfg = defaultConfig();
    if (typeof parsed.close_to_tray === 'boolean') cfg.close_to_tray = parsed.close_to_tray;
    if (typeof parsed.start_minimized === 'boolean') cfg.start_minimized = parsed.start_minimized;
    if (DISPLAYS.includes(parsed.display)) cfg.display = parsed.display;
    if (Array.isArray(parsed.mods_enabled)) cfg.mods_enabled = sanitizeModIds(parsed.mods_enabled);
    cfg.engine = sanitizeEngine(parsed.engine);
    return cfg;
  } catch (e) {
    try {
      const legacyProfile = process.env.RUWA_PROFILE && process.env.RUWA_PROFILE.trim()
        ? process.env.RUWA_PROFILE
        : path.join(dataRoot(), 'profile');
      const migrated = legacyConfigFromUserJs(
        fs.readFileSync(path.join(legacyProfile, 'user.js'), 'utf8')
      );
      if (migrated) {
        writeConfig(migrated);
        return migrated;
      }
    } catch (_) {}
    return defaultConfig();
  }
}

function writeConfig(cfg) {
  const clean = defaultConfig();
  clean.close_to_tray = !!cfg.close_to_tray;
  clean.start_minimized = !!cfg.start_minimized;
  clean.display = DISPLAYS.includes(cfg.display) ? cfg.display : 'auto';
  clean.mods_enabled = sanitizeModIds(cfg.mods_enabled);
  clean.engine = sanitizeEngine(cfg.engine);
  fs.mkdirSync(configDir(), { recursive: true });
  fs.writeFileSync(configPath(), JSON.stringify(clean, null, 2) + '\n');
  return clean;
}

function readCustomCss() {
  try {
    return fs.readFileSync(customCssPath(), 'utf8');
  } catch (_) {
    return '';
  }
}

function writeCustomCss(css) {
  let depth = 0;
  let inComment = false;
  let inStr = null;
  for (let i = 0; i < css.length; i++) {
    const c = css[i];
    if (inComment) {
      if (c === '*' && css[i + 1] === '/') { inComment = false; i++; }
      continue;
    }
    if (inStr) {
      if (c === '\\') i++;
      else if (c === inStr) inStr = null;
      continue;
    }
    if (c === '/' && css[i + 1] === '*') { inComment = true; i++; continue; }
    if (c === '"' || c === "'") { inStr = c; continue; }
    if (c === '{') depth++;
    if (c === '}') {
      depth--;
      if (depth < 0) throw new Error("unmatched closing brace '}'");
    }
  }
  if (inComment) throw new Error('unterminated /* comment */');
  if (depth !== 0) throw new Error(`unbalanced braces: ${depth} unclosed '{'`);
  fs.mkdirSync(configDir(), { recursive: true });
  fs.writeFileSync(customCssPath(), css);
}

function stripMozDocument(css) {
  let out = '';
  let i = 0;
  while (i < css.length) {
    if (css.startsWith('@-moz-document', i)) {
      const open = css.indexOf('{', i);
      if (open === -1) { out += css.slice(i); break; }
      let depth = 0;
      let j = open;
      let inComment = false;
      let inStr = null;
      for (; j < css.length; j++) {
        const c = css[j];
        if (inComment) {
          if (c === '*' && css[j + 1] === '/') { inComment = false; j++; }
          continue;
        }
        if (inStr) {
          if (c === '\\') j++;
          else if (c === inStr) inStr = null;
          continue;
        }
        if (c === '/' && css[j + 1] === '*') { inComment = true; j++; continue; }
        if (c === '"' || c === "'") { inStr = c; continue; }
        if (c === '{') depth++;
        if (c === '}') { depth--; if (depth === 0) break; }
      }
      out += css.slice(open + 1, Math.min(j, css.length)) + '\n';
      i = Math.min(j + 1, css.length);
      continue;
    }
    out += css[i];
    i++;
  }
  return out;
}

function whatsappHost(host) {
  const h = String(host || '').trim().replace(/\.+$/, '').toLowerCase();
  return ['whatsapp.com', 'whatsapp.net', 'wa.me'].some(
    (b) => h === b || h.endsWith('.' + b)
  );
}

function navigationAllowed(raw) {
  try {
    const u = new URL(raw);
    if (u.protocol === 'http:' || u.protocol === 'https:') {
      return whatsappHost(u.hostname);
    }
    return ['about:', 'blob:', 'data:'].includes(u.protocol);
  } catch (_) {
    return false;
  }
}

function appDir() {
  return app.getAppPath();
}

function modsDir() {
  const override = process.env.RUWA_DIR;
  if (override && override.trim()) return path.join(override, 'mods');
  const local = path.join(appDir(), 'mods');
  if (fs.existsSync(local)) return local;
  const packaged = path.join(path.dirname(appDir()), 'mods');
  if (fs.existsSync(packaged)) return packaged;
  return local;
}

function iconPath() {
  const local = path.join(appDir(), 'icon.png');
  if (fs.existsSync(local)) return local;
  return path.join(__dirname, '..', 'data', 'icons', 'ruwa.png');
}

function baseCss() {
  for (const p of [path.join(appDir(), 'base.css'), path.join(__dirname, '..', 'assets', 'base.css')]) {
    try {
      return fs.readFileSync(p, 'utf8');
    } catch (_) {}
  }
  return '';
}

function listMods() {
  const dir = modsDir();
  const out = [];
  let entries = [];
  try {
    entries = fs.readdirSync(dir, { withFileTypes: true });
  } catch (_) {
    return out;
  }
  for (const e of entries) {
    if (!e.isDirectory() || !validModId(e.name)) continue;
    try {
      const meta = JSON.parse(fs.readFileSync(path.join(dir, e.name, 'mod.json'), 'utf8'));
      if (meta.id !== e.name || typeof meta.name !== 'string') continue;
      const files = meta.files;
      if (!Array.isArray(files) || files.length === 0 || !files.every((f) => typeof f === 'string')) continue;
      out.push({
        id: e.name,
        name: meta.name,
        version: typeof meta.version === 'string' ? meta.version : '',
        description: typeof meta.description === 'string' ? meta.description : '',
      });
    } catch (_) {}
  }
  out.sort((a, b) => a.id.localeCompare(b.id));
  return out;
}

function buildModsRegistry(enabled) {
  const dir = modsDir();
  const chunks = [];
  const ids = [];
  for (const id of sanitizeModIds(enabled)) {
    let meta;
    try {
      meta = JSON.parse(fs.readFileSync(path.join(dir, id, 'mod.json'), 'utf8'));
    } catch (_) {
      continue;
    }
    if (meta.id !== id || !Array.isArray(meta.files)) continue;
    const parts = [];
    let ok = true;
    for (const rel of meta.files) {
      if (typeof rel !== 'string' || rel.startsWith('/') || rel.split('/').includes('..')) { ok = false; break; }
      const full = path.join(dir, id, rel);
      if (!full.startsWith(path.join(dir, id) + path.sep) || !fs.existsSync(full)) { ok = false; break; }
      parts.push(fs.readFileSync(full, 'utf8'));
    }
    if (!ok) continue;
    ids.push(id);
    chunks.push(`  ${JSON.stringify(id)}: function() {\n${parts.join('\n')}\n  }`);
  }
  const boot =
    '\n;(function(){var enabled=' + JSON.stringify(ids) + ',seen={};' +
    'for(var i=0;i<enabled.length;i++){var id=enabled[i];' +
    'if(!id||seen[id])continue;seen[id]=true;' +
    'var fn=(typeof RUWA_MODS!=="undefined")?RUWA_MODS[id]:null;' +
    'if(typeof fn!=="function")continue;' +
    'try{fn();}catch(e){console.warn("[ruwa-mods] mod failed: "+id,e);}}})();';
  return `var RUWA_MOD_IDS=${JSON.stringify(ids)};\nvar RUWA_MODS={\n${chunks.join(',\n')}\n};${boot}`;
}

const REBRAND_JS = `(function(){
if (window.__ruwaRebrand) return;
window.__ruwaRebrand = true;
var seen = new WeakSet();
var queued = false;
function rename(node) {
  try {
    if (!node.nodeValue || seen.has(node)) return;
    var p = node.parentNode;
    if (!p || !document.body.contains(node)) return;
    var tag = p.tagName;
    if (p.isContentEditable || tag === 'SCRIPT' || tag === 'STYLE' || tag === 'TEXTAREA' || tag === 'INPUT' || tag === 'NOSCRIPT') return;
    if (p.closest && p.closest('.uaaa-message-ui')) return;
    var v = node.nodeValue;
    if (v.indexOf('WhatsApp') === -1 && v.indexOf('whatsapp') === -1 && v.indexOf('WHATSAPP') === -1) { seen.add(node); return; }
    var nv = v.replace(/WHATSAPP/g, 'RUWA').replace(/WhatsApp/g, 'Ruwa').replace(/whatsapp/g, 'ruwa');
    if (nv !== v) node.nodeValue = nv;
    seen.add(node);
  } catch (e) { console.warn('[ruwa] rebrand skipped:', e && e.message); }
}
function sweep(root) {
  try {
    var walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, null, false);
    var batch = [], n, count = 0;
    while ((n = walker.nextNode())) batch.push(n);
    for (var i = 0; i < batch.length && count < 600; i++) { rename(batch[i]); count++; }
  } catch (e) { console.warn('[ruwa] rebrand sweep skipped:', e && e.message); }
}
function schedule() {
  if (queued) return;
  queued = true;
  (window.requestAnimationFrame || function (f) { setTimeout(f, 100); })(function () {
    queued = false;
    try { sweep(document.body); } catch (e) {}
  });
}
try {
  sweep(document.body);
  new MutationObserver(function (muts) {
    try {
      for (var i = 0; i < muts.length; i++) {
        var m = muts[i];
        if (m.type === 'characterData') { seen.delete(m.target); rename(m.target); }
        else if (m.type === 'childList') {
          for (var j = 0; j < m.addedNodes.length; j++) {
            var a = m.addedNodes[j];
            if (a.nodeType === 3) { seen.delete(a); rename(a); }
          }
          schedule();
        }
      }
    } catch (e) {}
  }).observe(document.body, { childList: true, subtree: true, characterData: true });
} catch (e) { console.warn('[ruwa] rebrand off:', e && e.message); }
})();`;

function autostartPath() {
  const x = process.env.XDG_CONFIG_HOME;
  const base = x && x.trim() ? x : path.join(homeDir(), '.config');
  return path.join(base, 'autostart', 'ruwa.desktop');
}

function themeState() {
  return { dark: nativeTheme.shouldUseDarkColors };
}

function desktopEntry(exec, minimized) {
  const execLine = minimized ? `${exec} --minimized` : exec;
  return (
    '[Desktop Entry]\nType=Application\nName=Ruwa\nGenericName=WhatsApp Client\n' +
    'Comment=Lightweight WhatsApp desktop client\n' +
    `TryExec=${exec}\nExec=${execLine} \nIcon=ruwa\nTerminal=false\n` +
    'Categories=Network;InstantMessaging;\nStartupWMClass=Ruwa\nStartupNotify=true\n' +
    'X-GNOME-Autostart-enabled=true\n'
  );
}

function currentExe() {
  return process.execPath;
}

let win = null;
let mainView = null;
let panelView = null;
let tray = null;
let ses = null;
let cfg = defaultConfig();
let panelOpen = false;
let quitting = false;

function selectSession() {
  if (cfg.engine.persistentCookies) return session.defaultSession;
  return session.fromPartition('inmemory:ruwa', { cache: false });
}

function detectGpu() {
  try {
    const entries = fs.readdirSync('/dev/dri');
    if (entries.some((n) => n.startsWith('renderD') || n.startsWith('card'))) return true;
  } catch (_) {}
  return false;
}

function runMarkerPath() {
  return path.join(dataRoot(), 'electron', 'run.json');
}

function readRunMarker() {
  try {
    return JSON.parse(fs.readFileSync(runMarkerPath(), 'utf8'));
  } catch (_) {
    return null;
  }
}

function writeRunMarker(marker) {
  try {
    fs.mkdirSync(path.dirname(runMarkerPath()), { recursive: true });
    fs.writeFileSync(runMarkerPath(), JSON.stringify(marker));
  } catch (_) {}
}

function autoEngine(stored) {
  const eng = sanitizeEngine(stored);
  const hwPresent = detectGpu();
  const last = readRunMarker();
  const crashedWithGpu = !!(last && last.cleanExit === false && last.gpu);
  let gpu = hwPresent;
  if (crashedWithGpu) {
    gpu = false;
    vlog('previous run crashed with GPU enabled; falling back to software rendering');
  }
  eng.disableGpu = !gpu;
  eng.forceSoftwareRendering = false;
  eng.gpuSameProcess = false;
  eng.singleProcess = false;
  eng.processPerSite = true;
  eng.jsMemoryMB = 0;
  eng.diskCacheMB = 0;
  eng.animatedScrolling = true;
  eng.backgroundOptimizations = true;
  eng.disablePageAnimations = false;
  eng.disablePinchZoom = false;
  vlog(`engine auto: gpu=${gpu} (dri=${hwPresent} lastCrash=${crashedWithGpu})`);
  return { eng, gpu };
}

function applyEngineFlags(eng) {
  const cmd = app.commandLine;
  if (eng.disableGpu || eng.forceSoftwareRendering) {
    app.disableHardwareAcceleration();
  }
  if (eng.forceSoftwareRendering) {
    cmd.appendSwitch('enable-unsafe-swiftshader');
  }
  if (eng.gpuSameProcess) {
    cmd.appendSwitch('in-process-gpu');
  }
  if (eng.disableVsync) {
    cmd.appendSwitch('disable-gpu-vsync');
  }
  if (eng.singleProcess) {
    cmd.appendSwitch('single-process');
  }
  if (eng.processPerSite) {
    cmd.appendSwitch('process-per-site');
  }
  if (eng.jsMemoryMB > 0) {
    cmd.appendSwitch('js-flags', `--max-old-space-size=${eng.jsMemoryMB}`);
  }
  if (eng.diskCacheMB > 0) {
    cmd.appendSwitch('disk-cache-size', String(eng.diskCacheMB * 1048576));
  }
  if (!eng.animatedScrolling) {
    cmd.appendSwitch('disable-smooth-scrolling');
  }
  if (!eng.backgroundOptimizations) {
    cmd.appendSwitch('disable-background-timer-throttling');
    cmd.appendSwitch('disable-backgrounding-occluded-windows');
    cmd.appendSwitch('disable-renderer-backgrounding');
  }
  if (eng.disablePageAnimations) {
    cmd.appendSwitch('disable-animations');
  }
  if (eng.disablePinchZoom) {
    cmd.appendSwitch('disable-pinch');
  }
}

function showMain() {
  if (!win) return;
  if (win.isMinimized()) win.restore();
  win.show();
  win.focus();
}

function reloadMain() {
  if (mainView) {
    vlog('reloading main view');
    mainView.webContents.reload();
  }
}

function openExternal(url) {
  vlog(`opening externally: ${url}`);
  shell.openExternal(url).catch((e) => console.error(`ruwa: cannot open URL ${url}: ${e}`));
}

function layout() {
  if (!win || !mainView || !panelView) return;
  const [w, h] = win.getContentSize();
  const W = Math.max(100, Math.floor(w));
  const H = Math.max(100, Math.floor(h));
  if (panelOpen) {
    mainView.setVisible(false);
    panelView.setBounds({ x: 0, y: 0, width: W, height: H });
    panelView.setVisible(true);
  } else {
    panelView.setVisible(false);
    mainView.setBounds({ x: 0, y: 0, width: W, height: H });
    mainView.setVisible(true);
  }
}

function setPanel(open) {
  panelOpen = open;
  layout();
  if (open && panelView) panelView.webContents.focus();
  else if (mainView) mainView.webContents.focus();
}

function togglePanel() {
  showMain();
  setPanel(!panelOpen);
}

function openSettings() {
  togglePanel();
}

function updateTitle() {
  if (win) {
    try {
      win.setTitle('Ruwa');
    } catch (_) {}
  }
  if (tray) {
    try {
      tray.setToolTip('Ruwa');
    } catch (_) {}
  }
}

function createTray() {
  try {
    const img = nativeImage.createFromPath(iconPath()).resize({ width: 64, height: 64 });
    tray = new Tray(img);
    tray.setToolTip('Ruwa');
    tray.setContextMenu(
      Menu.buildFromTemplate([
        { label: 'Show', click: () => showMain() },
        { label: 'Reload', click: () => reloadMain() },
        { label: 'Settings', click: () => openSettings() },
        { type: 'separator' },
        {
          label: 'Quit',
          click: () => {
            quitting = true;
            app.quit();
          },
        },
      ])
    );
    tray.on('click', () => showMain());
  } catch (e) {
    console.error(`ruwa: tray unavailable (${e}), continuing without it`);
    tray = null;
  }
}

const KNOWN_FLAGS = new Set(['--show', '--quit', '--minimized', '--reload', '-h', '--help']);

function userArgs(argv) {
  return argv.slice(1).filter((a) => KNOWN_FLAGS.has(a));
}

function createWindow(startHidden) {
  ses.setUserAgent(chromeUA());
  vlog(`user agent: ${chromeUA()}`);

  win = new BrowserWindow({
    width: 1280,
    height: 800,
    minWidth: 800,
    minHeight: 600,
    title: 'Ruwa',
    show: false,
    backgroundColor: '#1d1f1f',
  });

  Menu.setApplicationMenu(
    Menu.buildFromTemplate([
      {
        label: 'Ruwa',
        submenu: [
          { label: 'Reload', accelerator: 'CmdOrCtrl+R', click: () => reloadMain() },
          { label: 'Settings', accelerator: 'CmdOrCtrl+,', click: () => togglePanel() },
          { type: 'separator' },
          {
            label: 'Quit',
            accelerator: 'CmdOrCtrl+Q',
            click: () => {
              quitting = true;
              app.quit();
            },
          },
        ],
      },
    ])
  );

  mainView = new WebContentsView({
    webPreferences: {
      session: ses,
      contextIsolation: true,
      sandbox: false,
      backgroundThrottling: false,
    },
  });

  panelView = new WebContentsView({
    webPreferences: {
      session: ses,
      preload: path.join(appDir(), 'preload-panel.js'),
      contextIsolation: true,
      sandbox: true,
    },
  });

  win.contentView.addChildView(mainView);
  win.contentView.addChildView(panelView);
  setPanel(process.env.RUWA_PANEL === '1');

  mainView.webContents.loadURL(WHATSAPP_HOME);
  panelView.webContents.loadFile(path.join(appDir(), 'panel.html'));

  ses.setPermissionRequestHandler((wc, permission, callback) => {
    const url = wc.getURL ? wc.getURL() : '';
    vlog(`permission '${permission}' from ${url}`);
    if (!navigationAllowed(url)) {
      callback(false);
      return;
    }
    callback(
      [
        'media',
        'audio-capture',
        'video-capture',
        'display-capture',
        'notifications',
        'geolocation',
        'fullscreen',
        'pointerLock',
        'keyboard-lock',
        'persistent-storage',
        'idle-detection',
      ].includes(permission)
    );
  });

  ses.on('will-download', (e, item) => {
    try {
      const name = item.getFilename() || 'whatsapp-download';
      fs.mkdirSync(downloadsDir(), { recursive: true });
      item.setSavePath(path.join(downloadsDir(), path.basename(name)));
      vlog(`download: ${name}`);
    } catch (err) {
      console.error(`ruwa: download failed: ${err}`);
    }
  });

  mainView.webContents.setWindowOpenHandler(({ url }) => {
    if (navigationAllowed(url)) {
      mainView.webContents.loadURL(url);
    } else {
      openExternal(url);
    }
    return { action: 'deny' };
  });
  mainView.webContents.on('will-navigate', (e, url) => {
    if (!navigationAllowed(url)) {
      e.preventDefault();
      openExternal(url);
    }
  });

  mainView.webContents.on('dom-ready', () => {
    const url = mainView.webContents.getURL();
    if (!navigationAllowed(url)) return;
    const css = baseCss() + '\n' + stripMozDocument(readCustomCss());
    if (css.trim()) {
      mainView.webContents.insertCSS(css).catch(() => {});
    }
    const enabled = readConfig().mods_enabled;
    vlog(`injecting ${enabled.length} mods into ${url}`);
    const js = buildModsRegistry(enabled);
    mainView.webContents.executeJavaScript(js, true).catch(() => {});
    mainView.webContents.executeJavaScript(REBRAND_JS, true).catch(() => {});
  });

  mainView.webContents.on('page-title-updated', () => {
    updateTitle();
  });

  for (const [label, wc] of [
    ['main', mainView.webContents],
    ['panel', panelView.webContents],
  ]) {
    wc.on('did-fail-load', (e, code, desc, url) => {
      console.error(`ruwa: ${label} view failed to load ${url}: ${code} ${desc}`);
    });
  }

  win.on('resize', layout);

  win.on('close', (e) => {
    if (!quitting && readConfig().close_to_tray) {
      e.preventDefault();
      win.hide();
    }
  });

  if (startHidden) {
    console.log('ruwa: starting minimized to tray');
  } else {
    win.show();
  }
  layout();
}

function registerIpc() {
  ipcMain.handle('panel:get-config', () => readConfig());
  ipcMain.handle('panel:set-config', (e, patch) => {
    cfg = writeConfig({ ...readConfig(), ...(patch || {}) });
    return cfg;
  });
  ipcMain.handle('panel:get-css', () => {
    const cur = readCustomCss();
    return cur.trim() ? cur : CUSTOM_CSS_SEED;
  });
  ipcMain.handle('panel:set-css', (e, css) => {
    writeCustomCss(String(css || ''));
    return true;
  });
  ipcMain.handle('panel:list-mods', () => ({ available: listMods(), enabled: readConfig().mods_enabled }));
  ipcMain.handle('panel:set-mods', (e, ids) => {
    cfg = writeConfig({ ...readConfig(), mods_enabled: sanitizeModIds(ids) });
    return cfg.mods_enabled;
  });
  ipcMain.handle('panel:get-autostart', () => fs.existsSync(autostartPath()));
  ipcMain.handle('panel:set-autostart', (e, on) => {
    const p = autostartPath();
    if (on) {
      fs.mkdirSync(path.dirname(p), { recursive: true });
      fs.writeFileSync(p, desktopEntry(currentExe(), readConfig().start_minimized));
    } else {
      try {
        fs.unlinkSync(p);
      } catch (_) {}
    }
    return !!on;
  });
  ipcMain.handle('panel:logout', async () => {
    await ses.clearStorageData();
    reloadMain();
    return true;
  });
  ipcMain.handle('panel:reload-main', () => {
    reloadMain();
    return true;
  });
  ipcMain.handle('panel:get-version', () => app.getVersion());
  ipcMain.handle('panel:get-theme', () => themeState());
  nativeTheme.on('theme-updated', () => {
    if (panelView) {
      try {
        panelView.webContents.send('panel:theme', themeState());
      } catch (_) {}
    }
  });
}

function printUsage() {
  process.stdout.write(
    'ruwa — lightweight WhatsApp desktop client\n\nUsage: ruwa [options]\n\nOptions:\n' +
      '  --show        Focus the running window (default for second instance)\n' +
      '  --quit        Quit the running instance and exit\n' +
      '  --minimized   Start minimized to tray (no window)\n' +
      '  --reload      Ask the running instance to reload the page\n' +
      '  -h, --help    Show this help\n\nEnv:\n' +
      '  RUWA_BACKEND   auto|wayland|x11 (overrides Settings display backend)\n' +
      '  RUWA_VERBOSE=1 Extra logging\n'
  );
}

function parseCli(args) {
  const cli = { minimized: false, quit: false, reload: false };
  for (const a of args) {
    if (a === '--minimized') cli.minimized = true;
    else if (a === '--quit') cli.quit = true;
    else if (a === '--reload') cli.reload = true;
    else if (a === '--show') continue;
    else if (a === '-h' || a === '--help') {
      printUsage();
      process.exit(0);
    } else if (a.startsWith('-')) {
      console.error(`ruwa: unknown option '${a}'`);
      printUsage();
      process.exit(2);
    } else {
      console.error(`ruwa: unexpected argument '${a}'`);
      printUsage();
      process.exit(2);
    }
  }
  if (cli.quit && cli.minimized) {
    console.error('ruwa: --quit cannot be combined with --minimized');
    process.exit(2);
  }
  return cli;
}

function handleSecondInstance(argv) {
  const args = userArgs(argv);
  vlog(`second instance: ${JSON.stringify(args)}`);
  if (args.includes('--quit')) {
    quitting = true;
    app.quit();
  } else if (args.includes('--reload')) {
    reloadMain();
    showMain();
  } else {
    showMain();
  }
}

function applyDisplayBackend(display) {
  const backend = (process.env.RUWA_BACKEND || '').trim().toLowerCase() || display;
  if (backend === 'x11' || backend === 'xwayland') {
    delete process.env.WAYLAND_DISPLAY;
    process.env.GDK_BACKEND = 'x11';
  } else if (backend === 'wayland') {
    process.env.GDK_BACKEND = 'wayland';
  }
}

async function main() {
  const cli = parseCli(userArgs(process.argv));

  app.commandLine.appendSwitch('autoplay-policy', 'no-user-gesture-required');
  app.setPath('userData', path.join(dataRoot(), 'electron'));

  if (!app.requestSingleInstanceLock()) {
    app.quit();
    return;
  }
  app.on('second-instance', (e, argv) => handleSecondInstance(argv));

  cfg = readConfig();
  vlog(
    `config: close_to_tray=${cfg.close_to_tray} start_minimized=${cfg.start_minimized} ` +
      `display=${cfg.display} mods=${JSON.stringify(cfg.mods_enabled)}`
  );
  applyDisplayBackend(cfg.display);
  const auto = autoEngine(cfg.engine);
  cfg.engine = auto.eng;
  try {
    writeConfig(cfg);
  } catch (_) {}
  applyEngineFlags(cfg.engine);
  writeRunMarker({ cleanExit: false, gpu: auto.gpu, at: Date.now() });
  app.on('before-quit', () => writeRunMarker({ cleanExit: true, gpu: auto.gpu, at: Date.now() }));

  await app.whenReady();

  ses = selectSession();
  vlog(`session persistent=${cfg.engine.persistentCookies}`);

  if (cli.quit) {
    console.error('ruwa: not running');
    app.quit();
    return;
  }

  registerIpc();
  createTray();
  createWindow(cli.minimized || cfg.start_minimized);
}

main().catch((e) => {
  console.error(`ruwa: fatal: ${e && e.stack ? e.stack : e}`);
  process.exit(1);
});
