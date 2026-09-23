'use strict';

const { app, BrowserWindow, dialog, ipcMain, Menu, session } = require('electron');
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const sourceRoot = path.resolve(__dirname, '..');
const applicationDirectory = app.isPackaged
  ? (process.platform === 'darwin' ? path.resolve(path.dirname(process.execPath), '../../..') : path.dirname(process.execPath))
  : sourceRoot;
// The optional single-file launcher keeps its unpacked app and data beside itself.
const portableRoot = app.isPackaged && process.env.REPOTOWER_PORTABLE_ROOT && path.isAbsolute(process.env.REPOTOWER_PORTABLE_ROOT)
  ? process.env.REPOTOWER_PORTABLE_ROOT : applicationDirectory;
const runtimeRoot = path.join(portableRoot, 'runtime-data');
// Set every Electron-owned writable directory before Chromium starts.
for (const [name, child] of Object.entries({ appData: 'app', userData: 'user', sessionData: 'session', temp: 'tmp', crashDumps: 'crashes' })) {
  const destination = path.join(runtimeRoot, child);
  fs.mkdirSync(destination, { recursive: true });
  app.setPath(name, destination);
}
fs.mkdirSync(path.join(runtimeRoot, 'logs'), { recursive: true });
app.setAppLogsPath(path.join(runtimeRoot, 'logs'));
process.env.TEMP = app.getPath('temp');
process.env.TMP = app.getPath('temp');
process.env.TMPDIR = app.getPath('temp');
if (process.platform === 'linux') {
  // Linux Unix-domain sockets have a 108-byte pathname limit. A private short
  // alias keeps Chromium's socket path short while its bytes stay beside the app.
  const aliasRoot = fs.mkdtempSync('/tmp/repotower-');
  const alias = path.join(aliasRoot, 't');
  fs.symlinkSync(app.getPath('temp'), alias, 'dir');
  process.env.TMPDIR = alias;
  const removeAlias = () => {
    try { fs.unlinkSync(alias); } catch { /* Already removed. */ }
    try { fs.rmdirSync(aliasRoot); } catch { /* Never remove unexpected files. */ }
  };
  app.once('will-quit', () => app.releaseSingleInstanceLock());
  app.once('quit', removeAlias);
  process.once('exit', removeAlias);
}
process.env.XDG_CACHE_HOME = path.join(runtimeRoot, 'cache');
process.env.XDG_CONFIG_HOME = path.join(runtimeRoot, 'config');
app.commandLine.appendSwitch('disable-background-networking');
app.commandLine.appendSwitch('disable-component-update');
app.commandLine.appendSwitch('disable-domain-reliability');
app.commandLine.appendSwitch('no-proxy-server');
app.setName('RepoTower');

let window = null;
let sidecar = null;
let nextId = 1;
let queue = Promise.resolve();
const pending = new Map();
const binaryName = process.platform === 'win32' ? 'repotower-core.exe' : 'repotower-core';
const sidecarPath = app.isPackaged
  ? path.join(process.resourcesPath, 'sidecar', binaryName)
  : path.join(sourceRoot, 'target', 'release', binaryName);
const demoPath = app.isPackaged
  ? path.join(process.resourcesPath, 'fixtures', 'demo-project')
  : path.join(sourceRoot, 'fixtures', 'demo-project');

function rejectPending(message, child) {
  for (const [id, job] of pending) {
    if (child && job.child !== child) continue;
    clearTimeout(job.timer);
    job.reject(new Error(message));
    pending.delete(id);
  }
}

