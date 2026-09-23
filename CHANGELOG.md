# Changelog

## 0.6.0

- Replaced Electron/React with a native Rust egui/wgpu desktop application.
- Linked the existing multi-language analyzer directly into the application.
- Preserved graph layout, causal animation, camera behavior, drag/reset, cut history and individual restoration.
- Embedded fonts, sample projects, grammar parsers and license texts; no runtime downloads.
- Replaced the self-extracting Windows launcher with the actual standalone application binary.
- Added native GUI interaction checks and screenshots on the release matrix.
- Retained portable data, English/light defaults, Chinese/Japanese and dark mode.
## 0.5.0

First public GitHub release.

- English and light mode for new profiles; existing appearance settings are preserved.
- Native portable archives for Windows x64, macOS arm64/x64 and Linux x64.
- Windows single-download EXE with adjacent extraction cache, integrity checks and local settings.
- MIT license, English and Chinese documentation, source archives and SHA-256 checksums.
- Automated native tests, packaging and versioned GitHub releases.
- Linux deep-directory startup support without relocating application data.

## 0.4.0

- Right mouse dragging in All, connected arrow movement, individual/all position reset.
- Persistent per-view camera, delayed relationship previews and double-click Nearby.
- Individual restoration of disconnected files, persistent impact playback and history.
- Compact help popover and interaction regression checks.

## 0.3.0

- Java, Python, C, C++, Go, Rust and C# analysis with built-in grammars and examples.
- Resolution diagnostics and exact-edge regression fixtures.

## 0.2.0

- Compact 2D graph with causal impact waves, playback and view controls.
- Light/dark appearances and English, Chinese and Japanese UI.

## 0.1.0

- Initial offline dependency-analysis prototype for JavaScript and TypeScript.
