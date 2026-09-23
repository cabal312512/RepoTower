import { _electron as electron } from 'playwright-core';
import { createRequire } from 'node:module';
import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';
import assert from 'node:assert/strict';

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
const demo = packaged
  ? path.join(resources, 'fixtures/demo-project')
  : path.join(root, 'fixtures/demo-project');
const results = path.join(root, 'test-results');
await mkdir(results, { recursive: true });
async function hashSources() {
  const hash = createHash('sha256');
  for (const file of (await readdir(demo, { recursive: true, withFileTypes: true }))
    .filter((item) => item.isFile())
    .map((item) => path.join(item.parentPath, item.name))
    .sort()) {
    hash.update(path.relative(demo, file));
    hash.update(await readFile(file));
  }
  return hash.digest('hex');
}
const beforeHash = await hashSources();
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
  await page.getByTestId('open-demo').click();
  await page.getByTestId('graph-scene').waitFor();
  const graph = page.getByTestId('graph-scene');
  const camera = page.locator('.graph-board > g[transform]');
  const node = (id) => page.locator(`[data-testid="graph-node"][data-id="${id}"]`);
  const config = 'src/core/config.ts',
    auth = 'src/core/auth.ts',
    palette = 'src/ui/palette.ts';
  const settle = () =>
    page.evaluate(
      () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
    );
  async function select(id) {
    await page.getByTestId('module-search').fill(id);
    await page.getByTestId('module-list-item').first().click();
    await settle();
  }
  async function disconnect(id) {
    await select(id);
    await page.waitForFunction(
      () => document.querySelector('[data-testid="pull-button"]')?.disabled === false,
    );
    await page.getByTestId('pull-button').click();
    await page.getByTestId('graph-skip').click();
    await page.getByTestId('impact-report').waitFor();
  }
  const initialCamera = await camera.getAttribute('transform');
  await page.getByRole('button', { name: '放大', exact: true }).click();
  const zoomed = await camera.getAttribute('transform');
  assert.notEqual(zoomed, initialCamera);
  await node(auth).click();
  await settle();
  assert.equal(await camera.getAttribute('transform'), zoomed);
  assert.equal(await graph.getAttribute('data-view'), 'overview');
  await page.mouse.move(10, 120);
  await node(palette).hover();
  await page.waitForTimeout(100);
  assert.equal(await node(auth).getAttribute('aria-pressed'), 'true');
  await page.waitForTimeout(300);
  assert.equal(await node(palette).getAttribute('aria-pressed'), 'true');
  await page.mouse.move(10, 120);
  assert.equal(await node(auth).getAttribute('aria-pressed'), 'true');
  checks.push(
    'Click preserves zoom and All view; hover previews wait 350 ms and return to the selected file',
  );

  await page.getByTestId('graph-fit').click();
  const beforeMoveCamera = await camera.getAttribute('transform');
  const authPosition = await node(auth).getAttribute('transform');
  const palettePosition = await node(palette).getAttribute('transform');
  const authWire = page
    .locator(`[data-testid="graph-edge"][data-from="${auth}"] .graph-wire`)
    .first();
  const wireBefore = await authWire.getAttribute('d');
  async function moveFile(id, dx, dy) {
    const box = await node(id).boundingBox();
    assert.ok(box);
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down({ button: 'right' });
    await page.mouse.move(box.x + box.width / 2 + dx, box.y + box.height / 2 + dy, { steps: 8 });
    await page.mouse.up({ button: 'right' });
    await settle();
  }
  await moveFile(auth, 68, -38);
  const movedAuth = await node(auth).getAttribute('transform');
  assert.notEqual(movedAuth, authPosition);
  assert.notEqual(await authWire.getAttribute('d'), wireBefore);
  assert.match(await authWire.getAttribute('marker-end'), /^url\(/);
  assert.equal(await camera.getAttribute('transform'), beforeMoveCamera);
  assert.equal(await page.locator('.graph-position-menu').count(), 0);
  const ends = await authWire.evaluate((wire) => {
    const source = document
      .querySelector(`[data-id="${wire.parentElement.dataset.from}"]`)
      .transform.baseVal.getItem(0).matrix;
    const start = wire.getPointAtLength(0);
    return { x: start.x, y: start.y, expectedX: source.e + 158, expectedY: source.f + 22 };
  });
  assert.ok(Math.abs(ends.x - ends.expectedX) < 0.01 && Math.abs(ends.y - ends.expectedY) < 0.01);
  await moveFile(palette, 35, 42);
  await node(auth).dblclick();
  assert.equal(await graph.getAttribute('data-view'), 'neighbors');
  await page.getByTestId('graph-overview').click();
  assert.equal(await node(auth).getAttribute('transform'), movedAuth);
  assert.equal(await camera.getAttribute('transform'), beforeMoveCamera);
  await page.getByTestId('graph-positions').click();
  await page.getByTestId('graph-reset-selected').click();
  assert.equal(await node(auth).getAttribute('transform'), authPosition);
  assert.equal(await authWire.getAttribute('d'), wireBefore);
  assert.notEqual(await node(palette).getAttribute('transform'), palettePosition);
  await page.getByTestId('graph-positions').click();
  await page.getByTestId('graph-reset-all').click();
  assert.equal(await node(palette).getAttribute('transform'), palettePosition);
  assert.equal(await camera.getAttribute('transform'), beforeMoveCamera);
  checks.push(
    'Right drag moves nodes with attached arrow endpoints; one/all position resets are independent; double-click and view round-trips retain the All camera and positions',
  );

  await disconnect(config);
  await select(palette);
  assert.equal(await page.getByTestId('graph-impact-view').count(), 1);
  await page.getByTestId('graph-impact-view').click();
  await page.getByTestId('graph-replay').click();
  await page.getByTestId('graph-play').click();
  await page.getByTestId('graph-scrub').fill('2');
  const pausedWave = await graph.getAttribute('data-wave');
  await page.getByTestId('graph-overview').click();
  await node(config).click();
  await page.getByTestId('restore-file').waitFor();
  await page.getByTestId('graph-impact-view').click();
  assert.equal(await graph.getAttribute('data-wave'), pausedWave);
  await page.getByTestId('graph-scrub').fill('6');
  await disconnect(palette);
  await page.getByTestId('graph-report-select').selectOption(config);
  assert.equal(await node(config).getAttribute('data-wave'), '0');
  await page.getByTestId('graph-replay').click();
  await page.getByTestId('graph-play').click();
  await select(config);
  await page.getByTestId('restore-file').click();
  await page.waitForFunction(
    () => document.querySelector('[data-testid="pull-button"]')?.disabled === false,
  );
  await page.getByTestId('graph-overview').click();
  const remaining = await page.evaluate((id) => window.repoTower.impact(id, []), palette);
  const unavailable = new Set(remaining.waves.flat());
  const states = await page
    .getByTestId('graph-node')
    .evaluateAll((nodes) =>
      nodes.map((item) => ({ id: item.dataset.id, state: item.dataset.state })),
    );
  for (const item of states)
    assert.equal(
      item.state,
      item.id === palette ? 'removed' : unavailable.has(item.id) ? 'affected' : 'standing',
      item.id,
    );
  assert.equal(await node(config).getAttribute('data-state'), 'standing');
  await page.getByTestId('undo-toolbar').click();
  await page.getByTestId('restore-file').waitFor();
  assert.equal(await node(config).getAttribute('data-state'), 'removed');
  assert.equal(await node(palette).getAttribute('data-state'), 'removed');
  await select(palette);
  await page.getByTestId('restore-file').click();
  await page.waitForFunction(
    () => document.querySelector('[data-testid="pull-button"]')?.disabled === false,
  );
  await select(config);
  await node(config).dblclick();
  assert.equal(await graph.getAttribute('data-view'), 'neighbors');
  await page.getByTestId('restore-file').click();
  await page.waitForFunction(
    () => document.querySelector('[data-testid="pull-button"]')?.disabled === false,
  );
  await page.getByTestId('graph-overview').click();
  assert.equal(await page.locator('[data-testid="graph-node"][data-state="standing"]').count(), 23);
  assert.equal(await page.getByTestId('graph-impact-view').count(), 0);
  checks.push(
    'Impact playback stays accessible after selection; earlier cuts can be replayed, individually restored and undone; remaining shared failures exactly match Rust recalculation',
  );

  await select(config);
  await page.getByTestId('pull-button').click();
  await page.getByTestId('graph-play').click();
  const livePaused = await graph.getAttribute('data-wave');
  await select(palette);
  assert.equal(await graph.getAttribute('data-wave'), livePaused);
  await select(config);
  await page.getByTestId('restore-file').click();
  await page.getByTestId('graph-overview').click();
  assert.equal(await page.locator('[data-testid="graph-node"][data-state="standing"]').count(), 23);
  checks.push('A live or paused disconnection can also be restored from its selected origin');

  assert.equal(await page.locator('.graph-bottom-note, .graph-direction').count(), 0);
  await app.evaluate(({ BrowserWindow }) => BrowserWindow.getAllWindows()[0].setSize(760, 520));
  await page.waitForFunction(() => innerWidth <= 770);
  for (const language of ['zh', 'en', 'ja']) {
    await page.getByTestId('settings-toggle').click();
    await page.getByTestId(language === 'en' ? 'theme-light' : 'theme-dark').click();
    await page.getByTestId('language-select').selectOption(language);
    await page.getByTestId('settings-toggle').click();
    await page.getByTestId('graph-help').click();
    const helpBox = await page.getByRole('dialog').boundingBox();
    const graphBox = await graph.boundingBox();
    assert.ok(
      helpBox.x >= graphBox.x &&
        helpBox.y >= graphBox.y &&
        helpBox.x + helpBox.width <= graphBox.x + graphBox.width &&
        helpBox.y + helpBox.height <= graphBox.y + graphBox.height,
    );
    assert.equal(
      await page.evaluate(
        () =>
          document.documentElement.scrollWidth <= innerWidth &&
          document.documentElement.scrollHeight <= innerHeight,
      ),
      true,
    );
    await page.screenshot({
      path: path.join(results, `interaction-help-${language}.png`),
      animations: 'disabled',
    });
    await page.keyboard.press('Escape');
    assert.equal(await page.getByRole('dialog').count(), 0);
  }
  await page.getByTestId('settings-toggle').click();
  await page.getByTestId('language-select').selectOption('zh');
  await page.getByTestId('settings-toggle').click();
  checks.push(
    'Help stays behind the question-mark button, closes with Escape, and fits the minimum window in three languages and both themes',
  );
  assert.equal(await hashSources(), beforeHash);
  assert.deepEqual(errors, []);
  assert.deepEqual(externalRequests, []);
  const report = {
    status: 'passed',
    version: metadata.version,
    packaged,
    checks,
    sourceHashUnchanged: true,
    errors,
    externalRequests,
  };
  await writeFile(
    path.join(results, packaged ? 'interactions-packaged.json' : 'interactions.json'),
    JSON.stringify(report, null, 2),
  );
  console.log(JSON.stringify(report, null, 2));
} catch (error) {
  const page = await app.firstWindow().catch(() => null);
  if (page) {
    await page.screenshot({ path: path.join(results, 'interaction-failure.png') });
    console.error((await page.locator('body').innerText()).slice(0, 2500));
  }
  console.error('Renderer errors:', errors);
  throw error;
} finally {
  await app.close();
}
