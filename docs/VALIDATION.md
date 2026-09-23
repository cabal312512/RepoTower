# Native validation

RepoTower 0.6.0 replaces the Electron UI with a Rust egui/wgpu application. Checks below run against the current native code and binary; historical 0.5.0 results remain in that Git tag.

## Analyzer and native state

`cargo test --workspace --locked` covers 61 existing analyzer tests and 8 native desktop tests. Coverage includes exact dependency edges, JS/TS import forms, Java/C# type references, Python packages and exports, C/C++ include search rules, Go package symbols, Rust module/use paths, mixed-language isolation, malformed sources, ignored files, bounded scans, cycles and a 1,200-file chain.

Native tests check deterministic layout, real edge direction, exact impact waves, zoom anchoring, shared failures after selective restoration, undo, restoring a live cut and restoring an earlier cut during a later animation. Stale progress/results cannot overwrite a new or closed project.

The demo remains 23 files and 24 dependency edges. Cutting `src/core/config.ts` produces waves of `1, 2, 4, 3, 2, 1, 1`, including the root: 13 affected files and 9 unaffected files.

## Real native-window tests

`node scripts/test-native.mjs --packaged` starts the actual packaged executable and injects pointer, keyboard and drop events into its input loop. It checks:

- English/light defaults and persistent English/Chinese/Japanese plus light/dark settings.
- Embedded example loading, correct nodes/edges and all seven additional language samples.
- Zoom preservation, delayed hover preview and double-click Nearby.
- Right-button node dragging, matching connections, overlap/hit order and selected/all position reset.
- Dependency-wave animation, pause, step, scrub, completion, replay and history navigation.
- Independent restoration, shared failures, undo, restore all and restoration during another active cut.
- Search, relation navigation, Enter-to-select, files outside the 250-node overview and empty projects.
- Minimum 760 × 520 layout, separate translated help and source-file hash preservation.

The driver captures actual GPU frames for both appearances, the impact view, Chinese/Japanese and the minimum-size window. Local Windows captures are visually inspected for readable text, matching graph layout, unclipped controls and impact visibility.

`node scripts/test-native-single.mjs` checks that the Windows standalone download is byte-identical to the tested packaged binary. It then copies only that EXE into a clean folder and verifies startup, the embedded demo, adjacent data, no old payload extraction and the executable size budget.

## Reproduce

```powershell
. ./scripts/rust-env.ps1
node scripts/build-native.mjs
cargo fmt --all -- --check
node scripts/test-native.mjs --packaged
node scripts/distribute.mjs
node scripts/build-single.mjs
node scripts/test-native-single.mjs
```

On headless Linux use `xvfb-run -a` for GUI tests. Reports, failure state and screenshots are saved under ignored `test-results/`. Runtime test profiles and generated large fixtures stay there as well.

## Scope

Local checks use Windows x64 with the project-local Rust 1.90 GNU toolchain. Hosted Windows x64, macOS arm64/x64 and Linux x64 results are recorded by the [build workflow](https://github.com/cabal312512/RepoTower/actions/workflows/build.yml). The publisher requires every platform job to pass before uploading a public release.

These checks do not establish every possible compiler resolution rule, GPU driver, OS version or project shape. No frame-rate, memory or startup-time benchmark is claimed. Static impact is potential dependency impact, not a prediction that a program must crash.
