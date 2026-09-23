// The native executable is already self-contained. No archive or launcher stub.
import fs from 'node:fs/promises';
import path from 'node:path';
import { artifacts, artifactBase, checksum, packageRoot } from './release-utils.mjs';
if (process.platform !== 'win32') throw new Error('Build the Windows EXE on Windows.');
await fs.mkdir(artifacts,{recursive:true});
const output=path.join(artifacts,`${artifactBase}.exe`);
await fs.copyFile(path.join(packageRoot,'RepoTower.exe'),output);
await checksum(output);
