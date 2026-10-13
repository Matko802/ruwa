'use strict';

const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('ruwaPanel', {
  getConfig: () => ipcRenderer.invoke('panel:get-config'),
  setConfig: (patch) => ipcRenderer.invoke('panel:set-config', patch),
  getCss: () => ipcRenderer.invoke('panel:get-css'),
  setCss: (css) => ipcRenderer.invoke('panel:set-css', css),
  listMods: () => ipcRenderer.invoke('panel:list-mods'),
  setMods: (ids) => ipcRenderer.invoke('panel:set-mods', ids),
  getAutostart: () => ipcRenderer.invoke('panel:get-autostart'),
  setAutostart: (on) => ipcRenderer.invoke('panel:set-autostart', on),
  logout: () => ipcRenderer.invoke('panel:logout'),
  close: () => ipcRenderer.send('panel:close'),
});
