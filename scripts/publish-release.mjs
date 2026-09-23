// Runs only in the release job after all native build/test jobs have succeeded.
import fs from 'node:fs/promises';
import path from 'node:path';
import { metadata, root, digest } from './release-utils.mjs';

const repository = process.env.GITHUB_REPOSITORY;
const commit = process.env.GITHUB_SHA;
const token = process.env.GITHUB_TOKEN;
if (repository !== 'cabal312512/RepoTower' || !token || !/^[0-9a-f]{40}$/.test(commit ?? ''))
  throw new Error('This publisher runs only in the authorized repository workflow.');
if (!/^\d+\.\d+\.\d+$/.test(metadata.version)) throw new Error('Use a stable semantic version.');
const tag = `v${metadata.version}`;
if (
  process.env.GITHUB_REF?.startsWith('refs/tags/') &&
  process.env.GITHUB_REF !== `refs/tags/${tag}`
)
  throw new Error('Tag and package version differ.');
const base = `https://api.github.com/repos/${repository}`;
async function api(url, options = {}) {
  const response = await fetch(url, {
    ...options,
    headers: {
      Authorization: `Bearer ${token}`,
      Accept: 'application/vnd.github+json',
      'X-GitHub-Api-Version': '2022-11-28',
      'Content-Type': 'application/json',
      ...options.headers,
    },
  });
  if (response.status === 404 && options.method === undefined) return null;
  if (!response.ok) throw new Error(`GitHub API ${response.status}: ${await response.text()}`);
  return response.json();
}
let release = await api(`${base}/releases/tags/${tag}`);
if (release && !release.draft) {
  console.log(`${tag} is already published; no release files were changed.`);
  process.exit(0);
}
if (release && release.target_commitish !== commit)
  throw new Error('An unfinished draft belongs to another commit; inspect it before publishing.');
const directory = path.join(root, 'release', 'artifacts');
const names = [
  'windows-x64-portable.zip',
  'windows-x64.exe',
  'macos-arm64.zip',
  'macos-x64.zip',
  'linux-x64.tar.gz',
].map((suffix) => `RepoTower-${metadata.version}-${suffix}`);
const hashes = new Map();
for (const name of names) {
  const hash = await digest(path.join(directory, name));
  const expected = (await fs.readFile(path.join(directory, `${name}.sha256`), 'utf8')).trim();
  if (expected !== `${hash}  ${name}`) throw new Error(`Checksum mismatch: ${name}`);
  hashes.set(name, hash);
}
await fs.writeFile(
  path.join(directory, 'SHA256SUMS.txt'),
  names.map((name) => `${hashes.get(name)}  ${name}\n`).join(''),
);
hashes.set('SHA256SUMS.txt', await digest(path.join(directory, 'SHA256SUMS.txt')));
const body = await fs.readFile(path.join(root, 'docs', 'releases', `${tag}.md`), 'utf8');
release ??= await api(`${base}/releases`, {
  method: 'POST',
  body: JSON.stringify({
    tag_name: tag,
    target_commitish: commit,
    name: `RepoTower ${metadata.version}`,
    body,
    draft: true,
    prerelease: false,
  }),
});
const upload = `https://uploads.github.com/repos/${repository}/releases/${release.id}/assets`;
for (const name of [...names, 'SHA256SUMS.txt']) {
  const existing = release.assets?.find((asset) => asset.name === name);
  if (existing) {
    if (existing.digest !== `sha256:${hashes.get(name)}`)
      throw new Error(`Draft already contains a different ${name}; refusing to overwrite it.`);
    continue;
  }
  await api(`${upload}?name=${encodeURIComponent(name)}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/octet-stream' },
    body: await fs.readFile(path.join(directory, name)),
  });
  console.log(`Uploaded ${name}`);
}
await api(`${base}/releases/${release.id}`, {
  method: 'PATCH',
  body: JSON.stringify({ draft: false, make_latest: 'true' }),
});
console.log(`Published https://github.com/${repository}/releases/tag/${tag}`);
