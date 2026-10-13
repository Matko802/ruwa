'use strict';

const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('ruwaBar', {
  reload: () => ipcRenderer.send('bar:reload'),
  toggleSettings: () => ipcRenderer.send('bar:toggle-settings'),
  minimize: () => ipcRenderer.send('bar:min'),
  maximizeToggle: () => ipcRenderer.send('bar:maxtoggle'),
  close: () => ipcRenderer.send('bar:close'),
  ready: () => ipcRenderer.send('bar:ready'),
  onTitle: (cb) => ipcRenderer.on('bar:title', (_e, title) => cb(title)),
  onMaximized: (cb) => ipcRenderer.on('bar:max', (_e, max) => cb(max)),
});
