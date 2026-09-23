// Capture actual native frames for documentation, without changing the user's profile.
import fs from 'node:fs/promises';
import path from 'node:path';
import { launch,root } from './native-driver.mjs';
import { packageRoot } from './release-utils.mjs';
const executable=path.join(packageRoot,...process.platform==='darwin'?['RepoTower.app','Contents','MacOS','RepoTower']:[process.platform==='win32'?'RepoTower.exe':'RepoTower']);
const app=await launch(executable,path.join(root,'test-results',`capture-${Date.now()}`));
const output=path.join(root,'docs','images');
await fs.mkdir(output,{recursive:true});
try {
  await app.screenshot(path.join(output,'native-welcome.png'));
  await app.click('welcome-demo');await app.waitFor(s=>s.modules===23);
  await app.send({action:'move',x:400,y:90});await app.screenshot(path.join(output,'release-light.png'));
  await fs.copyFile(path.join(output,'release-light.png'),path.join(output,'circuit-light.png'));
  await app.click('node:src/core/config.ts');await app.click('file-action');await app.click('skip');
  await app.send({action:'move',x:400,y:90});await app.screenshot(path.join(output,'release-impact.png'));
  await fs.copyFile(path.join(output,'release-impact.png'),path.join(output,'circuit-impact.png'));
  await app.click('restore-all');await app.click('settings');await app.click('theme-dark');await app.key('Escape');
  await app.send({action:'move',x:400,y:90});await app.screenshot(path.join(output,'circuit-dark.png'));
  await app.click('settings');await app.click('theme-light');await app.key('Escape');
  await app.send({action:'drop',path:path.join(root,'fixtures','language-samples','java')});
  const s=await app.waitFor(s=>s.modules===3);await app.click(`node:${s.nodes.find(n=>n.id.endsWith('/Config.java')).id}`);
  await app.click('file-action');await app.click('skip');await app.send({action:'move',x:400,y:90});await app.screenshot(path.join(output,'java-impact.png'));
  console.log(`Captured native documentation images: ${output}`);
} finally {await app.close();}