function startSidecar() {
  if (sidecar && !sidecar.killed) return sidecar;
  if (!fs.existsSync(sidecarPath)) {
    throw new Error('分析引擎未找到。开发环境请先运行 scripts/build.ps1，或使用完整的便携发布目录。');
  }
  const child = spawn(sidecarPath, [], {
    cwd: portableRoot,
    env: { ...process.env },
    stdio: ['pipe', 'pipe', 'pipe'],
    windowsHide: true,
    shell: false,
  });
  sidecar = child;
  let buffer = '';
  let stderr = '';
  child.stdout.setEncoding('utf8');
  child.stderr.setEncoding('utf8');
  child.stderr.on('data', chunk => { stderr = (stderr + chunk).slice(-4000); });
  child.stdout.on('data', chunk => {
    buffer += chunk;
    if (buffer.length > 128 * 1024 * 1024) {
      rejectPending('分析结果超过安全大小限制。请选择较小的项目目录。', child);
      child.kill();
      return;
    }
    let newline;
    while ((newline = buffer.indexOf('\n')) !== -1) {
      const line = buffer.slice(0, newline).trim();
      buffer = buffer.slice(newline + 1);
      if (!line) continue;
      let message;
      try { message = JSON.parse(line); }
      catch {
        rejectPending('分析引擎返回了无法读取的数据。请重新打开项目。', child);
        child.kill();
        return;
      }
      const job = pending.get(message.id);
      if (!job) continue;
      if (message.type === 'progress') {
        if (window && !window.isDestroyed()) window.webContents.send('repotower:progress', message.progress);
      } else if (message.type === 'result' || message.type === 'error') {
        clearTimeout(job.timer);
        pending.delete(message.id);
        if (message.type === 'result') job.resolve(message.result);
        else job.reject(new Error(typeof message.error === 'string' ? message.error : '分析失败，请重新选择项目。'));
      }
    }
  });
  child.on('error', error => {
    if (sidecar === child) sidecar = null;
    rejectPending(`无法启动分析引擎：${error.message}`, child);
  });
  child.on('exit', code => {
    if (sidecar === child) sidecar = null;
    rejectPending(`分析引擎已退出（${code ?? '已中止'}）。请重新打开项目。${stderr ? '\n' + stderr : ''}`, child);
  });
  child.stdin.on('error', () => rejectPending('分析引擎连接已关闭，请重新打开项目。', child));
  return child;
}

function request(command) {
  // Serialize scans and impact requests: the Rust process owns one current graph.
  const run = () => new Promise((resolve, reject) => {
    let child;
    try { child = startSidecar(); } catch (error) { reject(error); return; }
    const id = nextId++;
    const timer = setTimeout(() => {
      pending.delete(id);
      reject(new Error('分析超时，请选择较小的项目目录后重试。'));
      child.kill();
    }, 180_000);
    pending.set(id, { resolve, reject, timer, child });
    child.stdin.write(JSON.stringify({ id, ...command }) + '\n', error => {
      if (!error) return;
      clearTimeout(timer);
      pending.delete(id);
      reject(new Error('无法发送分析请求，请重新打开项目。'));
    });
  });
  const result = queue.then(run, run);
  queue = result.catch(() => {});
  return result;
}

function assertTrusted(event) {
  if (!window || event.sender !== window.webContents || event.senderFrame !== window.webContents.mainFrame) {
    throw new Error('IPC sender is not the application main frame.');
  }
}

async function analyzePath(value) {
  if (typeof value !== 'string' || value.length > 32768 || !path.isAbsolute(value) || value.includes('\0')) {
    throw new Error('请选择有效的项目文件夹。');
  }
  let directory;
  try {
    directory = await fs.promises.realpath(value);
    if (!(await fs.promises.stat(directory)).isDirectory()) throw new Error('not a directory');
  } catch { throw new Error('无法读取此文件夹，请检查路径与读取权限。'); }
  return request({ command: 'analyze', path: directory });
}

function registerIpc() {
  ipcMain.handle('repotower:window-minimize', event => {
    assertTrusted(event);
    window.minimize();
  });
  ipcMain.handle('repotower:window-toggle-maximize', event => {
    assertTrusted(event);
    if (window.isMaximized()) window.unmaximize();
    else window.maximize();
    return { maximized: window.isMaximized() };
  });
  ipcMain.handle('repotower:window-close', event => {
    assertTrusted(event);
    window.close();
  });
  ipcMain.handle('repotower:window-state', event => {
    assertTrusted(event);
    return { maximized: window.isMaximized() };
  });
  ipcMain.handle('repotower:open', async event => {
    assertTrusted(event);
    const result = await dialog.showOpenDialog(window, {
      title: 'RepoTower',
      properties: ['openDirectory', 'dontAddToRecent'],
    });
    return result.canceled || !result.filePaths[0] ? null : analyzePath(result.filePaths[0]);
  });
  ipcMain.handle('repotower:analyze', (event, value) => {
    assertTrusted(event);
    return analyzePath(value);
  });
  ipcMain.handle('repotower:demo', event => {
    assertTrusted(event);
    return analyzePath(demoPath);
  });
  ipcMain.handle('repotower:impact', (event, moduleId, excluded) => {
    assertTrusted(event);
    const isId = value => typeof value === 'string' && value.length > 0 && value.length <= 32768 && !value.includes('\0');
    if (!isId(moduleId) || !Array.isArray(excluded) || excluded.length > 100000 || !excluded.every(isId)) {
      throw new Error('模块参数无效，请重新打开项目。');
    }
    return request({ command: 'impact', moduleId, excluded });
  });
}

