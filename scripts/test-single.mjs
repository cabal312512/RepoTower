import { _electron as electron } from 'playwright-core';
import fs from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import assert from 'node:assert/strict';
import { root, artifacts, artifactBase, digest } from './release-utils.mjs';

if (process.platform !== 'win32') throw new Error('Single EXE tests require Windows.');
await fs.mkdir(path.join(root, 'test-results'), { recursive: true });
const folder = await fs.mkdtemp(path.join(root, 'test-results', 'single-'));
const executable = path.join(folder, `${artifactBase}.exe`);
await fs.copyFile(path.join(artifacts, path.basename(executable)), executable);
const env = { ...process.env, TEMP: path.join(root, '.tmp'), TMP: path.join(root, '.tmp') };
function extract() {
  const result = spawnSync(executable, ['--repotower-extract-only'], {
    env,
    windowsHide: true,
    timeout: 120000,
  });
  assert.equal(result.error, undefined);
  assert.equal(result.status, 0, result.stderr?.toString());
}
extract();
const data = path.join(folder, 'RepoTower-data');
const cached = (await fs.readdir(data)).find((name) => name.startsWith('app-'));
assert.ok(cached);
const asar = path.join(data, cached, 'resources/app.asar');
const original = await digest(asar);
const firstStat = await fs.stat(asar);
extract();
assert.equal(
  (await fs.stat(asar)).mtimeMs,
  firstStat.mtimeMs,
  'Warm launch must reuse the verified cache',
);
await fs.mkdir(path.join(data, 'runtime-data'), { recursive: true });
const marker = path.join(data, 'runtime-data', 'keep.txt');
await fs.writeFile(marker, 'settings survive cache repair');
await fs.writeFile(asar, 'corrupted cache');
extract();
assert.equal(await digest(asar), original);
assert.equal(await fs.readFile(marker, 'utf8'), 'settings survive cache repair');

// Launch the actual wrapper so argument forwarding and child lifetime are tested.
const app = await electron.launch({ executablePath: executable, env, timeout: 90000 });
try {
  const page = await app.firstWindow();
  await page.getByTestId('welcome').waitFor();
  assert.equal(await page.locator('.app').getAttribute('data-theme'), 'light');
  const userData = await app.evaluate(({ app }) => app.getPath('userData'));
  assert.ok(userData.startsWith(path.join(data, 'runtime-data') + path.sep));
  await page.getByTestId('settings-toggle').click();
  assert.equal(await page.getByTestId('language-select').inputValue(), 'en');
  await page.getByTestId('language-select').selectOption('ja');
  await page.getByTestId('settings-toggle').click();
  await page.getByTestId('open-demo').click();
  await page.getByTestId('workspace').waitFor({ timeout: 60000 });
  assert.equal(await page.getByTestId('graph-node').count(), 23);
  assert.equal(await page.getByTestId('graph-edge').count(), 24);
} finally {
  await app.close();
}
const reopened = await electron.launch({ executablePath: executable, env, timeout: 90000 });
try {
  const page = await reopened.firstWindow();
  await page.getByTestId('welcome').waitFor();
  await page.getByTestId('settings-toggle').click();
  assert.equal(await page.getByTestId('language-select').inputValue(), 'ja');
} finally {
  await reopened.close();
}
console.log(
  'Single EXE passed: extraction, cache reuse, corruption repair, preserved settings, real GUI launch, Rust demo analysis and preference persistence.',
);
