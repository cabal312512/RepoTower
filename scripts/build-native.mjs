// Native Windows/macOS/Linux build. Install JS dependencies with npm ci first.
// Windows users can use build.ps1 to bootstrap the project-local GNU toolchain.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const temp = path.join(root, '.tmp');
const cargoCache = path.join(root, '.cache', 'cargo');
const electronCache = path.join(root, '.cache', 'electron');
for (const directory of [temp, cargoCache, electronCache]) fs.mkdirSync(directory, { recursive: true });
const env = {
  ...process.env,
  TEMP: temp,
  TMP: temp,
  TMPDIR: temp,
  CARGO_HOME: cargoCache,
  CARGO_TARGET_DIR: path.join(root, 'target'),
  npm_config_cache: path.join(root, '.cache', 'npm'),
  ELECTRON_CACHE: electronCache,
  electron_config_cache: electronCache,
  ELECTRON_BUILDER_CACHE: path.join(root, '.cache', 'electron-builder'),
};
let cargo = process.platform === 'win32' ? 'cargo.exe' : 'cargo';
const portableRust = path.join(root, '.tools', 'rust', 'bin');
if (process.platform === 'win32' && fs.existsSync(path.join(portableRust, 'cargo.exe'))) {
  const compilerBin = path.join(root, '.tools', 'w64devkit', 'bin');
  cargo = path.join(portableRust, 'cargo.exe');
  env.RUSTUP_HOME = path.join(root, '.tools', 'rustup');
  env.CC = path.join(compilerBin, 'gcc.exe');
  env.CXX = path.join(compilerBin, 'g++.exe');
  env.AR = path.join(compilerBin, 'ar.exe');
  env.CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = env.CC;
  env.CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS = '-C link-self-contained=yes';
  // Windows environment keys are case-insensitive; retain a single PATH key.
  const previousPath = Object.entries(env).find(([key]) => key.toLowerCase() === 'path')?.[1] ?? '';
  for (const key of Object.keys(env)) if (key.toLowerCase() === 'path') delete env[key];
  env.PATH = [portableRust, compilerBin, previousPath].join(path.delimiter);
} else if (process.platform === 'win32') {
  // CI's native MSVC build should not require a separately installed VC runtime.
  env.CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS = '-C target-feature=+crt-static';
  env.CARGO_TARGET_AARCH64_PC_WINDOWS_MSVC_RUSTFLAGS = '-C target-feature=+crt-static';
}

function run(executable, args) {
  const result = spawnSync(executable, args, { cwd: root, env, stdio: 'inherit', windowsHide: true, shell: false });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${path.basename(executable)} ${args.join(' ')} failed (${result.status}).`);
}
function nodeScript(relative, args = []) { run(process.execPath, [path.join(root, relative), ...args]); }

if (!fs.existsSync(path.join(root, 'node_modules', 'vite', 'package.json'))) {
  throw new Error('JavaScript dependencies are missing. Run npm ci first (its cache is configured inside the project).');
}
if (!process.argv.includes('--skip-tests')) {
  run(cargo, ['test', '--workspace', '--locked']);
  nodeScript('node_modules/vitest/vitest.mjs', ['run']);
}
run(cargo, ['build', '--workspace', '--release', '--locked']);
nodeScript('node_modules/typescript/bin/tsc', ['--noEmit']);
nodeScript('node_modules/vite/bin/vite.js', ['build']);
if (!process.argv.includes('--skip-package')) {
  // Electron 44's npm package intentionally does not download its runtime in a
  // postinstall script. This is required on every platform before packaging/tests.
  nodeScript('node_modules/electron/install.js');
  nodeScript('scripts/package.mjs');
}
