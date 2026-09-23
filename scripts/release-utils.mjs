import fs from 'node:fs/promises';
import { createReadStream, createWriteStream } from 'node:fs';
import { pipeline } from 'node:stream/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';

export const root = path.resolve(import.meta.dirname, '..');
export const metadata = JSON.parse(await fs.readFile(path.join(root, 'package.json'), 'utf8'));
export const packageRoot = path.join(
  root,
  'release',
  `RepoTower-${metadata.version}-${process.platform}-${process.arch}`,
);
export const artifacts = path.join(root, 'release', 'artifacts');
export const temp = path.join(root, '.tmp');
export const platformName = { win32: 'windows', darwin: 'macos', linux: 'linux' }[process.platform];
export const artifactBase = `RepoTower-${metadata.version}-${platformName}-${process.arch}`;

export function assertInsideProject(target) {
  const relative = path.relative(root, path.resolve(target));
  if (
    !relative ||
    relative === '..' ||
    relative.startsWith(`..${path.sep}`) ||
    path.isAbsolute(relative)
  ) {
    throw new Error(`Refusing to change a path outside the project: ${target}`);
  }
}

export async function digest(file) {
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(file)) hash.update(chunk);
  return hash.digest('hex');
}

export async function checksum(file) {
  const line = `${await digest(file)}  ${path.basename(file)}\n`;
  await fs.writeFile(`${file}.sha256`, line);
  console.log(
    `${path.basename(file)} (${(await fs.stat(file)).size.toLocaleString('en')} bytes)\n${line.trim()}`,
  );
}

export async function zipDirectory(directory, output, prefix = '') {
  const { default: yazl } = await import('yazl');
  assertInsideProject(output);
  await fs.mkdir(path.dirname(output), { recursive: true });
  const zip = new yazl.ZipFile();
  const completion = pipeline(zip.outputStream, createWriteStream(output));
  // Attach a handler immediately while files are being enumerated.
  completion.catch(() => {});
  async function add(folder, relative) {
    const entries = (await fs.readdir(folder, { withFileTypes: true })).sort((a, b) =>
      a.name.localeCompare(b.name),
    );
    for (const item of entries) {
      if (item.name === 'runtime-data' || item.name === '.DS_Store') continue;
      const source = path.join(folder, item.name);
      const destination = path.posix.join(relative, item.name);
      if (item.isSymbolicLink())
        throw new Error(`Use the native archive tool for symlinks: ${source}`);
      if (item.isDirectory()) await add(source, destination);
      else if (item.isFile()) zip.addFile(source, destination, { compressionLevel: 6 });
    }
  }
  try {
    await add(directory, prefix);
    zip.end();
    await completion;
  } catch (error) {
    zip.outputStream.destroy(error);
    await completion.catch(() => {});
    throw error;
  }
}
