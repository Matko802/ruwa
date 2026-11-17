'use strict';

const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('ruwaPanel', {
  getConfig: () => ipcRenderer.invoke('panel:get-config'),
  setConfig: (patch) => ipcRenderer.invoke('panel:set-config', patch),
  getCss: () => ipcRenderer.invoke('panel:get-css'),
  setCss: (css) => ipcRenderer.invoke('panel:set-css', css),
  listplugins: () => ipcRenderer.invoke('panel:list-plugins'),
  setplugins: (ids) => ipcRenderer.invoke('panel:set-plugins', ids),
  getAutostart: () => ipcRenderer.invoke('panel:get-autostart'),
  setAutostart: (on) => ipcRenderer.invoke('panel:set-autostart', on),
  logout: () => ipcRenderer.invoke('panel:logout'),
  reloadMain: () => ipcRenderer.invoke('panel:reload-main'),
  getVersion: () => ipcRenderer.invoke('panel:get-version'),
  getTheme: () => ipcRenderer.invoke('panel:get-theme'),
  onTheme: (cb) => ipcRenderer.on('panel:theme', (_e, t) => cb(t)),
});
