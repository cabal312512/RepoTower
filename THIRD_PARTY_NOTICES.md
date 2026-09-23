# Third-party software

RepoTower is MIT licensed. Dependencies and embedded fonts retain their own licenses. `Cargo.lock`, `package-lock.json` and `assets/fonts/manifest.json` identify exact inputs.

| Component | Upstream | License |
| --- | --- | --- |
| egui / eframe / epaint | https://github.com/emilk/egui | MIT / Apache-2.0 |
| wgpu | https://github.com/gfx-rs/wgpu | MIT / Apache-2.0 |
| winit | https://github.com/rust-windowing/winit | Apache-2.0 |
| rfd | https://github.com/PolyMeilex/rfd | MIT |
| Tree-sitter runtime and language grammars | https://github.com/tree-sitter | MIT |
| petgraph | https://github.com/petgraph/petgraph | MIT / Apache-2.0 |
| serde / serde_json | https://github.com/serde-rs | MIT / Apache-2.0 |
| ignore | https://github.com/BurntSushi/ripgrep | MIT / Unlicense |
| Noto Sans CJK (SC, 2.004) | https://github.com/notofonts/noto-cjk | SIL Open Font License 1.1 |
| egui default fonts | https://github.com/emilk/egui/tree/main/crates/epaint_default_fonts | Ubuntu Font License, SIL OFL and MIT / Bitstream notices; see full texts |

The native executable embeds `THIRD_PARTY_LICENSES.txt`, the application license and the Noto font license. They are available in Settings → Licenses even when only the Windows EXE is downloaded. Archives also include standalone notices. The Noto font is embedded unmodified, compressed for storage, and never downloaded at runtime.

The generated license collection includes locked Rust dependencies for all supported targets, bundled font notices, local Rust/MinGW support-library notices and Node build-tool notices. Inclusion does not imply every optional or development dependency is linked into every release binary. Node, Electron, Chromium, WebView2, language SDKs and development toolchains are not shipped with the app.

The synthetic demonstration projects in `fixtures` are part of RepoTower under the same MIT license.
