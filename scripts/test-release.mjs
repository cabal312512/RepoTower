import './test-env.mjs';
import { _electron as electron } from 'playwright-core';
import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import { root, packageRoot } from './release-utils.mjs';

const results = path.join(
  root,
  'test-results',
  ...(process.platform === 'linux' ? ['deep-portable-location-'.repeat(6)] : []),
);
await fs.mkdir(results, { recursive: true });
const data = await fs.mkdtemp(path.join(results, 'release-profile-'));
const executable =
  process.platform === 'win32'
    ? 'RepoTower.exe'
    : process.platform === 'darwin'
      ? 'RepoTower.app/Contents/MacOS/RepoTower'
      : 'RepoTower';
const app = await electron.launch({
  executablePath: path.join(packageRoot, executable),
  env: { ...process.env, REPOTOWER_PORTABLE_ROOT: data, TEMP: data, TMP: data },
  timeout: 60000,
});
const errors = [],
  requests = [];
let linuxAlias;
try {
  const page = await app.firstWindow();
  page.on('pageerror', (error) => errors.push(error.message));
  page.on('request', (request) => {
    if (/^https?:/.test(request.url())) requests.push(request.url());
  });
  await page.getByTestId('welcome').waitFor();
  assert.equal(await page.locator('.app').getAttribute('data-theme'), 'light');
  await page.getByTestId('settings-toggle').click();
  assert.equal(await page.getByTestId('language-select').inputValue(), 'en');
  await page.getByTestId('settings-toggle').click();
  const directories = await app.evaluate(({ app }) =>
    ['appData', 'userData', 'sessionData', 'temp', 'crashDumps', 'logs'].map((key) =>
      app.getPath(key),
    ),
  );
  for (const directory of directories) assert.ok(directory.startsWith(data + path.sep), directory);
  if (process.platform === 'linux') {
    linuxAlias = await app.evaluate(() => process.env.TMPDIR);
    assert.ok(Buffer.byteLength(linuxAlias) < 60);
    assert.equal(await fs.realpath(linuxAlias), path.join(data, 'runtime-data', 'tmp'));
  }
  await page.getByTestId('open-demo').click();
  await page.getByTestId('workspace').waitFor({ timeout: 60000 });
  assert.equal(await page.getByTestId('graph-node').count(), 23);
  assert.equal(await page.getByTestId('graph-edge').count(), 24);
  if (process.argv.includes('--screenshots')) {
    await page.mouse.move(8, 8);
    await page.screenshot({
      path: path.join(root, 'docs/images/release-light.png'),
      animations: 'disabled',
    });
  }
  await page.getByTestId('module-search').fill('src/core/config.ts');
  await page.getByTestId('module-list-item').first().click();
  await page.getByTestId('pull-button').click();
  await page.getByTestId('graph-skip').click();
  await page.getByTestId('impact-report').waitFor();
  assert.equal(await page.getByTestId('graph-affected-count').innerText(), '13');
  assert.equal(await page.getByTestId('graph-untouched-count').innerText(), '9');
  if (process.argv.includes('--screenshots')) {
    await page.mouse.move(8, 8);
    await page.screenshot({
      path: path.join(root, 'docs/images/release-impact.png'),
      animations: 'disabled',
    });
  }
  assert.deepEqual(errors, []);
  assert.deepEqual(requests, []);
  console.log(
    'Release smoke passed: English/light defaults, adjacent data, 23 files / 24 edges, 13 affected / 9 untouched, no renderer errors or HTTP requests.',
  );
} finally {
  await app.close();
}
if (linuxAlias)
  assert.equal(
    await fs.stat(path.dirname(linuxAlias)).then(
      () => true,
      () => false,
    ),
    false,
    'The temporary socket alias must be removed after exit',
  );
