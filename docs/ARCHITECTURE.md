# Architecture

RepoTower 0.6 is a Rust application. It has no browser, webview, JavaScript renderer, local web server or analysis subprocess.

- `repotower-core` reads source files, parses built-in Tree-sitter grammars, resolves supported local references, computes metrics and performs reverse dependency traversal.
- `repotower-desktop` links that library directly. egui paints a custom compact window; wgpu uses Direct3D 12 on Windows, Metal on macOS and Vulkan/OpenGL ES on Linux. winit handles native windows and input. rfd provides native folder selection.
- Source scanning runs on a worker thread. Progress and results carry a generation number; stale results cannot replace a newly opened project.
- `graph.rs` deterministically groups files by dependency depth, immediate neighborhood or breadth-first impact wave. Four barycenter sweeps reduce crossings. Every rendered edge comes from the analyzer and points from dependency to consumer.
- `state.rs` keeps disconnected origins distinct from all unavailable files. Restoring an origin recomputes every remaining cut, preserving shared failures. Snapshots support undo. Completed reports remain available for replay.
- `canvas.rs` renders nodes, arrows, causal pulses and the timeline. Cameras are stored per view/report; selection does not refit them. Manual positions only affect All. Dragged nodes retain their drawing and hit-test order.
- `ui.rs`, `menus.rs` and `text.rs` implement the compact chrome, inspector and English/Chinese/Japanese strings.
- `runtime.rs` keeps application-managed files beside the binary, embeds compressed Noto CJK fonts and extracts only the small bundled examples when requested. The Windows binary itself requires no extraction.

Release builds embed fonts, examples, icons, grammars and license texts. `scripts/prepare-native-assets.mjs` downloads a versioned Noto font into `.cache/native-assets`, verifies SHA-256 and compresses it. There is no runtime font request. Asset and dependency downloads are build-time operations only.

The optional `--automation-stdio` flag accepts local JSON test input through inherited stdin. It injects native pointer/keyboard/drop events into the same egui input loop and returns state snapshots or GPU screenshots. It is disabled in normal launches, exposes no network port and does not execute commands or project code.

## Bounds and semantics

Analysis remains static. It does not run a compiler, package manager, project entry point, macro or build script. Resource limits, ignored directories, symlink handling and unresolved-reference diagnostics are implemented by the core. See [LANGUAGES.md](LANGUAGES.md).

All-view rendering is capped at 250 nodes; search retains access to the complete analyzed module list. Selected files outside the initial cap are included when rebuilding the view. Layout does not invent edges or treat geometric proximity as a dependency.

A pulse traverses an edge only when the consumer is exactly one breadth-first wave after the dependency. Other actual edges remain visible as context. Disconnecting is a simulation; no source file is rewritten, renamed or deleted.
