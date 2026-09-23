import fs from 'node:fs/promises';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import {
  artifacts,
  artifactBase,
  assertInsideProject,
  checksum,
  packageRoot,
  temp,
  zipDirectory,
} from './release-utils.mjs';

await fs.access(packageRoot);
await fs.mkdir(artifacts, { recursive: true });
if (process.platform === 'win32') {
  const file = path.join(artifacts, `${artifactBase}-portable.zip`);
  await zipDirectory(packageRoot, file, artifactBase);
  await checksum(file);
} else {
  const stage = path.join(temp, 'distribution');
  assertInsideProject(stage);
  await fs.rm(stage, { recursive: true, force: true });
  const folder = path.join(stage, artifactBase);
  await fs.cp(packageRoot, folder, {
    recursive: true,
    dereference: false,
    verbatimSymlinks: true,
    filter: (source) => !['runtime-data', '.DS_Store'].includes(path.basename(source)),
  });
  const file = path.join(
    artifacts,
    `${artifactBase}.${process.platform === 'darwin' ? 'zip' : 'tar.gz'}`,
  );
  assertInsideProject(file);
  await fs.rm(file, { force: true });
  const command = process.platform === 'darwin' ? 'ditto' : 'tar';
  const args =
    process.platform === 'darwin'
      ? ['-c', '-k', '--sequesterRsrc', '--keepParent', folder, file]
      : ['-czf', file, '-C', stage, artifactBase];
  const result = spawnSync(command, args, { stdio: 'inherit', shell: false });
  if (result.error || result.status !== 0)
    throw result.error ?? new Error(`${command} failed: ${result.status}`);
  await checksum(file);
  await fs.rm(stage, { recursive: true, force: true });
}
