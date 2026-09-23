import fs from 'node:fs/promises';
import path from 'node:path';
import crypto from 'node:crypto';
import { gzipSync } from 'node:zlib';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const folder = path.join(root, '.cache', 'native-assets');
await fs.mkdir(folder, { recursive: true });
const manifest = JSON.parse(await fs.readFile(path.join(root, 'assets', 'fonts', 'manifest.json')));
for (const asset of manifest) {
  const destination = path.join(folder, asset.name);
  let bytes = await fs.readFile(destination).catch(() => null);
  if (!bytes || crypto.createHash('sha256').update(bytes).digest('hex') !== asset.sha256) {
    console.log(`Downloading ${asset.name} into the project cache…`);
    for (let attempt=0;attempt<4;attempt++) {
      try {
        const response = await fetch(asset.url, { signal: AbortSignal.timeout(180_000) });
        if (!response.ok) throw new Error(`${response.status}: ${asset.url}`);
        bytes = Buffer.from(await response.arrayBuffer());
        break;
      } catch(error) { if(attempt===3) throw error; console.log(`Retrying font download (${attempt+1}/3)…`); }
    }
    if (crypto.createHash('sha256').update(bytes).digest('hex') !== asset.sha256) throw new Error(`Checksum mismatch: ${asset.name}`);
    await fs.writeFile(destination, bytes);
  }
  const compressed=gzipSync(bytes,{level:9,mtime:0});
  const previous=await fs.readFile(`${destination}.gz`).catch(()=>null);
  if (!previous?.equals(compressed)) await fs.writeFile(`${destination}.gz`,compressed);
}
console.log('Native assets verified. No runtime download is needed.');