function configureOfflineSession() {
  const browserSession = session.defaultSession;
  browserSession.setPermissionRequestHandler((_webContents, _permission, callback) => callback(false));
  browserSession.setPermissionCheckHandler(() => false);
  browserSession.webRequest.onBeforeRequest({ urls: ['<all_urls>'] }, (details, callback) => {
    callback({ cancel: /^(https?|wss?|ftp):/i.test(details.url) });
  });
  browserSession.on('will-download', event => event.preventDefault());
}

function findDirectoryArgument(args) {
  // Electron/Playwright can put debugging flags before the development app path.
  // Locate and skip that app entry instead of assuming it is argv[1]. A later
  // occurrence of the same directory remains a valid explicit user argument.
  let passedAppEntry = app.isPackaged;
  for (const argument of args) {
    if (!argument || argument.startsWith('-')) continue;
    const absolute = path.resolve(argument);
    if (!passedAppEntry) {
      if (path.relative(sourceRoot, absolute) === '' || path.relative(__filename, absolute) === '') passedAppEntry = true;
      continue;
    }
    try { if (fs.statSync(absolute).isDirectory()) return absolute; } catch { /* Ignore non-path flags. */ }
  }
  return undefined;
}

function sendOpenPath(directory) {
  if (!directory || !window || window.isDestroyed()) return;
  window.webContents.send('repotower:open-path', directory);
  if (window.isMinimized()) window.restore();
  window.focus();
}

function createWindow() {
  window = new BrowserWindow({
    title: 'RepoTower',
    icon: path.join(__dirname, 'assets', 'repotower.png'),
    width: 980,
    height: 680,
    minWidth: 760,
    minHeight: 520,
    frame: false,
    backgroundColor: '#f1f3ef',
    show: false,
    autoHideMenuBar: true,
    webPreferences: {
      preload: path.join(__dirname, 'preload.cjs'),
      nodeIntegration: false,
      nodeIntegrationInWorker: false,
      nodeIntegrationInSubFrames: false,
      contextIsolation: true,
      sandbox: true,
      webSecurity: true,
      allowRunningInsecureContent: false,
      webviewTag: false,
      spellcheck: false,
    },
  });
  window.webContents.setWindowOpenHandler(() => ({ action: 'deny' }));
  window.webContents.on('will-navigate', event => event.preventDefault());
  window.webContents.on('will-attach-webview', event => event.preventDefault());
  const sendWindowState = () => {
    if (window && !window.isDestroyed()) {
      window.webContents.send('repotower:window-state', { maximized: window.isMaximized() });
    }
  };
  window.on('maximize', sendWindowState);
  window.on('unmaximize', sendWindowState);
  window.once('ready-to-show', () => window?.show());
  window.webContents.once('did-finish-load', () => {
    const argument = findDirectoryArgument(process.argv.slice(1));
    if (argument) sendOpenPath(argument);
  });
  window.on('closed', () => { window = null; });
  window.loadFile(path.join(sourceRoot, 'dist', 'index.html')).catch(error => {
    dialog.showErrorBox('RepoTower 无法启动', `界面文件无法读取。请使用完整的发布目录或先运行构建。\n${error.message}`);
    app.quit();
  });
}

if (!app.requestSingleInstanceLock()) {
  app.quit();
} else {
  app.on('second-instance', (_event, args) => sendOpenPath(findDirectoryArgument(args.slice(1))));
  app.whenReady().then(() => {
    Menu.setApplicationMenu(null);
    configureOfflineSession();
    registerIpc();
    createWindow();
  }).catch(error => {
    dialog.showErrorBox('RepoTower 启动失败', error.message);
    app.quit();
  });
  app.on('activate', () => { if (!window) createWindow(); });
  app.on('window-all-closed', () => app.quit());
  app.on('before-quit', () => {
    rejectPending('应用正在关闭。');
    if (sidecar) sidecar.kill();
  });
}
