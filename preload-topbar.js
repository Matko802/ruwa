'use strict';

const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('ruwaBar', {
  reload: () => ipcRenderer.send('bar:reload'),
  toggleSettings: () => ipcRenderer.send('bar:toggle-settings'),
  onTitle: (cb) => ipcRenderer.on('bar:title', (_e, title) => cb(title)),
  onPanel: (cb) => ipcRenderer.on('bar:panel', (_e, open) => cb(open)),
});
