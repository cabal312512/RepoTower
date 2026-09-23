# Contributing

Use Rust 1.90, a native C compiler and Node.js 24+. Node runs asset preparation, packaging and GUI tests only; the shipped app is Rust. Initial dependency downloads need a network connection. Windows users can use the [project-local toolchain](docs/PORTABLE-TOOLCHAIN.md).

```sh
npm ci
node scripts/build-native.mjs
cargo fmt --all -- --check
node scripts/test-native.mjs --packaged
node scripts/distribute.mjs
```

The build runs the Rust workspace tests before packaging. On headless Linux use `xvfb-run -a node scripts/test-native.mjs --packaged`. To create and verify the standalone Windows download, run `node scripts/build-single.mjs` followed by `node scripts/test-native-single.mjs`. For local UI work, `npm run dev` builds and opens a debug native window.

The GUI test driver injects pointer, keyboard and folder-drop input into the real native executable and captures GPU screenshots. It checks visible interaction results, simulation state and source-file hashes. Normal launches expose neither a test listener nor a network port.

For resolver changes, add small source fixtures and assert exact edges, including ambiguous or unresolved references. For interaction changes, test user-visible behavior. Update English, Chinese and Japanese resources together. Do not execute analyzed projects, upload repository contents or add runtime network requests.

Use a feature branch and open a pull request against `main`. Describe the behavior and relevant validation. Do not commit private paths, secrets, caches, build tools or generated release binaries. Contributions are MIT licensed.

## Releases

The workflow builds and tests native Windows x64, macOS arm64/x64 and Linux x64 packages. A successful push to `main` publishes a release only when that package version has no existing public release. Pull requests only build and test. Existing public releases are never overwritten.

For a new version, update `package.json`, both lockfiles, both Cargo package versions, `RepoTower.cmd`, the title-bar/about version and download examples. Add `docs/releases/vVERSION.md`, update the changelog and validation notes, and review the complete diff. The release job verifies assets, creates a draft, uploads files and publishes only after all uploads succeed.

Windows binaries currently lack a signing certificate. macOS bundles are ad-hoc signed, not notarized. CI validation must not be described as testing every OS/GPU combination.
