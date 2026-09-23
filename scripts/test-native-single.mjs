import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import { artifactBase, artifacts, packageRoot, digest, root } from './release-utils.mjs';
import { launch } from './native-driver.mjs';

const source=path.join(artifacts,`${artifactBase}.exe`);
assert.equal(await digest(source),await digest(path.join(packageRoot,'RepoTower.exe')));
const directory=path.join(root,'test-results',`single-native-${Date.now()}`);
await fs.mkdir(directory,{recursive:true});
const executable=path.join(directory,'RepoTower.exe');await fs.copyFile(source,executable);
assert.deepEqual(await fs.readdir(directory),['RepoTower.exe']);
const app=await launch(executable,null);
try {
  const initial=await app.snapshot();assert.equal(path.resolve(initial.dataDir),path.join(directory,'runtime-data'));
  await app.click('welcome-demo');const s=await app.waitFor(s=>s.modules===23);
  assert.equal(s.edgeCount,24);assert.equal(s.theme,'light');assert.equal(s.language,'en');
  await app.screenshot(path.join(directory,'single.png'));
  assert(!(await fs.readdir(directory)).some(name=>name==='RepoTower-data'));
  assert((await fs.stat(executable)).size<60_000_000,'Native executable size budget exceeded');
  console.log('PASS single native EXE runs alone; no payload extraction; adjacent portable data');
} finally {await app.close();}
