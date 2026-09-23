# Distribution

Build with `node scripts/build-native.mjs`. This prepares verified embedded assets, tests the Rust workspace and packages the native binary. `node scripts/distribute.mjs` creates the platform archive and SHA-256 sidecar. On Windows, `node scripts/build-single.mjs` copies the exact same application binary into a standalone download; no launcher or payload is appended.

## Requirements

- Windows 10/11 x64, with a working Direct3D 12 adapter (hardware or supported software adapter). No WebView2 or VC runtime installer is required.
- macOS 11+ on Apple Silicon or Intel, with Metal support. CI verifies current macOS runners; older supported OS versions have not all been exercised.
- Linux x64 with glibc compatible with Ubuntu 22.04+, X11 or Wayland, a Vulkan or OpenGL ES 3-capable graphics stack, and xkbcommon. Native folder dialogs use the desktop's XDG portal service. Folder drag-and-drop and the `--project PATH` option can open projects independently of a portal dialog.

The app needs a writable containing folder for portable data. macOS `.app` bundles keep `runtime-data` beside the bundle. A read-only/translocated macOS bundle must be moved to a normal writable location before use. No automatic fallback writes to AppData or the user's home directory.

## Release files

- Windows x64 standalone `.exe`.
- Windows x64 `.zip`, including that EXE, documentation and notices.
- macOS Apple Silicon and Intel `.zip`, each containing a native `.app` and documentation.
- Linux x64 `.tar.gz`, with an executable and documentation.
- `SHA256SUMS.txt`, plus GitHub's automatic source ZIP and tar.gz archives.

Windows files are unsigned. macOS bundles receive an ad-hoc signature for native execution, but are neither Developer ID signed nor notarized. Operating-system download checks may therefore apply. Checksums are integrity checks, not signing certificates.

## Portable files

`runtime-data/preferences.json` stores appearance/language preferences. `runtime-data/examples/VERSION` contains embedded sample code only when a sample is opened. `runtime-data/temp` and `runtime-data/cache` are application-local temporary/cache locations. OS and GPU-driver bookkeeping is outside the app's control.

0.5.0's Electron launcher and adjacent `RepoTower-data` remain usable; 0.6.0 does not modify that release or migrate/delete its settings. The old release and Git tag remain available.

## Publishing

The workflow builds and tests Windows x64, macOS arm64/x64 and Linux x64 before creating a new versioned release. The publication job verifies all asset checksums, uploads a draft and publishes only after all uploads succeed. Existing public versions are never replaced. Pull requests build and test without publishing.
