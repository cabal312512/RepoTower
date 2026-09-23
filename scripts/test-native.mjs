import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { setTimeout as delay } from 'node:timers/promises';
import { launch, root } from './native-driver.mjs';
import { packageRoot } from './release-utils.mjs';

const executable = process.argv.includes('--packaged')
  ? path.join(packageRoot, ...process.platform === 'darwin' ? ['RepoTower.app','Contents','MacOS','RepoTower'] : [process.platform === 'win32' ? 'RepoTower.exe' : 'RepoTower'])
  : path.resolve(process.argv[2] || path.join(root, 'target', 'release', process.platform === 'win32' ? 'RepoTower.exe' : 'RepoTower'));
const runRoot = path.join(root, 'test-results', `native-${process.platform}-${process.arch}-${Date.now()}`);
const data = path.join(runRoot, 'runtime-data');
const checks = [];
async function hashFolder(folder) {
  const files = {};
  async function walk(dir) { for (const e of await fs.readdir(dir, { withFileTypes: true })) { const file = path.join(dir, e.name); if (e.isDirectory()) await walk(file); else if (e.isFile()) files[path.relative(folder, file)] = createHash('sha256').update(await fs.readFile(file)).digest('hex'); } }
  await walk(folder); return files;
}
const sourceBefore = await hashFolder(path.join(root, 'fixtures'));
let app = await launch(executable, data);
async function check(name, fn) { await fn(); checks.push(name); console.log(`PASS ${name}`); }
const node = id => `node:${id}`;
const config = 'src/core/config.ts';
async function drag(id, dx, dy) {
  const s = await app.snapshot(); const r = s.controls[node(id)]; assert(r, `Node missing: ${id}`);
  const x = r[0] + r[2] / 2, y = r[1] + r[3] / 2;
  await app.send({ action: 'button', x, y, button: 'right', down: true });
  await app.send({ action: 'move', x: x + dx, y: y + dy });
  await app.send({ action: 'button', x: x + dx, y: y + dy, button: 'right', down: false });
  return app.snapshot();
}
try {
  await check('Native launch defaults to English and light', async () => { const s = await app.snapshot(); assert.equal(s.renderer, 'native-wgpu'); assert.equal(s.language, 'en'); assert.equal(s.theme, 'light'); assert.equal(s.error, null); });
  await check('Embedded example opens offline', async () => { await app.click('welcome-demo'); const s = await app.waitFor(s => s.modules === 23); assert.equal(s.edgeCount, 24); assert.equal(s.nodes.length, 23); assert.equal(s.edges.length, 24); });
  await app.send({ action: 'move', x: 400, y: 90 });
  await app.screenshot(path.join(runRoot, 'light.png'));
  await check('Selection preserves zoom and pan', async () => { await app.click('zoom-in'); const before = await app.snapshot(); await app.click(node(config)); const after = await app.snapshot(); assert.equal(after.selected, config); assert.deepEqual(after.camera, before.camera); });
  await app.click('fit');
  await check('Hover preview waits while a node is selected', async () => { const s = await app.snapshot(); const r = s.controls[node('src/core/auth.ts')]; const x = r[0] + r[2]/2, y = r[1] + r[3]/2; await app.send({ action: 'move', x, y }); const first = await app.snapshot(); assert.notEqual(first.hovered, 'src/core/auth.ts'); await delay(410); const after = await app.snapshot(); assert.equal(after.hovered, 'src/core/auth.ts'); assert.equal(after.selected, config); });
  await check('Right drag moves the file and connected routes', async () => { const before = await app.snapshot(); const original = before.nodes.find(n => n.id === config); const after = await drag(config, 48, 28); const moved = after.nodes.find(n => n.id === config); assert(Math.abs(moved.x - original.x - 48 / before.camera.scale) < 0.1); assert(Math.abs(moved.y - original.y - 28 / before.camera.scale) < 0.1); assert.deepEqual(after.edges, before.edges); assert.deepEqual(after.camera, before.camera); });
  await check('Reset selected position leaves other positions intact', async () => { await drag('src/utils/format.ts', 15, -18); const second = (await app.snapshot()).nodes.find(n => n.id === 'src/utils/format.ts'); await app.click(node(config)); await app.click('positions'); await app.click('reset-selected-position'); const reset = await app.snapshot(); assert.equal(reset.nodes.find(n => n.id === config).x, 44); assert.deepEqual(reset.nodes.find(n => n.id === 'src/utils/format.ts'), second); });
  await check('Reset all positions restores the remaining moved file', async () => { await app.click('positions'); await app.click('reset-all-positions'); assert.equal((await app.snapshot()).nodes.find(n => n.id === 'src/utils/format.ts').x, 44); });
  await check('Double-click opens Nearby and returning restores the camera', async () => { const before = await app.snapshot(); const r = before.controls[node(config)]; const x = r[0]+r[2]/2, y=r[1]+r[3]/2; await delay(500); await app.clickAt(x,y); await app.clickAt(x,y); const s = await app.snapshot(); assert.equal(s.view, 'Nearby'); assert.equal(s.nodes.length, 3); await app.click('view-all'); assert.deepEqual((await app.snapshot()).camera, before.camera); });
  await check('Disconnect follows real dependency waves', async () => { await app.click('file-action'); await app.click('play'); const s = await app.snapshot(); assert.equal(s.view, 'Impact'); assert.equal(s.report.totalAffected, 13); assert.deepEqual(s.report.waves.map(w=>w.length), [1,2,4,3,2,1,1]); assert.equal(s.nodes.length,14); assert.equal(s.playing,false); });
  await check('Step, scrub and complete preserve the cut', async () => { await app.click('next'); assert((await app.snapshot()).playhead >= 1); const s = await app.snapshot(), r=s.controls.scrub; await app.clickAt(r[0]+r[2]*0.5,r[1]+r[3]/2); assert(Math.abs((await app.snapshot()).playhead-3)<0.1); await app.click('skip'); const done = await app.snapshot(); assert.equal(done.unavailable.length,14); assert.deepEqual(done.origins,[config]); assert.equal(done.live,false); });
  await app.send({ action: 'move', x: 400, y: 90 });
  await app.screenshot(path.join(runRoot, 'impact.png'));
  await check('Impact replay remains accessible after selecting elsewhere', async () => { await app.click('view-all'); await app.click(node('src/utils/format.ts')); await app.click('view-impact'); await app.click('replay'); assert.equal((await app.snapshot()).playing,true); await app.click('skip'); });
  await check('Multiple cuts preserve independent history', async () => { await app.click('view-all'); await app.click(node('src/ui/palette.ts')); await app.click('file-action'); await app.click('skip'); assert.equal((await app.snapshot()).reports.length,2); await app.click('reports'); await app.click(`report-${config}`); const s=await app.snapshot(); assert.equal(s.report.removedModuleId,config); assert.equal(s.view,'Impact'); });
  await check('Reselect and restore one cut keeps shared failures', async () => { await app.click('view-all'); await app.click(node(config)); await app.click('file-action'); const s=await app.snapshot(); assert.deepEqual(s.origins,['src/ui/palette.ts']); assert(!s.unavailable.includes('src/core/auth.ts')); assert(s.unavailable.includes('src/app/main.ts')); });
  await check('Undo and restore all', async () => { await app.key('Z',true); assert.equal((await app.snapshot()).origins.length,2); await app.click('restore-all'); const s=await app.snapshot(); assert.equal(s.origins.length,0); assert.equal(s.unavailable.length,0); });
  await check('Dark, Chinese and Japanese settings render and persist', async () => { await app.click('settings'); await app.click('theme-dark'); await app.click('language-zh'); assert.equal((await app.snapshot()).language,'zh'); await app.key('Escape'); await app.screenshot(path.join(runRoot,'dark-zh.png')); await app.click('settings'); await app.click('language-ja'); await app.key('Escape'); await app.screenshot(path.join(runRoot,'dark-ja.png')); const s=await app.snapshot(); assert.equal(s.language,'ja'); assert.equal(s.theme,'dark'); });
  await check('Help is separate and translated', async () => { await app.click('help'); await app.screenshot(path.join(runRoot,'help-ja.png')); await app.key('Escape'); });
  await check('Minimum window remains usable', async () => { await app.send({ action:'resize',width:760,height:520 }); await delay(200); await app.screenshot(path.join(runRoot,'small.png')); const s=await app.snapshot(); assert(s.controls['restore-all'][0]<760); await app.send({action:'resize',width:980,height:680}); });
  await check('Search and relation navigation select real files',async()=>{
    await app.click('search');await app.send({action:'text',text:'config.ts'});await app.click(`result-${config}`);
    assert.equal((await app.snapshot()).selected,config);await app.click('callers');await app.click('result-src/core/auth.ts');
    assert.equal((await app.snapshot()).selected,'src/core/auth.ts');
    await app.click('search');await app.key('A',true);await app.send({action:'text',text:'palette.ts'});await app.key('Enter');
    assert.equal((await app.snapshot()).selected,'src/ui/palette.ts');
  });
  await check('A paused live cut can be restored immediately',async()=>{
    await app.click('file-action');await app.click('play');await app.click(node('src/ui/palette.ts'));await app.click('file-action');
    const s=await app.snapshot();assert.equal(s.live,false);assert.equal(s.unavailable.length,0);assert.equal(s.origins.length,0);
  });
  await check('An earlier cut can be restored during a later animation',async()=>{
    await app.click(node(config));await app.click('file-action');await app.click('skip');await app.click('view-all');
    await app.click(node('src/ui/palette.ts'));await app.click('file-action');await app.click('play');await app.click('view-all');
    await app.click(node(config));await app.click('file-action');let s=await app.snapshot();assert.equal(s.live,false);assert.deepEqual(s.origins,['src/ui/palette.ts']);assert(s.unavailable.includes('src/app/main.ts'));assert(!s.unavailable.includes('src/core/auth.ts'));await app.click('restore-all');
  });
  await app.close(); app=await launch(executable,data);
  await check('Warm launch loads adjacent preferences', async () => { const s=await app.snapshot(); assert.equal(s.language,'ja'); assert.equal(s.theme,'dark'); });
  await app.click('settings'); await app.click('theme-light'); await app.click('language-en'); await app.key('Escape');
  for (const language of ['java','python','c','cpp','go','rust','csharp']) {
    await check(`Native ${language} analysis and impact`, async () => {
      await app.send({action:'drop',path:path.join(root,'fixtures','language-samples',language)});
      const s=await app.waitFor(s=>!s.loading&&s.modules!==null); assert.equal(s.modules,3); assert(s.edgeCount>=2); assert.equal(s.error,null);
      const base=s.nodes.find(n=>/config\.(java|py|h|hpp|go|rs|cs)$/i.test(n.id)); assert(base,`Config file missing for ${language}`);
      await app.click(node(base.id)); await app.click('file-action'); await app.click('skip'); assert.equal((await app.snapshot()).report.totalAffected,2);
    });
  }
  await check('Search reaches files outside the 250-node overview',async()=>{
    const folder=path.join(runRoot,'large-项目');await fs.mkdir(folder,{recursive:true});
    await Promise.all(Array.from({length:280},(_,i)=>fs.writeFile(path.join(folder,`module-${String(i).padStart(3,'0')}.ts`),`export const value = ${i};\n`)));
    await app.send({action:'drop',path:folder});let s=await app.waitFor(s=>s.modules===280);assert.equal(s.nodes.length,250);assert(!s.nodes.some(n=>n.id==='module-279.ts'));
    await app.click('search');await app.send({action:'text',text:'module-279'});await app.key('Enter');s=await app.snapshot();assert.equal(s.selected,'module-279.ts');assert(s.nodes.some(n=>n.id==='module-279.ts'));
  });
  await check('Empty projects and fresh scans clear previous simulation state',async()=>{
    const folder=path.join(runRoot,'empty');await fs.mkdir(folder);await app.send({action:'drop',path:folder});const s=await app.waitFor(s=>s.modules===0);assert.equal(s.nodes.length,0);assert.equal(s.origins.length,0);
    await app.send({action:'drop',path:path.join(root,'fixtures','demo-project')});await app.waitFor(s=>s.modules===23);
  });
  await check('Inspected source files remain byte-for-byte unchanged', async () => { assert.deepEqual(await hashFolder(path.join(root,'fixtures')),sourceBefore); });
  await fs.writeFile(path.join(runRoot,'checks.json'),JSON.stringify({executable,checks},null,2));
  console.log(`${checks.length} native GUI checks passed. Screenshots: ${runRoot}`);
} catch (error) {
  await app.screenshot(path.join(runRoot,'failure.png')).catch(()=>{});
  await fs.writeFile(path.join(runRoot,'failure.json'),JSON.stringify({error:error.stack,state:await app.snapshot().catch(()=>null),stderr:app.stderr},null,2));
  throw error;
} finally { await app.close(); }
