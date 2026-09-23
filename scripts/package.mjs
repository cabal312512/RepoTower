import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const temp = path.join(root, '.tmp');
process.env.TEMP = temp;
process.env.TMP = temp;
process.env.TMPDIR = temp;
process.env.ELECTRON_CACHE = path.join(root, '.cache', 'electron');
process.env.electron_config_cache = process.env.ELECTRON_CACHE;
process.env.ELECTRON_BUILDER_CACHE = path.join(root, '.cache', 'electron-builder');
await fs.mkdir(temp, { recursive: true });

function assertInsideProject(target) {
  const relative = path.relative(root, path.resolve(target));
  if (!relative || relative === '..' || relative.startsWith(`..${path.sep}`) || path.isAbsolute(relative)) {
    throw new Error(`Refusing to replace a directory outside the project: ${target}`);
  }
}

async function exists(file) {
  try { await fs.access(file); return true; } catch { return false; }
}

async function findArchive(directory, name, depth = 3) {
  if (!(await exists(directory))) return null;
  for (const item of await fs.readdir(directory, { withFileTypes: true })) {
    if (item.isFile() && item.name === name) return directory;
    if (item.isDirectory() && depth > 0) {
      const found = await findArchive(path.join(directory, item.name), name, depth - 1);
      if (found) return found;
    }
  }
  return null;
}

const metadata = JSON.parse(await fs.readFile(path.join(root, 'package.json'), 'utf8'));
await import('./collect-licenses.mjs');
const documentationFiles = ['LICENSE', 'README.md', 'THIRD_PARTY_NOTICES.md', 'THIRD_PARTY_LICENSES.txt'];
const electronVersion = JSON.parse(await fs.readFile(path.join(root, 'node_modules', 'electron', 'package.json'), 'utf8')).version;
const platform = process.platform;
const arch = process.arch;
const binaryName = platform === 'win32' ? 'repotower-core.exe' : 'repotower-core';
const binary = path.join(root, 'target', 'release', binaryName);
if (!(await exists(binary)) || !(await exists(path.join(root, 'dist', 'index.html')))) {
  throw new Error('Build the Rust release binary and frontend before packaging. On Windows run scripts/build.ps1.');
}

const stage = path.join(temp, 'package-app');
const resourceStage = path.join(temp, 'package-resources');
const packageOutput = path.join(temp, 'package-output');
const output = path.join(root, 'release');
for (const target of [stage, resourceStage, packageOutput]) {
  assertInsideProject(target);
  await fs.rm(target, { recursive: true, force: true });
  await fs.mkdir(target, { recursive: true });
}
assertInsideProject(path.join(output, `RepoTower-${metadata.version}-${platform}-${arch}`));
await fs.cp(path.join(root, 'desktop'), path.join(stage, 'desktop'), { recursive: true });
await fs.cp(path.join(root, 'dist'), path.join(stage, 'dist'), { recursive: true });
if (await exists(path.join(root, 'docs'))) await fs.cp(path.join(root, 'docs'), path.join(stage, 'docs'), { recursive: true });
await fs.writeFile(path.join(stage, 'package.json'), JSON.stringify({
  name: metadata.name,
  productName: 'RepoTower',
  version: metadata.version,
  description: metadata.description,
  author: metadata.author,
  license: metadata.license,
  main: 'desktop/main.cjs',
}, null, 2) + '\n');
await fs.mkdir(path.join(resourceStage, 'sidecar'), { recursive: true });
await fs.copyFile(binary, path.join(resourceStage, 'sidecar', binaryName));
if (platform !== 'win32') await fs.chmod(path.join(resourceStage, 'sidecar', binaryName), 0o755);
await fs.cp(path.join(root, 'fixtures', 'demo-project'), path.join(resourceStage, 'fixtures', 'demo-project'), { recursive: true });
await fs.cp(path.join(root, 'fixtures', 'language-samples'), path.join(resourceStage, 'fixtures', 'language-samples'), { recursive: true });
for (const name of documentationFiles) {
  if (await exists(path.join(root, name))) await fs.copyFile(path.join(root, name), path.join(stage, name));
}

