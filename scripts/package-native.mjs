import fs from 'node:fs/promises';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { root, metadata, packageRoot, assertInsideProject } from './release-utils.mjs';

assertInsideProject(packageRoot);
await fs.mkdir(packageRoot, { recursive: true });
const binary = path.join(root, 'target', 'release', process.platform === 'win32' ? 'RepoTower.exe' : 'RepoTower');
await fs.access(binary);
let destination;
if (process.platform === 'darwin') {
  const app = path.join(packageRoot, 'RepoTower.app', 'Contents');
  await fs.mkdir(path.join(app, 'MacOS'), { recursive: true });
  await fs.mkdir(path.join(app, 'Resources'), { recursive: true });
  destination = path.join(app, 'MacOS', 'RepoTower');
  await fs.copyFile(path.join(root, 'desktop/assets/repotower.icns'), path.join(app, 'Resources', 'RepoTower.icns'));
  await fs.writeFile(path.join(app, 'Info.plist'), `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>RepoTower</string>
<key>CFBundleDisplayName</key><string>RepoTower</string>
<key>CFBundleIdentifier</key><string>io.github.cabal312512.RepoTower</string>
<key>CFBundleExecutable</key><string>RepoTower</string>
<key>CFBundleIconFile</key><string>RepoTower.icns</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>${metadata.version}</string>
<key>CFBundleVersion</key><string>${metadata.version}</string>
<key>LSMinimumSystemVersion</key><string>11.0</string>
<key>NSHighResolutionCapable</key><true/>
<key>NSSupportsAutomaticGraphicsSwitching</key><true/>
</dict></plist>\n`);
} else destination = path.join(packageRoot, process.platform === 'win32' ? 'RepoTower.exe' : 'RepoTower');
await fs.copyFile(binary, destination);
if (process.platform === 'win32') {
  const { Data, NtExecutable, NtExecutableResource, Resource } = await import('resedit');
  const exe = NtExecutable.from(await fs.readFile(destination));
  const resources = NtExecutableResource.from(exe);
  const icon = Data.IconFile.from(await fs.readFile(path.join(root, 'desktop/assets/repotower.ico')));
  Resource.IconGroupEntry.replaceIconsForResource(resources.entries, 1, 1033, icon.icons.map(e => e.data));
  const version = Resource.VersionInfo.createEmpty();
  version.setFileVersion(`${metadata.version}.0`,1033); version.setProductVersion(`${metadata.version}.0`,1033);
  version.setStringValues({lang:1033,codepage:1200},{CompanyName:'Cabal',ProductName:'RepoTower',FileDescription:'RepoTower native dependency explorer',OriginalFilename:'RepoTower.exe',LegalCopyright:'Copyright (c) 2026 Cabal and RepoTower contributors'});
  version.outputToResourceEntries(resources.entries); resources.outputResource(exe);
  await fs.writeFile(destination,Buffer.from(exe.generate()));
} else await fs.chmod(destination, 0o755);
for (const file of ['LICENSE','README.md','THIRD_PARTY_NOTICES.md','THIRD_PARTY_LICENSES.txt']) await fs.copyFile(path.join(root,file),path.join(packageRoot,file));
await fs.cp(path.join(root,'docs'),path.join(packageRoot,'docs'),{recursive:true});
await fs.mkdir(path.join(packageRoot,'desktop','assets'),{recursive:true});
await fs.copyFile(path.join(root,'desktop','assets','repotower.svg'),path.join(packageRoot,'desktop','assets','repotower.svg'));
await fs.copyFile(path.join(root,'assets/fonts/OFL.txt'),path.join(packageRoot,'LICENSE-Noto.txt'));
if (process.platform === 'darwin') {
  const result = spawnSync('codesign',['--force','--deep','--sign','-',path.join(packageRoot,'RepoTower.app')],{stdio:'inherit'});
  if (result.status!==0) throw new Error('Ad-hoc macOS signing failed');
}
console.log(`Native application: ${destination} (${(await fs.stat(destination)).size.toLocaleString()} bytes)`);
