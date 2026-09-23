# RepoTower architecture

## Product

A read-only, offline dependency viewer for Java, Python, C/C++, Go, Rust, C# and JS/TS in a compact desktop window. The default window is 980 × 680, resizable down to 760 × 520. The interface has a project/search bar, an SVG dependency circuit and a bottom selection/result dock. Chinese, English and Japanese share light/dark themes; preferences stay local and do not reset analysis or simulation.

Selecting a file exposes its actual dependencies and consumers. Disconnecting it shows an inspectable causal trace, with distinct target, affected and pending states. Playback supports pause, steps, seeking, replay and immediate completion. The UI does not execute or edit repository files.

## Runtime boundary

Rust owns scanning, parsing, resolution, SCC condensation, dependency depth, reachability and impact metrics. React receives metadata, never source text. SVG owns presentation, selection, viewport and trace playback. Zustand stores selection, unavailable IDs and undo snapshots.

Electron hosts a sandboxed renderer with context isolation and no renderer Node access. Validated preload IPC mediates file selection, analysis and window controls. A persistent native Rust process uses JSON lines on stdin/stdout; no application HTTP server is started.

The portable Electron host and GNU Rust build keep dependencies under the project. Development tools, package caches and temporary files are local; runtime writable directories are redirected beside the executable before Chromium starts. The renderer CSP and Electron session enforce the runtime offline boundary. The larger Chromium distribution is an explicit portability tradeoff.

## Graph contract and arrow direction

IDs in `src/types/analysis.ts` are normalized repository-relative paths. Analyzer edges use **importer → dependency**: `A imports B` yields `source=A, target=B`. Fan-out counts distinct internal dependencies and fan-in counts distinct importers. External specifiers and unresolved imports are separate from internal edges.

The circuit deliberately displays **dependency → consumer**: the same edge is drawn from B to A. The question-mark guide explains this convention. Layout reverses existing analyzer edges for display; it never invents a link between adjacent nodes.

## Language analysis

The core first scans and reads bounded UTF-8 sources, establishing the complete set of eligible file IDs. A second pass builds language-specific indexes and resolves references only to that set. Each language analyzer returns resolved internal targets, external dependencies or explicit unresolved diagnostics through a shared contract. Tree-sitter grammars are compiled into the native sidecar; no language installation, compiler, build script or repository interpreter is invoked.

Java and C# index declared types and resolve syntactic type references. A namespace/package wildcard is a name-lookup rule, not an edge to every file. Python resolves modules and required package initializers, distinguishing exported attributes from fallback submodule imports. C/C++ resolves quoted includes relative to the current file, then follows explicit search paths from the root compile_commands.json when available. It recognizes -I, -iquote, -isystem and MSVC /I without executing commands; only scanned project files become targets. Missing search paths, conflicting configurations and possible external-file shadowing remain unresolved rather than falling back to basename or path-suffix matches. Go indexes local go.mod packages; a package import conservatively includes all its production files, while same-package lexical references can add file edges. Rust follows declared module trees before resolving use paths. JS/TS retains its relative module resolver.

Parser/index instances are reused during a scan. Small configuration reads are cached, capped at 1 MiB per file and confined by canonical path to the selected root. The shared graph deduplicates file pairs independently of how many imports or type references identify them. Source text and indexes stay in the native analysis process.

These are source-level dependency models, not compiler execution. Build variants, generated code, macro expansion, symbol inference and runtime import hooks can change real dependency sets. Unsupported or ambiguous explicit local references produce diagnostics rather than arbitrary file matches. Full per-language behavior is documented in [LANGUAGES.md](LANGUAGES.md).

## Depth and deterministic layout

Petgraph computes strongly connected components iteratively. Multi-file cycles and self-loops get stable cycle IDs. SCCs form a DAG; sinks have depth 0, other components have `1 + max(dependency depth)`. Members of a cycle share depth.

Overview places modules in dependency-depth columns. Neighbor view places dependencies, the selected file and its consumers in three columns. Impact view places the target in column 0 and its affected consumers in columns matching Rust's shortest-distance waves. During impact or result inspection, the all/impact-path switch can show unaffected files in their normal colors alongside the trace. Deterministic barycenter ordering keeps connected branches close, with paths as a stable tie-breaker.

SVG renders bounded nodes and real edges; its viewport supports fit, pan and zoom. Each view shows at most 250 nodes and exposes its hidden count. Large graphs retain full analysis counts even when only part of the graph is drawn; search retains the selected module and neighbor view narrows the visible scope.

Selection, the neighborhood anchor and the active impact report are independent. Single selection leaves the view and camera intact; double-click explicitly opens a neighborhood. Cameras are remembered per view (and neighborhood/report origin). External search only pans when its result is outside the viewport. A selected node keeps its relationship highlight until a different hover lasts 350 ms.