const archiveName = `electron-v${electronVersion}-${platform}-${arch}.zip`;
const electronZipDir = await findArchive(path.join(root, '.cache', 'downloads'), archiveName)
  ?? await findArchive(path.join(root, '.cache', 'electron'), archiveName);
const { packager } = await import('@electron/packager');
const packages = await packager({
  dir: stage,
  name: 'RepoTower',
  executableName: 'RepoTower',
  icon: path.join(root, 'desktop', 'assets', `repotower.${platform === 'win32' ? 'ico' : platform === 'darwin' ? 'icns' : 'png'}`),
  appVersion: metadata.version,
  appCopyright: 'Copyright (c) 2026 Cabal and RepoTower contributors',
  electronVersion,
  platform,
  arch,
  asar: true,
  overwrite: true,
  prune: false,
  // Build the complete candidate first, keeping the current release usable.
  out: packageOutput,
  tmpdir: path.join(temp, 'packager'),
  extraResource: [path.join(resourceStage, 'sidecar'), path.join(resourceStage, 'fixtures')],
  ...(electronZipDir ? { electronZipDir } : { download: { cacheRoot: process.env.ELECTRON_CACHE } }),
  win32metadata: { CompanyName: 'Cabal', FileDescription: 'RepoTower offline dependency explorer', ProductName: 'RepoTower' },
});

for (const directory of packages) {
  assertInsideProject(directory);
  const electronLicense = path.join(directory, 'LICENSE');
  if (await exists(electronLicense)) await fs.rename(electronLicense, path.join(directory, 'LICENSE.electron.txt'));
  // Keep concise documentation accessible without extracting app.asar.
  for (const name of documentationFiles) {
    if (await exists(path.join(root, name))) await fs.copyFile(path.join(root, name), path.join(directory, name));
  }
  if (await exists(path.join(root, 'docs'))) await fs.cp(path.join(root, 'docs'), path.join(directory, 'docs'), { recursive: true });
  await fs.writeFile(path.join(directory, 'PORTABLE.txt'),
    'RepoTower portable edition\r\n\r\nOpen RepoTower.exe (Windows), RepoTower.app (macOS), or RepoTower (Linux).\r\nKeep this complete directory together. No installation or internet connection is needed.\r\nApplication data stays in runtime-data next to the executable/application.\r\nSource repositories are only read; disconnecting a node never deletes a source file.\r\n');
  const destination = path.join(output, `RepoTower-${metadata.version}-${platform}-${arch}`);
  const previous = path.join(temp, `previous-RepoTower-${metadata.version}-${platform}-${arch}-${Date.now()}`);
  assertInsideProject(destination);
  assertInsideProject(previous);
  await fs.mkdir(output, { recursive: true });
  const hasPrevious = await exists(destination);
  // Refuse to relocate a running copy. Different released versions live in
  // different directories, so the user can keep the previous version open.
  if (hasPrevious && platform === 'win32') {
    const running = spawnSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command',
      "if (@(Get-Process -Name RepoTower -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $env:REPOTOWER_PACKAGE_EXECUTABLE }).Count -gt 0) { exit 13 }; exit 0"], {
      env: { ...process.env, REPOTOWER_PACKAGE_EXECUTABLE: path.join(destination, 'RepoTower.exe') },
      windowsHide: true,
      shell: false,
      timeout: 15000,
    });
    if (running.error || running.status !== 0) {
      throw new Error(`Cannot replace ${destination}. Close this version of RepoTower before rebuilding. The existing release was kept unchanged.`, { cause: running.error });
    }
  }
  if (hasPrevious) await fs.rename(destination, previous);
  try {
    await fs.rename(directory, destination);
    const previousData = path.join(previous, 'runtime-data');
    if (hasPrevious && await exists(previousData)) {
      await fs.rename(previousData, path.join(destination, 'runtime-data'));
    }
  } catch (error) {
    if (await exists(destination)) await fs.rename(destination, directory);
    if (hasPrevious) await fs.rename(previous, destination);
    throw error;
  }
  if (hasPrevious) {
    await fs.rm(previous, { recursive: true, force: true }).catch(error => {
      console.warn(`Updated release is ready; the old copy remains at ${previous}: ${error.message}`);
    });
  }
  console.log(`Portable application ready: ${destination}`);
}
