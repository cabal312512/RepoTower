import fs from 'node:fs/promises';
import { createReadStream, createWriteStream } from 'node:fs';
import { pipeline } from 'node:stream/promises';
import path from 'node:path';
import { Data, NtExecutable, NtExecutableResource, Resource } from 'resedit';
import {
  root,
  metadata,
  artifacts,
  artifactBase,
  checksum,
  digest,
  packageRoot,
  temp,
  zipDirectory,
} from './release-utils.mjs';

if (process.platform !== 'win32') throw new Error('The single EXE must be built on Windows.');
await fs.access(path.join(packageRoot, 'RepoTower.exe'));
const stub = path.join(root, 'target', 'release', 'repotower-launcher.exe');
await fs.access(stub).catch(() => {
  throw new Error('Build the workspace first: node scripts/build-native.mjs');
});
await fs.mkdir(artifacts, { recursive: true });
await fs.mkdir(temp, { recursive: true });
const payload = path.join(temp, 'single-payload.zip');
await zipDirectory(packageRoot, payload);

// Add ordinary Windows version/icon resources before attaching the archive.
const executable = NtExecutable.from(await fs.readFile(stub));
const resources = NtExecutableResource.from(executable);
const icon = Data.IconFile.from(await fs.readFile(path.join(root, 'desktop/assets/repotower.ico')));
Resource.IconGroupEntry.replaceIconsForResource(
  resources.entries,
  1,
  1033,
  icon.icons.map((entry) => entry.data),
);
const version = Resource.VersionInfo.createEmpty();
version.setFileVersion(`${metadata.version}.0`, 1033);
version.setProductVersion(`${metadata.version}.0`, 1033);
version.setStringValues(
  { lang: 1033, codepage: 1200 },
  {
    CompanyName: 'Cabal',
    ProductName: 'RepoTower',
    FileDescription: 'RepoTower portable launcher',
    OriginalFilename: `${artifactBase}.exe`,
    LegalCopyright: 'Copyright (c) 2026 Cabal and RepoTower contributors',
  },
);
version.outputToResourceEntries(resources.entries);
resources.outputResource(executable);
const bytes = Buffer.from(executable.generate());
const footer = Buffer.alloc(64);
footer.write('RTOWER01', 0, 'ascii');
footer.writeBigUInt64LE(BigInt(bytes.length), 8);
footer.writeBigUInt64LE(BigInt((await fs.stat(payload)).size), 16);
Buffer.from(await digest(payload), 'hex').copy(footer, 24);
const output = path.join(artifacts, `${artifactBase}.exe`);
await fs.writeFile(output, bytes);
await pipeline(createReadStream(payload), createWriteStream(output, { flags: 'a' }));
await fs.appendFile(output, footer);
await checksum(output);