In the overview, right-button pointer dragging applies per-file offsets in graph coordinates. Pointer capture continues through canvas boundaries. SVG routes and arrow endpoints use the adjusted coordinates; graph connectivity and Rust results are unchanged. Single/all position reset removes offsets independently of simulation undo. Offsets survive view switches and simulation actions, and clear on repository replacement. Help and position controls are compact popovers.

## Impact semantics

Rust performs breadth-first traversal over reverse import edges. Wave 0 contains only the disconnected target. Each next wave contains newly visited importers at minimum distance. Already unavailable IDs are excluded from both propagation and counts. Cycles and converging branches visit a module once.

`totalAffected` and `blastRadius` exclude the target. `directAffected` is wave 1's size; `transitiveAffected = totalAffected - directAffected`. Ratios use the original module count. The UI keeps the disconnected target, other affected files and remaining files separate.

A display edge is a shortest causal edge only when:

```text
wave(importer) = wave(dependency) + 1
```

Playback sends a pulse from dependency to importer along that real edge, then marks the importer affected. Other real edges remain contextual; same-wave links and cycle return edges do not initiate another wave. A consumer becoming unavailable does not make its independent dependencies unavailable.

Clicking an affected node exposes its preceding causal dependencies. Pause/seek/replay change presentation, not Rust's impact result. Completion commits a simulation once; another available file can then start a new disconnect. Undo restores the complete pre-action snapshot, and restore-all clears the sequence. Async requests are checked against the current analysis and selection before committing.

Explicit cut origins and their reports are stored separately from unavailable IDs and action history. Selecting a disconnected origin always offers restoration, including during another live cut. Restoration first completes any live operation, removes the requested origin, then asks Rust to recompute each remaining cut in order against the accumulated unavailable set. The queried origin itself is excluded from that exclusion set if an earlier remaining cut already reaches it. The resulting union therefore retains shared failures. Restoration commits atomically only if the analysis and unavailable-set identity are still current, and records a complete undo snapshot. Reports remain accessible regardless of selection and can be switched without mutating the simulation.

Affected means potentially impacted according to static imports, not proven runtime failure. Type-only imports contribute to this graph.

## Curated demo and verification contract

`fixtures/demo-project` is a dependency-free, source-only project with 23 TypeScript modules, 24 internal edges, no unresolved/external imports and one isolated two-module cycle.

Disconnecting `src/core/config.ts` yields:

| Wave | Modules                                                  |
| ---- | -------------------------------------------------------- |
| 0    | core/config                                              |
| 1    | core/auth, core/http                                     |
| 2    | data/activity, data/profile, data/projects, data/session |
| 3    | features/account, features/board, features/feed          |
| 4    | views/settings, views/workspace                          |
| 5    | app/router                                               |
| 6    | app/main                                                 |

Paths are relative to `src/` and end in `.ts`. Exactly 13 other modules are affected and 9 remain available. The independent theme, formatting and plugin branches provide a visible counterexample to whole-project failure. Main imports theme, so main can be affected while theme remains available.

Rust fixture tests assert exact wave membership, require an actual preceding-wave dependency for every affected module and verify unaffected IDs. Disconnecting the isolated plugin registry affects the hooks file and plugin preview once, then stops.

## Analysis limits and metrics

The scanner accepts 24 extensions across the supported languages and honors local ignore files. It skips symlinks, dependency/build/cache directories and unsupported encodings. Module, file-size, total-source, entry-count and edge-count limits return clearly warned partial results. It does not fetch external libraries or read globally installed SDKs. JS/TS resolution still excludes tsconfig paths, package exports and workspace package linking.

The core retains LOC, fan-in/out, SCC, depth, blast metrics and an experimental stability heuristic. The compact UI prioritizes dependencies and impact. Core metrics and their formulas are documented in the [analysis README](../crates/repotower-core/README.md); they are not a runtime correctness or security rating.

## Source layout

- `crates/repotower-core`: reusable Rust library and JSON-lines process.
- `desktop`: Electron main, preload and portable runtime boundary.
- `src/graph`: deterministic SVG layout, dependency edges and causal playback.
- `src/state`, `src/components`, `src/lib`, `src/types`: application state, compact chrome, localization and typed contract.
- `fixtures`: focused graph cases, curated demo and generated performance inputs.
- `scripts`: portable build, package and integration checks.
- `.tools`, `.cache`, `.tmp`, `target`: ignored local development dependencies and artifacts.
- `release`: portable distribution; `docs`: architecture and actual validation evidence.

## Verification boundaries

Core tests check graph correctness independently of animation. Frontend graph tests check deterministic placement, real-edge direction, wave causality, bounded rendering and playback state. Desktop checks exercise the real Electron/Rust bridge, source immutability, preferences, window behavior, local data locations and blocked network requests.

Actual completed checks and their platform scope are recorded in [VALIDATION.md](VALIDATION.md). Windows validation does not establish macOS/Linux behavior; each platform must build and exercise its native sidecar and host.
