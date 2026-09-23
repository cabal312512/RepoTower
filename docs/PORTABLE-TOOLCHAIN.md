# Project-local Windows toolchain

The Windows build uses a portable Rust GNU toolchain. It does not install Rustup,
Visual Studio, system packages, services, registry entries or a persistent PATH.
Development downloads require a network connection; the finished application does
not. Deleting this project directory also removes these development dependencies.

## Versions and sources

* Rust compiler, Cargo, rustfmt, standard library and bundled MinGW linker support: **1.90.0**,
  `x86_64-pc-windows-gnu`, from `https://static.rust-lang.org/dist/`.
* C compiler for Tree-sitter parsers: **w64devkit 2.10.0 / GCC 16.2.0**, from the
  [upstream release](https://github.com/skeeto/w64devkit/releases/tag/v2.10.0).
* Rust archives are checked against the official SHA-256 sidecars. The w64devkit
  archive is checked against the SHA-256 digest published in its GitHub release.
* Rust components are extracted directly, without running an installer. The
  w64devkit release is a 7-Zip self-extractor; its explicit destination is `.tools`.

## Commands

From the repository root in PowerShell:

```powershell
# First build only: download and extract the pinned portable dependencies.
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/setup-rust.ps1

# Apply process-local settings. Closing this shell discards them.
. .\scripts\rust-env.ps1
cargo test --manifest-path crates/repotower-core/Cargo.toml
cargo build --release --manifest-path crates/repotower-core/Cargo.toml
```

The release binary is `target/release/repotower-core.exe`. Do not run Cargo from a
fresh terminal before dot-sourcing the environment if you want all caches to stay
inside the project. `rust-env.ps1` sets `CARGO_HOME`, `RUSTUP_HOME`, `TEMP`, `TMP`,
the compiler/linker variables, target directory and current-process PATH. It never
changes user or machine environment settings.

Rust links its matching bundled MinGW support libraries through
`-C link-self-contained=yes`. w64devkit builds the C parser objects. This avoids a
libgcc naming mismatch between Rust's standard library and recent GCC releases.

## Local directories

| Directory | Contents |
| --- | --- |
| `.tools/rust` | Portable Rust executables and standard library |
| `.tools/w64devkit` | Portable C compiler and its support files |
| `.cache/downloads` | Pinned toolchain archives and checksums |
| `.cache/cargo` | Rust crate index, archives and source cache |
| `.cache/npm` | JavaScript package cache |
| `.tmp` | Build and extraction temporary files |
| `target` | Rust build output |
| `runtime-data` | Local desktop settings, Chromium cache and temporary files |

These directories are excluded from Git. The portable release needs neither Rust
nor Node installed on the recipient's machine. The desktop host sets its writable
Electron directories before browser initialization. It also denies renderer
network requests and permission prompts, and runs the analysis executable through
stdin/stdout without opening a local HTTP port.

Other platforms can compile the same Rust source with a native Rust toolchain and
package the Electron host natively. Windows GNU binaries cannot be reused as
macOS or Linux sidecars; each release must include the matching native executable.
