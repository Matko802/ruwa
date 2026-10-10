'use strict';

const { app, BrowserWindow, ipcMain, shell, Notification } = require('electron');
const fs = require('fs');

const WHATSAPP_HOME = 'https://web.whatsapp.com/';
const TEST_LOG = process.env.RUWA_TEST_LOG || '';

function testLog(line) {
  if (TEST_LOG) {
    try { fs.appendFileSync(TEST_LOG, line + '\n'); } catch (e) {}
  }
}

function whatsappHost(hostname) {
  const h = String(hostname || '').toLowerCase().replace(/\.$/, '');
  return h === 'whatsapp.com' || h === 'whatsapp.net' ||
    h.endsWith('.whatsapp.com') || h.endsWith('.whatsapp.net');
}

function isWhatsAppUrl(u) {
  try {
    return whatsappHost(new URL(u).hostname);
  } catch (e) {
    return false;
  }
}

function firstArgUrl() {
  if (process.env.RUWA_URL) return process.env.RUWA_URL;
  const a = process.argv.slice(2).find((x) => /^https?:\/\//.test(x));
  return a || WHATSAPP_HOME;
}

const INJECT = `(function(){
  if (window.__ruwa_injected) return; window.__ruwa_injected = true;
  function send(){ var a=[].slice.call(arguments); if (window.gelectron && window.gelectron.send) { window.gelectron.send.apply(null, a); } }
  function parseUnread(t){ var m=/^\\((\\d+)\\)/.exec(String(t||'')); return m ? parseInt(m[1],10) : 0; }
  var lastSent = -1;
  function checkTitle(){ var n=parseUnread(document.title); if(n!==lastSent){ lastSent=n; send('ruwa-unread', n); } }
  try {
    var titleEl = document.querySelector('head > title');
    new MutationObserver(checkTitle).observe(titleEl || document.documentElement, { subtree:true, childList:true, characterData:true });
  } catch(e){}
  checkTitle();
  setInterval(checkTitle, 10000);
  function hostOf(u){ try { return new URL(u, location.href).hostname.toLowerCase().replace(/\\.$/,''); } catch(e){ return ''; } }
  function isWA(h){ return h==='whatsapp.com'||h==='whatsapp.net'||h.slice(-13)==='.whatsapp.com'||h.slice(-13)==='.whatsapp.net'; }
  document.addEventListener('click', function(e){
    var t = e.target; var a = (t && t.closest) ? t.closest('a[href]') : null;
    if (!a) return;
    var url; try { url = new URL(a.getAttribute('href'), location.href).toString(); } catch(err){ return; }
    var proto; try { proto = new URL(url).protocol; } catch(err){ return; }
    if (proto!=='http:' && proto!=='https:') { e.preventDefault(); send('ruwa-open-external', url); return; }
    if (!isWA(hostOf(url))) { e.preventDefault(); if (e.stopPropagation) e.stopPropagation(); send('ruwa-open-external', url); }
  }, true);
  try {
    var origOpen = window.open;
    window.open = function(url){ if (url) send('ruwa-open-external', String(url)); return null; };
  } catch(e){}
})();`;

let win = null;
let lastUnread = 0;
let lastNotifyAt = 0;

function setUnread(n) {
  n = Number(n) || 0;
  if (n === lastUnread) return;
  const prev = lastUnread;
  lastUnread = n;
  testLog('unread:' + n);
  if (win && !win.isDestroyed()) {
    try { win.setTitle(n > 0 ? '(' + n + ') Ruwa' : 'Ruwa'); } catch (e) {}
  }
  if (n > prev) {
    const now = Date.now();
    if (now - lastNotifyAt > 30000) {
      lastNotifyAt = now;
      try {
        new Notification({ title: 'Ruwa', body: n === 1 ? '1 new message' : n + ' new messages' }).show();
      } catch (e) {}
    }
  }
}

function openExternal(url) {
  const u = String(url || '');
  if (!u || isWhatsAppUrl(u)) return;
  testLog('open-external:' + u);
  if (!TEST_LOG) {
    try { shell.openExternal(u); } catch (e) {}
  }
}

function inject() {
  if (win && !win.isDestroyed()) {
    try { win.webContents.executeJavaScript(INJECT); } catch (e) {}
    if (process.env.RUWA_EXTRA_JS) {
      try {
        win.webContents.executeJavaScript(
          require('fs').readFileSync(process.env.RUWA_EXTRA_JS, 'utf8')
        );
      } catch (e) {}
    }
  }
}

function createWindow() {
  win = new BrowserWindow({
    width: 1200,
    height: 850,
    title: 'Ruwa',
    backgroundColor: '#0b141a',
    webPreferences: { contextIsolation: true, nodeIntegration: false },
  });
  win.loadURL(firstArgUrl());
  setTimeout(inject, 6000);
  setInterval(inject, 15000);
  win.on('closed', () => { win = null; });
}

ipcMain.on('ruwa-unread', (event, data) => {
  const n = Array.isArray(data) ? data[0] : data;
  setUnread(n);
});

ipcMain.on('ruwa-open-external', (event, data) => {
  const u = Array.isArray(data) ? data[0] : data;
  openExternal(u);
});

ipcMain.on('ruwa-report', (event, data) => {
  const r = Array.isArray(data) ? data[0] : data;
  testLog('report:' + r);
});

const gotLock = app.requestSingleInstanceLock();
if (!gotLock) {
  app.quit();
} else {
  app.on('second-instance', () => {
    if (win) {
      try {
        if (win.isMinimized()) win.restore();
        win.show();
        win.focus();
      } catch (e) {}
    }
  });
  app.whenReady().then(createWindow);
  app.on('window-all-closed', () => app.quit());
}
