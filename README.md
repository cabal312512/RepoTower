<p align="center"><img src="desktop/assets/repotower.svg" width="64" alt="RepoTower"></p>

# RepoTower

A small, offline desktop tool for exploring source dependencies. Disconnect a file and trace its potential impact through an interactive 2D graph.

[![Build](https://github.com/cabal312512/RepoTower/actions/workflows/build.yml/badge.svg)](https://github.com/cabal312512/RepoTower/actions/workflows/build.yml)
[![Release](https://img.shields.io/github/v/release/cabal312512/RepoTower)](https://github.com/cabal312512/RepoTower/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

[Download](https://github.com/cabal312512/RepoTower/releases/latest) · [简体中文](docs/README.zh-CN.md) · [Language support](docs/LANGUAGES.md) · [Contributing](CONTRIBUTING.md)

![RepoTower showing a dependency graph in light mode](docs/images/release-light.png)

## Download

Get the assets from the [latest release](https://github.com/cabal312512/RepoTower/releases/latest):

| Platform                | Download                                   | Start                                              |
| ----------------------- | ------------------------------------------ | -------------------------------------------------- |
| Windows x64             | `RepoTower-0.5.0-windows-x64-portable.zip` | Extract the complete folder; open `RepoTower.exe`. |
| Windows x64, single EXE | `RepoTower-0.5.0-windows-x64.exe`          | Put it in a writable folder and open it.           |
| macOS Apple Silicon     | `RepoTower-0.5.0-macos-arm64.zip`          | Extract; open `RepoTower.app`.                     |
| macOS Intel             | `RepoTower-0.5.0-macos-x64.zip`            | Extract; open `RepoTower.app`.                     |
| Linux x64               | `RepoTower-0.5.0-linux-x64.tar.gz`         | Extract; run `./RepoTower`.                        |

No language SDK, Node.js or Rust installation is required to use the app. The single EXE unpacks its application cache and keeps settings in **`RepoTower-data` beside the EXE**; it is a single download, not an app that leaves no files. Folder editions keep settings in adjacent `runtime-data`. Use a writable location.

Releases include SHA-256 checksums and source archives. The binaries are unsigned; macOS builds are not notarized. Linux needs the normal desktop libraries used by Electron. See [distribution notes](docs/DISTRIBUTION.md).

## Explore a project

1. Open a project folder, drop one onto the window, or choose **Try demo**.
2. Select a file to see its dependencies and consumers. Double-click it for **Nearby**.
3. Choose **Disconnect file**. The impact view follows actual dependency edges, one wave at a time.
4. Pause, step, replay, or select any disconnected file to restore it. **Restore all** clears the simulation.

**Source files are only read. Disconnecting a file never edits, deletes or executes it.**

- **Java, Python, C, C++, Go, Rust, C#, JavaScript and TypeScript**, with built-in parsers and examples.
- **All**, **Nearby** and **Impact** views; search, cycle detection and unresolved-reference details.
- Drag nodes with the right mouse button in All. Connections follow; reset one position or every position.
- Independent cameras for each view; selecting a file preserves your zoom.
- Multiple disconnections, individual restoration, undo and persistent impact history.
- Compact 980 × 680 window. English and light mode by default; Chinese, Japanese and dark mode available.
- Offline operation, no account, telemetry, repository uploads or auto-updater.

![Tracing the effect of disconnecting a configuration file](docs/images/release-impact.png)

Arrows point from a **dependency to a file that uses it**. Impact means potential reachability through static dependencies, not a prediction that every consumer will fail at runtime. Type-only imports count. Unresolved imports remain visible instead of producing guessed edges. See [supported constructs and limitations](docs/LANGUAGES.md).

The built-in demo has 23 files and 24 import edges. Disconnecting `src/core/config.ts` affects 13 other files; 9 remain available. [Demo explanation](fixtures/demo-project/README.md).

| Action               | Control                            |
| -------------------- | ---------------------------------- |
| Select / open Nearby | Click / double-click a node        |
| Pan / zoom           | Drag empty space / mouse wheel     |
| Move a node in All   | Right mouse drag                   |
| Reset positions      | Position menu or node context menu |
| Fit / deselect       | `R` / `Esc`                        |
| Undo                 | `Ctrl+Z` / `Cmd+Z`                 |
| Help                 | `?` button                         |

## Build from source

Use **Node.js 24+**, **Rust 1.90+**, and a native C toolchain. Build on the operating system and architecture you want to distribute. Initial dependency downloads require internet access; the finished app works offline.

```sh
npm ci
node scripts/build-native.mjs
node scripts/distribute.mjs
```

On Windows, `scripts/build.ps1` can bootstrap a project-local GNU/Rust toolchain. See [portable toolchain setup](docs/PORTABLE-TOOLCHAIN.md). Once it is configured:

```powershell
. ./scripts/rust-env.ps1
node scripts/build-native.mjs
node scripts/distribute.mjs
node scripts/build-single.mjs
```

Packages and checksums are written to `release/artifacts/`. Build tools, caches, temporary files and runtime data are excluded from Git. Dependencies are pinned in the npm and Cargo lockfiles; the source archives do not bundle those dependencies.

[Architecture](docs/ARCHITECTURE.md) · [Validation](docs/VALIDATION.md) · [Changelog](CHANGELOG.md) · [Security](SECURITY.md)

## License

[MIT](LICENSE). Copyright © 2026 Cabal and RepoTower contributors. Bundled dependencies retain their own licenses in `THIRD_PARTY_LICENSES.txt` and the Electron distribution notices.
