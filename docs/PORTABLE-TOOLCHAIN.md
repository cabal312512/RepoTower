# Project-local Windows toolchain

Windows can build with the portable Rust GNU toolchain in this repository's ignored directories. Setup does not install Rustup, Visual Studio, services, registry entries or a permanent PATH. Initial downloads need a network connection; the finished application does not.

## Tools

- Rust compiler, Cargo, rustfmt, standard library and MinGW linker support: 1.90.0, `x86_64-pc-windows-gnu`, from the official Rust distribution. SHA-256 sidecars are verified.
- Tree-sitter's C compiler: w64devkit 2.10.0 from its [upstream release](https://github.com/skeeto/w64devkit/releases/tag/v2.10.0), verified against its published digest.
- Node.js 24+ is a build/test helper. Use an existing installation, or extract the official portable archive into `.tools/node`.
- The CJK font download is pinned by release and SHA-256 in `assets/fonts/manifest.json`.

## Build

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build.ps1
```

For direct Cargo commands, first apply the process-local environment:

```powershell
. ./scripts/rust-env.ps1
node scripts/prepare-native-assets.mjs
cargo test --workspace --locked
cargo build -p repotower-desktop --release --locked
```

The native application is `target/release/RepoTower.exe`. Package it with `node scripts/package-native.mjs`; create downloads with `node scripts/distribute.mjs` and `node scripts/build-single.mjs`.

`rust-env.ps1` sets CARGO_HOME, RUSTUP_HOME, TEMP/TMP, the target directory, compiler/linker paths and the current process PATH. Closing that shell discards the settings. Run Cargo through this environment to keep caches local.

## Local directories

| Directory | Contents |
| --- | --- |
| `.tools/rust`, `.tools/w64devkit` | Portable build tools |
| `.cache/downloads` | Toolchain downloads and checksums |
| `.cache/cargo`, `.cache/npm` | Dependency caches |
| `.cache/native-assets` | Verified embedded fonts and compressed copies |
| `.tmp` | Build temporary files |
| `target` | Rust build outputs |
| `release` | Packaged applications and archives |
| `test-results` | Native interaction reports, screenshots and isolated test profiles |

These directories are excluded from Git. Application-managed runtime data is adjacent to whichever native executable is launched. No Node, Rust, browser engine or language SDK is required on a recipient's machine. Windows GNU builds statically link their matching Rust/MinGW support libraries; CI's Windows MSVC build requests a static C runtime.

macOS and Linux compile the same Rust source on their respective native build runners. Cross-platform release validation is recorded in GitHub Actions.
