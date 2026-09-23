import './test-env.mjs';
import { _electron as electron } from 'playwright-core';
import { createRequire } from 'node:module';
import { mkdir, writeFile, readdir, readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';
import assert from 'node:assert/strict';

const require = createRequire(import.meta.url);
const root = path.resolve(import.meta.dirname, '..');
const metadata = JSON.parse(await readFile(path.join(root, 'package.json'), 'utf8'));
const packaged = process.argv.includes('--packaged');
const release = path.join(root, 'release', `RepoTower-${metadata.version}-${process.platform}-${process.arch}`);
const executable = process.platform === 'win32' ? 'RepoTower.exe' : process.platform === 'darwin' ? 'RepoTower.app/Contents/MacOS/RepoTower' : 'RepoTower';
const resources = process.platform === 'darwin' ? path.join(release, 'RepoTower.app', 'Contents', 'Resources') : path.join(release, 'resources');
const demoRoot = packaged ? path.join(resources, 'fixtures', 'demo-project') : path.join(root, 'fixtures', 'demo-project');
const results = path.join(root, 'test-results');
const images = path.join(root, 'docs', 'images');
await Promise.all([mkdir(results, { recursive: true }), mkdir(images, { recursive: true })]);
async function sourceHash() {
  const hash = createHash('sha256');
  const files = (await readdir(demoRoot, { recursive: true, withFileTypes: true }))
    .filter(entry => entry.isFile()).map(entry => path.join(entry.parentPath, entry.name)).sort();
  for (const file of files) { hash.update(path.relative(demoRoot, file)); hash.update(await readFile(file)); }
  return hash.digest('hex');
}
const beforeHash = await sourceHash();
const app = await electron.launch({
  executablePath: packaged ? path.join(release, executable) : require('electron'), args: packaged ? [] : [root], cwd: root,
  env: { ...process.env, TEMP: path.join(root, '.tmp'), TMP: path.join(root, '.tmp') }, timeout: 60000,
});
const checks = [], errors = [], externalRequests = [], waveEvidence = [];
try {
  const page = await app.firstWindow();
  page.setDefaultTimeout(15000);
  page.on('pageerror', error => errors.push(error.message));
  page.on('request', request => { if (/^https?:/.test(request.url())) externalRequests.push(request.url()); });
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.getByTestId('welcome').waitFor();
  async function appearance(theme, language) {
    if (!(await page.locator('.settings-popover').count())) await page.getByTestId('settings-toggle').click();
    if (theme) await page.getByTestId(`theme-${theme}`).click();
    if (language) await page.getByTestId('language-select').selectOption(language);
    await page.getByTestId('settings-toggle').click();
    if (theme) assert.equal(await page.locator('.app').getAttribute('data-theme'), theme);
  }
  async function shot(name, documentationName) {
    await page.mouse.move(8, 8);
    await page.evaluate(() => document.activeElement?.blur?.());
    await page.screenshot({ path: path.join(results, name), animations: 'disabled' });
    if (documentationName) await page.screenshot({ path: path.join(images, documentationName), animations: 'disabled' });
  }
  async function selectFile(query) {
    await page.getByTestId('module-search').fill(query);
    await page.getByTestId('module-list-item').first().click();
    await page.waitForFunction(() => document.querySelector('[data-testid="pull-button"]')?.disabled === false);
  }
  await appearance('dark', 'zh');
  const host = await app.evaluate(({ app, BrowserWindow }) => {
    const window = BrowserWindow.getAllWindows()[0];
    return { bounds: window.getBounds(), minimum: window.getMinimumSize(), userData: app.getPath('userData'),
      sessionData: app.getPath('sessionData'), prefs: window.webContents.getLastWebPreferences() };
  });
  // Fractional Windows display scaling rounds the outer frame by a few pixels.
  assert.ok(Math.abs(host.bounds.width - 980) <= 4);
  assert.ok(Math.abs(host.bounds.height - 680) <= 4);
  assert.deepEqual(host.minimum, [760, 520]);
  await appearance('dark', 'zh');
  await shot('01-welcome.png');
  await page.getByRole('button', { name: '放大 / 还原', exact: true }).click();
  await page.waitForFunction(async () => (await window.repoTower.getWindowState()).maximized);
  await page.getByRole('button', { name: '放大 / 还原', exact: true }).click();
  await page.waitForFunction(async () => !(await window.repoTower.getWindowState()).maximized);
  await page.getByRole('button', { name: '最小化', exact: true }).click();
  assert.equal(await app.evaluate(({ BrowserWindow }) => BrowserWindow.getAllWindows()[0].isMinimized()), true);
  await app.evaluate(({ BrowserWindow }) => BrowserWindow.getAllWindows()[0].restore());
  checks.push('Compact 980×680 window, 760×520 minimum, native maximize/restore/minimize');
  await page.getByTestId('open-demo').click();
  await page.getByTestId('workspace').waitFor({ timeout: 60000 });
  await page.getByTestId('graph-scene').waitFor();
  assert.equal(await page.getByTestId('graph-node').count(), 23);
  assert.equal(await page.locator('canvas').count(), 0);
  assert.match(await page.locator('.repo-count').textContent(), /23.*24/s);
  const analysis = await page.evaluate(() => window.repoTower.loadDemo());
  assert.equal(analysis.totalModules, 23);
  assert.equal(analysis.totalEdges, 24);
  const config = analysis.modules.find(module => module.relativePath === 'src/core/config.ts');
  assert.ok(config);
  const impact = await page.evaluate(id => window.repoTower.impact(id, []), config.id);
  assert.equal(impact.totalAffected, 13);
  assert.deepEqual(impact.waves.map(wave => wave.length), [1, 2, 4, 3, 2, 1, 1]);
  const realEdges = new Set(analysis.edges.map(edge => `${edge.target}\0${edge.source}`));
  const overviewEdges = await page.getByTestId('graph-edge').evaluateAll(edges => edges.map(edge => [edge.dataset.from, edge.dataset.to]));
  assert.equal(overviewEdges.length, 24);
  for (const [from, to] of overviewEdges) assert.ok(realEdges.has(`${from}\0${to}`), `Invented edge ${from} → ${to}`);
  await shot('02-circuit-dark.png', 'circuit-dark.png');
  checks.push('Real Rust scan: 23 modules, 24 exact dependency→consumer SVG connections, no 3D canvas');
  await selectFile('config.ts');
  assert.match(await page.getByTestId('potential-count').textContent(), /^13(?:\s*\/\s*23)?$/);
  assert.equal(await page.locator('.dock-file strong').textContent(), 'config.ts');
  await shot('03-config-preview.png');
  await appearance('light', 'en');
  assert.equal(await page.getByTestId('pull-button').innerText(), 'Disconnect');
  assert.equal(await page.locator('.dock-file strong').textContent(), 'config.ts');
  await page.getByTestId('graph-overview').click();
  await shot('04-circuit-light.png', 'circuit-light.png');
  await appearance(null, 'ja');
  assert.equal(await page.getByTestId('pull-button').innerText(), '切り離す');
  assert.equal(await page.locator('.dock-file strong').textContent(), 'config.ts');
  await shot('05-japanese.png');
  await appearance('dark', 'zh');
  checks.push('Search config.ts predicts 13 affected; themes and Chinese/English/Japanese preserve selection');
  await page.getByTestId('pull-button').click();
  await page.getByTestId('graph-play').waitFor();
  await page.getByTestId('graph-play').click();
  await page.getByTestId('graph-scrub').focus();
  await page.getByTestId('graph-scrub').press('Home');
  const graph = page.getByTestId('graph-scene');
  assert.equal(await graph.getAttribute('data-wave'), '0');
  await page.waitForTimeout(400);
  assert.equal(await graph.getAttribute('data-wave'), '0');
  assert.equal(await page.getByTestId('graph-affected-count').textContent(), '0');
  const waveById = new Map(impact.waves.flatMap((ids, wave) => ids.map(id => [id, wave])));
  const drawnEdges = await page.getByTestId('graph-edge').evaluateAll(edges => edges.map(edge => ({
    from: edge.dataset.from, to: edge.dataset.to, causal: edge.dataset.causal === 'true',
  })));
  for (const edge of drawnEdges) {
    assert.ok(realEdges.has(`${edge.from}\0${edge.to}`), `Invented impact edge ${edge.from} → ${edge.to}`);
    assert.equal(edge.causal, waveById.get(edge.to) === waveById.get(edge.from) + 1);
  }
  for (let wave = 0; wave < impact.waves.length; wave++) {
    if (wave > 0) await page.getByTestId('graph-next').click();
    assert.equal(await graph.getAttribute('data-wave'), String(wave));
    const states = await page.getByTestId('graph-node').evaluateAll(nodes => nodes.map(node => ({
      id: node.dataset.id, wave: Number(node.dataset.wave), state: node.dataset.state,
    })));
    assert.equal(states.length, 14);
    for (const node of states) {
      assert.equal(node.wave, waveById.get(node.id));
      assert.equal(node.state, node.wave === 0 ? 'removed' : node.wave <= wave ? 'affected' : 'pending');
    }
    const arrived = impact.waves.slice(1, wave + 1).flat().length;
    assert.equal(await page.getByTestId('graph-affected-count').textContent(), String(arrived));
    waveEvidence.push({ wave, arrived, states });
    if (wave === 2) await shot('06-step-two.png');
  }
  await page.getByTestId('graph-prev').click();
  assert.equal(await graph.getAttribute('data-wave'), '5');
  assert.equal(await page.getByTestId('graph-affected-count').textContent(), '12');
  await page.getByTestId('graph-play').click();
  await page.locator('.graph-pulse').first().waitFor();
  await page.getByTestId('impact-report').waitFor();
  assert.equal(await graph.getAttribute('data-wave'), '6');
  assert.equal(await page.getByTestId('graph-affected-count').textContent(), '13');
  assert.equal(await page.getByTestId('graph-untouched-count').textContent(), '9');
  assert.deepEqual(await page.getByTestId('impact-report').locator('b').allTextContents(), ['1', '13', '9']);
  await shot('07-circuit-impact.png', 'circuit-impact.png');
  await page.getByRole('button', { name: '放大', exact: true }).click();
  const camera = page.locator('.graph-board > g[transform]');
  const zoomedTransform = await camera.getAttribute('transform');
  await page.locator('[data-testid="graph-node"][data-id="src/core/auth.ts"]').click();
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  assert.equal(await camera.getAttribute('transform'), zoomedTransform);
  assert.match(await page.locator('.graph-tip-path').textContent(), /auth\.ts$/);
  await page.locator('.graph-board').click({ position: { x: 10, y: 100 } });
  await page.getByTestId('graph-fit').click();
  checks.push('Pause and 7 explicit steps match Rust waves; real shortest-path edges carry pulses; final 13 affected and 9 intact');
  await page.getByTestId('graph-overview').click();
  assert.equal(await page.getByTestId('graph-node').count(), 23);
  assert.equal(await page.locator('[data-testid="graph-node"][data-state="standing"]').count(), 9);
  assert.equal(await page.locator('[data-testid="graph-node"][data-state="affected"]').count(), 13);
  assert.equal(await page.locator('[data-testid="graph-node"][data-state="removed"]').count(), 1);
  await shot('09-impact-overview.png');
  await page.getByTestId('graph-impact-view').click();
  await page.getByTestId('graph-replay').click();
  await page.getByTestId('graph-play').click();
  assert.deepEqual(await page.getByTestId('impact-report').locator('b').allTextContents(), ['1', '13', '9']);
  await page.getByTestId('undo-report').click();
  await page.getByTestId('graph-overview').click();
  assert.equal(await page.locator('[data-testid="graph-node"][data-state="standing"]').count(), 23);
  await page.waitForFunction(() => document.querySelector('[data-testid="pull-button"]')?.disabled === false);
  await page.getByTestId('pull-button').click();
  await page.getByTestId('graph-skip').click();
  await page.getByTestId('impact-report').waitFor();
  const palette = analysis.modules.find(module => module.relativePath === 'src/ui/palette.ts');
  assert.ok(palette);
  const nextImpact = await page.evaluate(({ id, excluded }) => window.repoTower.impact(id, excluded),
    { id: palette.id, excluded: impact.waves.flat() });
  assert.equal(nextImpact.totalAffected, 3);
  await selectFile('palette.ts');
  assert.match(await page.getByTestId('potential-count').textContent(), /^3(?:\s*\/\s*23)?$/);
  await page.getByTestId('pull-button').click();
  await page.getByTestId('graph-skip').click();
  await page.getByTestId('impact-report').waitFor();
  assert.deepEqual(await page.getByTestId('impact-report').locator('b').allTextContents(), ['1', '3', '5']);
  await page.getByTestId('graph-overview').click();
  assert.equal(await page.locator('[data-testid="graph-node"][data-state="standing"]').count(), 5);
  assert.equal(await page.locator('[data-testid="graph-node"][data-state="affected"]').count(), 16);
  assert.equal(await page.locator('[data-testid="graph-node"][data-state="removed"]').count(), 2);
  await shot('10-consecutive-cuts.png');
  await page.getByTestId('reset-toolbar').click();
  await page.getByTestId('graph-overview').click();
  assert.equal(await page.locator('[data-testid="graph-node"][data-state="standing"]').count(), 23);
  assert.equal(await page.getByTestId('impact-report').count(), 0);
  checks.push('Overview shows intact branches; replay/undo preserve state; consecutive config and palette cuts exclude prior failures; reset restores all 23 files');
  await app.evaluate(({ BrowserWindow }) => BrowserWindow.getAllWindows()[0].setSize(760, 520));
  await page.waitForFunction(() => innerWidth <= 770);
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth && document.documentElement.scrollHeight <= innerHeight), true);
  const dock = await page.getByTestId('selection-dock').boundingBox();
  assert.ok(dock && dock.x >= 0 && dock.y > 0);
  await shot('08-minimum-window.png');
  await app.evaluate(({ BrowserWindow }) => BrowserWindow.getAllWindows()[0].setSize(980, 680));
  checks.push('Minimum 760×520 window keeps the graph and dock inside the viewport');
  async function chooseDirectory(directory) {
    await app.evaluate(({ dialog }, selected) => {
      dialog.showOpenDialog = async () => ({ canceled: false, filePaths: [selected] });
    }, directory);
    await page.getByTestId('title-open').click();
  }
  const emptyDirectory = path.join(root, '.tmp', 'empty-e2e-repository');
  await mkdir(emptyDirectory, { recursive: true });
  await chooseDirectory(emptyDirectory);
  await page.locator('.empty-project').waitFor();
  assert.match(await page.locator('.empty-project').textContent(), /没有找到支持的源码文件/);
  await chooseDirectory(path.join(root, '.tmp', `missing-e2e-${Date.now()}`));
  await page.getByRole('alert').waitFor();
  await page.getByTestId('close-error').click();
  await page.getByTestId('home').click();
  await page.getByTestId('open-demo').click();
  await page.getByTestId('graph-node').first().waitFor();
  assert.equal(await page.getByTestId('graph-node').count(), 23);
  checks.push('Folder picker IPC handles empty and invalid directories with recoverable UI');
  const largeDirectory = path.join(root, '.tmp', 'graph-e2e-300');
  await mkdir(largeDirectory, { recursive: true });
  await Promise.all(Array.from({ length: 300 }, (_, index) => writeFile(
    path.join(largeDirectory, `module-${String(index).padStart(3, '0')}.ts`),
    `${index ? `import './module-${String(Math.floor((index - 1) / 2)).padStart(3, '0')}';\n` : ''}export const value = ${index};\n`,
  )));
  await chooseDirectory(largeDirectory);
  await page.waitForFunction(() => document.querySelector('.repo-count')?.textContent?.includes('300'));
  assert.equal(await page.getByTestId('graph-node').count(), 250);
  assert.match(await page.locator('.graph-limit').textContent(), /250\s*\/\s*300/);
  await selectFile('module-299.ts');
  assert.equal(await page.locator('.dock-file strong').textContent(), 'module-299.ts');
  await page.getByRole('button', { name: '相邻', exact: true }).click();
  assert.ok(await page.getByTestId('graph-node').count() < 10);
  await shot('11-large-map-search.png');
  checks.push('300-file repository caps overview at 250 visible nodes and search reaches an omitted file');
  await appearance('light', 'ja');
  await page.reload();
  await page.getByTestId('welcome').waitFor();
  assert.equal(await page.locator('.app').getAttribute('data-theme'), 'light');
  assert.equal(await page.locator('html').getAttribute('lang'), 'ja');
  assert.equal(await page.locator('.welcome h1').textContent(), 'リポジトリを開く');
  await appearance('dark', 'zh');
  checks.push('Theme and language survive full reload; test leaves Chinese/dark defaults');
  assert.ok(host.userData.startsWith(root));
  assert.ok(host.sessionData.startsWith(root));
  assert.equal(host.prefs.sandbox, true);
  assert.equal(host.prefs.contextIsolation, true);
  assert.equal(host.prefs.nodeIntegration, false);
  assert.equal(await page.evaluate(() => typeof window.require), 'undefined');
  checks.push('Renderer sandbox, context isolation and project-local application data remain enforced');
  assert.equal(await page.evaluate(async () => {
    try { await fetch('https://example.com/repotower-offline-probe'); return false; } catch { return true; }
  }), true);
  assert.deepEqual(externalRequests, []);
  checks.push('CSP blocks external network access before requests leave the renderer');
  assert.equal(await sourceHash(), beforeHash);
  assert.deepEqual(errors, []);
  checks.push('All demo source hashes unchanged after simulation/replay/undo/reset; no renderer errors');
  const report = { status: 'passed', version: metadata.version, packaged, checks, waveEvidence, errors, externalRequests };
  await writeFile(path.join(results, packaged ? 'desktop-packaged.json' : 'desktop.json'), JSON.stringify(report, null, 2));
  console.log(JSON.stringify({ ...report, waveEvidence: `${waveEvidence.length} verified steps` }, null, 2));
} catch (error) {
  const page = await app.firstWindow().catch(() => null);
  if (page) {
    await page.screenshot({ path: path.join(results, 'failure.png') }).catch(() => {});
    console.error((await page.locator('body').innerText()).slice(0, 6000));
  }
  console.error('Renderer errors:', errors);
  throw error;
} finally { await app.close(); }
