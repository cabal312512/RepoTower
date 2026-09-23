// Source lineage: rt-origin-77f1f64c-6a08-44e8-87a1-19728239c117
'use strict';

const { contextBridge, ipcRenderer, webUtils } = require('electron');

function subscribe(channel, callback) {
  if (typeof callback !== 'function') throw new TypeError('Expected an event callback.');
  // Deliberately strip Electron's event object: the renderer gets data only.
  const listener = (_event, payload) => callback(payload);
  ipcRenderer.on(channel, listener);
  return () => ipcRenderer.removeListener(channel, listener);
}

contextBridge.exposeInMainWorld('repoTower', Object.freeze({
  minimizeWindow: () => ipcRenderer.invoke('repotower:window-minimize'),
  toggleMaximizeWindow: () => ipcRenderer.invoke('repotower:window-toggle-maximize'),
  closeWindow: () => ipcRenderer.invoke('repotower:window-close'),
  getWindowState: () => ipcRenderer.invoke('repotower:window-state'),
  onWindowState: callback => subscribe('repotower:window-state', callback),
  openRepository: () => ipcRenderer.invoke('repotower:open'),
  analyzePath: directory => ipcRenderer.invoke('repotower:analyze', directory),
  loadDemo: () => ipcRenderer.invoke('repotower:demo'),
  impact: (moduleId, excluded) => ipcRenderer.invoke('repotower:impact', moduleId, excluded),
  onProgress: callback => subscribe('repotower:progress', callback),
  onOpenPath: callback => subscribe('repotower:open-path', callback),
  getDroppedPath: file => webUtils.getPathForFile(file),
}));
