# RepoTower 0.5 verification record

Date: 2026-09-23. Platform exercised: Windows x64, project-local Rust 1.90 GNU / w64devkit, Node and Electron. No macOS/Linux runtime validation is claimed.

This record covers the public-release preparation, interaction behavior and the Java, Python, C, C++, Go, Rust, C# and JS/TS analyzers. Hosted native results are recorded by the [build workflow](https://github.com/cabal312512/RepoTower/actions/workflows/build.yml); this document distinguishes local checks from hosted runs.

## Completed checks

- `cargo test --workspace --locked`: **64 Rust tests passed**: 61 analyzer tests (6 library, 11 original analysis integration, 1 CLI integration, 5 Java, 11 Python, 10 C/C++, 4 Go, 5 Rust, 5 C# and 3 mixed-language tests), plus 3 launcher tests for bounded payload reads, integrity, cache repair and safe extraction paths.
- The Rust suite covers import extraction, relative resolution, graph direction, SCC/cycles/self-loops, minimum reverse-distance waves, unavailable-node exclusion, scanner ignore rules, malformed-source partial results, deterministic analysis and a 1,200-module deep chain.
- Language regressions cover actual type references rather than namespace-wide edges, duplicate type diagnostics, C# partial declarations, Python package initialization and exports, namespace packages, local Go packages, Rust module/use/scoped paths and mixed-language isolation.
- C/C++ tests reject same-name/suffix guesses, including a project `mock/stdio.h` substituting for a system header. They check explicit relative includes, compiler include paths and their order, conflicting compile configurations, external-path shadowing, ignored headers and conservative handling of headers without their own compilation profile. Compilation commands are parsed as data and never executed.
- Mixed-language tests retain monorepo `packages` directories, enforce bounded configuration reads and keep JS extension fallback confined to JS/TS files.
- The curated demo is verified as **23 modules, 24 edges, 0 unresolved imports, 0 external imports and 1 cycle**. Every module's blast metric matches a BFS impact query.
- Disconnecting `src/core/config.ts` has exact waves of `1, 2, 4, 3, 2, 1, 1` including the target. Tests verify 13 affected dependents, 9 unaffected files and at least one real preceding-wave import for every affected node.
- The isolated registry/hooks cycle visits each file once and does not touch the workspace branch.

## Frontend and desktop results

`npm test`: **55 tests passed** across 6 files: 25 graph layout/playback tests, 11 simulation-history tests, 8 diagnostic translation tests, 6 UI tests and 5 saved-preference tests. TypeScript checking, the production Vite build and the Rust release build passed. Fresh and invalid profiles use English/light mode; existing settings are preserved.

Graph tests cover exact dependency direction, converging and cyclic paths, arrival-gated node states, pause/step/replay, retaining positions in the all view, switching views without resetting playback, external inspection and completing a paused simulation from the bottom dock. State tests cover consecutive operations, exact undo, unavailable-file rejection and duplicate completion. UI tests reject late results after closing a project, preserve pending operations through appearance changes and expose unresolved imports for the selected file. Diagnostic tests cover Chinese, English and Japanese, including the new language-specific messages.

New regressions cover right-button node movement with attached wires, independent single/all position resets, camera preservation on selection and view round trips, explicit double-click neighborhoods, cancellable 350 ms hover previews and dismissible help. Restoration tests keep shared failures, restore complete undo snapshots, reject late calculations after closing the repository and restore earlier cuts while a later cut is still playing.

`node scripts/test-desktop.mjs`: **12 desktop checks passed**, using the real Electron host and Rust process:

- Default 980 × 680 window; native maximize/restore/minimize; 760 × 520 minimum with no document overflow.
- Demo scan produces 23 modules and 24 SVG edges, each matched to an actual analyzer edge. No WebGL canvas remains.
- Each of the 7 explicit steps is checked node by node against Rust waves. The result has 13 affected files and 9 untouched files; overview shows 1 disconnected, 13 affected and 9 normal nodes.
- Replay and undo preserve simulation history. Disconnecting config followed by palette excludes previously unavailable files, leaves 5 untouched files and restores exactly through undo/reset.
- A generated 300-file repository renders the explicit 250-node limit; search reaches a file omitted from that overview.
- Folder-picker IPC handles empty and invalid directories, then successfully reopens the demo.
- Chinese/English/Japanese and dark/light switches preserve selection. Settings survive a full reload.
- Renderer sandbox and context isolation stay enabled, Node is unavailable to page code, and writable application paths stay within the project.
- CSP blocks an external fetch before an HTTP request leaves the renderer. No external requests or renderer errors occurred.
- SHA-256 of all demonstration source files is unchanged after analysis, simulation, replay, undo and reset.

`node scripts/test-languages.mjs`: **7 language desktop checks passed**. Each fixture is opened through the folder-picker IPC and scanned by the real Rust analyzer. Java, Python, C, C++, Go and C# each produce 3 files and 2 exact dependency edges; Rust produces 3 files and 3 edges because module declarations also create dependencies. Every case checks the displayed language, matches every SVG edge to a backend edge, disconnects the configuration file, verifies exactly 2 affected files and then undoes the operation. All sample source hashes remain unchanged. No external HTTP requests or renderer errors occurred.

`node scripts/test-interactions.mjs`: **5 interaction scenarios passed** in real Electron. They check delayed hover, unchanged camera transforms, exact arrow attachment after right dragging, independent position resets, double-click and view round trips, persistent playback and report switching, individual restoration against fresh Rust results, restoration during a paused cut, and help at the minimum window in all three languages and both themes. Source hashes remain unchanged and no renderer errors or external HTTP requests occur.

All three packaged verification scripts passed against `RepoTower-0.5.0-win32-x64/RepoTower.exe`: **12 general checks, 5 interaction scenarios and 7 language checks**, with unchanged source hashes, no renderer errors and no external HTTP requests. Each desktop script accepts `--packaged`. Reports are `test-results/desktop[-packaged].json`, `test-results/interactions[-packaged].json` and `test-results/languages[-packaged].json`.

`node scripts/test-release.mjs --screenshots` additionally verified the actual packaged app with a fresh profile: English/light defaults, all six Electron writable paths beside the app, the 23-file/24-edge demo, and the 13-affected/9-unaffected result. It captured the English release screenshots without renderer errors or HTTP requests.

`node scripts/test-single.mjs` passed against the real Windows single EXE: first extraction, reuse without rewriting intact files, repair after deliberate app-cache corruption, preservation of adjacent settings, GUI launch through the wrapper, Rust-backed demo analysis, and language persistence after a complete close/reopen.

Reproducible commands, from the project root:

```powershell
. ./scripts/rust-env.ps1
cargo test --workspace --locked --offline
cargo fmt --all -- --check
npm.cmd test
npm.cmd run build
npm.cmd run test:desktop
npm.cmd run test:languages
npm.cmd run test:interactions
node scripts/test-desktop.mjs --packaged
node scripts/test-languages.mjs --packaged
node scripts/test-interactions.mjs --packaged
node scripts/test-release.mjs --screenshots
node scripts/build-single.mjs
node scripts/test-single.mjs
```

Integration results and failure screenshots belong in ignored `test-results/`. Do not infer a passing result from an existing report generated by another version.

## Visual verification

Screenshots from the running 0.5 application include both appearances, the dependency overview, completed impact view and a Java impact chain. Desktop automation checks the help panel at the minimum window size in all languages. The English release overview and impact captures were visually inspected. Node labels, report colors and controls fit the compact shell; longer filenames expose their complete path on inspection.

Current screenshots:

- `docs/images/circuit-dark.png`
- `docs/images/circuit-light.png`
- `docs/images/circuit-impact.png`
- `docs/images/java-impact.png`

The English README uses `release-light.png` and `release-impact.png`, generated by `node scripts/test-release.mjs --screenshots`. `node scripts/capture-demo.mjs` regenerates the other captures through the verified desktop flow.

## Performance scope

No new renderer performance numbers are claimed yet. Generated chain and broad fixtures remain available for repeatable analysis and display checks. Report fixture shape, module/edge count, build type and measured operation with any future timing. A browser animation-frame interval alone does not establish GPU presentation rate or performance on another computer.

## Distribution boundary

The Windows distribution is a portable folder or self-extracting single EXE. Both require a writable location for adjacent data; see [DISTRIBUTION.md](DISTRIBUTION.md). The recipient needs no Rust, Node, Python, JDK, Go or .NET SDK installation. All language grammars are compiled into the analyzer; analysis does not execute project code or build scripts. Development toolchains, npm/Cargo caches and temporary files stay in ignored project directories. The Electron host accounts for most package size.

The portable resources include the original JS/TS demonstration and seven independent language samples. See [LANGUAGES.md](LANGUAGES.md) for the static-analysis boundaries; passing these fixtures does not imply complete compiler or runtime resolution.

The package is unsigned. macOS/Linux must build and validate their matching native host and analyzer. GitHub Actions configuration does not establish a completed hosted run, upload or release. Operating-system bookkeeping remains outside application control.
