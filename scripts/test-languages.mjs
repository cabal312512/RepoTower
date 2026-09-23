import './test-env.mjs';
import { _electron as electron } from 'playwright-core';
import { createRequire } from 'node:module';
import { readFile, readdir, mkdir, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import assert from 'node:assert/strict';
import path from 'node:path';

const require = createRequire(import.meta.url);
const root = path.resolve(import.meta.dirname, '..');
const metadata = JSON.parse(await readFile(path.join(root, 'package.json'), 'utf8'));
const packaged = process.argv.includes('--packaged');
const release = path.join(
  root,
  'release',
  `RepoTower-${metadata.version}-${process.platform}-${process.arch}`,
);
const executable =
  process.platform === 'win32'
    ? 'RepoTower.exe'
    : process.platform === 'darwin'
      ? 'RepoTower.app/Contents/MacOS/RepoTower'
      : 'RepoTower';
const resources =
  process.platform === 'darwin'
    ? path.join(release, 'RepoTower.app/Contents/Resources')
    : path.join(release, 'resources');
const samples = packaged
  ? path.join(resources, 'fixtures/language-samples')
  : path.join(root, 'fixtures/language-samples');
const resultPath = path.join(root, 'test-results');
await mkdir(resultPath, { recursive: true });
async function hashSources() {
  const hash = createHash('sha256');
  for (const file of (await readdir(samples, { recursive: true, withFileTypes: true }))
    .filter((item) => item.isFile())
    .map((item) => path.join(item.parentPath, item.name))
    .sort()) {
    hash.update(path.relative(samples, file));
    hash.update(await readFile(file));
  }
  return hash.digest('hex');
}
const before = await hashSources();
const cases = [
  { folder: 'java', label: 'Java', config: 'src/main/java/toy/Config.java', edges: 2 },
  { folder: 'python', label: 'Python', config: 'config.py', edges: 2 },
  { folder: 'c', label: 'C/C++', config: 'config.h', edges: 2 },
  { folder: 'cpp', label: 'C/C++', config: 'config.hpp', edges: 2 },
  { folder: 'go', label: 'Go', config: 'config/config.go', edges: 2 },
  { folder: 'rust', label: 'Rust', config: 'src/config.rs', edges: 3 },
  { folder: 'csharp', label: 'C#', config: 'Config.cs', edges: 2 },
];
const app = await electron.launch({
  executablePath: packaged ? path.join(release, executable) : require('electron'),
  args: packaged ? [] : [root],
  cwd: root,
  env: { ...process.env, TEMP: path.join(root, '.tmp'), TMP: path.join(root, '.tmp') },
});
const checks = [],
  errors = [],
  externalRequests = [];
try {
  const page = await app.firstWindow();
  page.setDefaultTimeout(15000);
  page.on('pageerror', (error) => errors.push(error.message));
  page.on('request', (request) => {
    if (/^https?:/.test(request.url())) externalRequests.push(request.url());
  });
  await page.getByTestId('welcome').waitFor();
  await page.getByTestId('settings-toggle').click();
  await page.getByTestId('theme-dark').click();
  await page.getByTestId('language-select').selectOption('zh');
  await page.getByTestId('settings-toggle').click();
  assert.match(
    await page.locator('.welcome > p').innerText(),
    /Java.*Python.*C\/C\+\+.*Go.*Rust.*C#/,
  );
  for (const example of cases) {
    const repository = path.join(samples, example.folder);
    await app.evaluate(({ dialog }, repository) => {
      dialog.showOpenDialog = async () => ({ canceled: false, filePaths: [repository] });
    }, repository);
    await page.getByTestId('title-open').click();
    await page.waitForFunction(
      (label) => document.querySelector('.statusline > span:last-child')?.textContent === label,
      example.label,
    );
    await page.waitForFunction(() => !document.querySelector('.loading-overlay'));
    // C and C++ share a status label, so also verify the newly loaded repository.
    await page.waitForFunction(
      (name) => document.querySelector('.repo-button strong')?.textContent === name,
      example.folder,
    );
    assert.equal(await page.getByTestId('graph-node').count(), 3);
    assert.equal(await page.getByTestId('graph-edge').count(), example.edges);
    const analysis = await page.evaluate(
      (repository) => window.repoTower.analyzePath(repository),
      repository,
    );
    assert.equal(analysis.totalModules, 3);
    assert.equal(analysis.totalEdges, example.edges);
    assert.equal(analysis.unresolvedCount, 0);
    const expected = new Set(analysis.edges.map((edge) => `${edge.target}\0${edge.source}`));
    for (const edge of await page
      .getByTestId('graph-edge')
      .evaluateAll((elements) =>
        elements.map((element) => [element.dataset.from, element.dataset.to]),
      ))
      assert.ok(expected.has(edge.join('\0')));
    await page.getByTestId('module-search').fill(example.config);
    await page.getByTestId('module-list-item').first().click();
    await page.waitForFunction(
      () => document.querySelector('[data-testid="pull-button"]')?.disabled === false,
    );
    assert.equal((await page.getByTestId('potential-count').innerText()).trim(), '2');
    await page.getByTestId('pull-button').click();
    await page.getByTestId('graph-skip').click();
    await page.getByTestId('impact-report').waitFor();
    assert.equal(
      await page.locator('[data-testid="graph-node"][data-state="affected"]').count(),
      2,
    );
    assert.equal(await page.locator('[data-testid="graph-node"][data-state="removed"]').count(), 1);
    assert.equal(await page.getByTestId('graph-untouched-count').innerText(), '0');
    await page.mouse.move(8, 8);
    await page.screenshot({
      path: path.join(resultPath, `language-${example.folder}.png`),
      animations: 'disabled',
    });
    if (example.folder === 'java')
      await page.screenshot({
        path: path.join(root, 'docs/images/java-impact.png'),
        animations: 'disabled',
      });
    await page.getByTestId('undo-report').click();
    await page.getByTestId('reset-toolbar').waitFor();
    assert.equal(
      await page.locator('[data-testid="graph-node"][data-state="affected"]').count(),
      0,
    );
    checks.push(
      `${example.label} (${example.folder}): 3 files, ${example.edges} real edges, config affects 2 files, undo restores state`,
    );
  }
  assert.equal(await hashSources(), before);
  assert.deepEqual(errors, []);
  assert.deepEqual(externalRequests, []);
  const result = {
    status: 'passed',
    version: metadata.version,
    packaged,
    checks,
    sourceHashUnchanged: true,
    errors,
    externalRequests,
  };
  await writeFile(
    path.join(resultPath, packaged ? 'languages-packaged.json' : 'languages.json'),
    JSON.stringify(result, null, 2),
  );
  console.log(JSON.stringify(result, null, 2));
} catch (error) {
  const page = await app.firstWindow().catch(() => null);
  if (page) {
    await page.screenshot({ path: path.join(resultPath, 'language-failure.png') });
    console.error((await page.locator('body').innerText()).slice(0, 3000));
  }
  throw error;
} finally {
  await app.close();
}
