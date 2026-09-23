# Contributing

Use Node.js 24+, Rust 1.90+ and a native C compiler. Initial dependency downloads need an internet connection. Windows users can use the project-local setup described in [PORTABLE-TOOLCHAIN.md](docs/PORTABLE-TOOLCHAIN.md).

```sh
npm ci
node scripts/build-native.mjs
cargo fmt --all -- --check
cargo test --workspace --locked
npm test
node scripts/test-languages.mjs --packaged
node scripts/distribute.mjs
```

The native build runs Rust and frontend tests before packaging. On headless Linux, run desktop checks with `xvfb-run -a`. Windows also supports `node scripts/test-desktop.mjs --packaged`, `node scripts/test-interactions.mjs --packaged`, and, after `node scripts/build-single.mjs`, `node scripts/test-single.mjs`.

Keep changes focused. For dependency resolution, add a small source fixture and assert the exact edges, including ambiguous or unresolved references. For interaction changes, test the user-visible result. Update all three UI languages and the relevant documentation. Do not execute analyzed projects, upload repository contents or add network requests to the desktop app.

Use a `codex/` or descriptive feature branch and open a pull request against `main`. Include the behavior changed and the checks you ran. Avoid private project paths, secrets and large generated assets in reports or commits. Code submitted to this repository is covered by its MIT license.

## Releases

The build workflow validates native Windows x64, macOS arm64/x64 and Linux x64 packages. A successful push to `main` publishes a release **only when the version in `package.json` has no existing release**. Pull requests only build and test. Existing public releases and assets are never overwritten.

For a new version, update `package.json`, both lockfiles, the Cargo package versions, `RepoTower.cmd`, the title-bar version and download examples. Add `docs/releases/vVERSION.md`, update the changelog and validation notes, then review the complete change before merging. The release job collects the native archives, Windows single EXE and checksums, creates a draft, uploads assets, and publishes it when all uploads succeed. A failed draft may be retried for the same commit.

Native binaries are currently unsigned. Do not describe CI completion as code signing, notarization, or evidence of runtime behavior on untested OS versions.
