# RepoTower 0.3 analysis core

The library and JSON-lines executable are read-only. Analysis does not execute project code, resolve packages on the network or write to the analyzed directory. Source text is not part of the response.

## Protocol

Keep one `repotower-core` process running and send one JSON object per line. IDs are opaque JSON values echoed unchanged. The process keeps the latest successful analysis for impact requests.

```json
{"id":1,"command":"analyze","path":"/path/to/repository"}
{"id":2,"command":"impact","moduleId":"src/utils.ts","excluded":["src/removed.ts"]}
```

Each request ends with `{"id":...,"type":"result","result":...}` or `{"id":...,"type":"error","error":"..."}`. Analysis also emits `{"id":...,"type":"progress","progress":{"stage":"scan","completed":42}}`; stages are scan, parse, graph, metrics, complete. stdout contains protocol only. A failed analysis preserves the previous graph.

## Analysis rules

- Files: Java (`.java`), Python (`.py`, `.pyi`), C/C++ (`.c`, `.h`, `.cc`, `.cpp`, `.cxx`, `.hpp`, `.hh`, `.hxx`, `.inl`, `.ipp`), Go (`.go`), Rust (`.rs`), C# (`.cs`), JS/TS (`.js`, `.jsx`, `.ts`, `.tsx`, `.mjs`, `.cjs`, `.mts`, `.cts`). UTF-8 only. `linesOfCode` counts nonempty physical lines, including comments.
- Respect nested `.gitignore`, `.ignore` and `.repotowerignore` even without a `.git` directory. Rules use gitignore syntax; the latter files allow analysis-specific exclusions. Do not read global/parent ignore rules. Skip symlinks and common generated, cache, dependency and VCS directories, including `dist`, `build`, `out`, `target`, `node_modules` and `.git`.
- Limits: 10,000 modules, 2 MiB per source file, 128 MiB combined source, 200,000 visited directory entries and 100,000 distinct internal edges. Every truncation produces a warning; metrics then describe only the retained graph.
- A first pass reads eligible source files; language-specific Tree-sitter parsers and cached source indexes resolve dependencies in a second pass. Java/C# type lookup, Python module loading, C/C++ includes, Go package references and Rust module/use paths have separate resolvers. The compiled grammars need no installed language environment. Parse errors preserve detectable references with a warning. No source evaluation or build invocation occurs.
- Only successfully read/scanned files can become internal dependency targets. Configuration reads are bounded and root-confined. Unknown external libraries stay external; missing/ambiguous explicit local references produce unresolved diagnostics. The graph does not imply complete compiler or runtime symbol resolution. Conditional source branches may both contribute edges.
- JS/TS keeps static imports (including type-only imports), re-exports, literal `import()`, bare `require()`, and TypeScript `import = require()`. Relative resolution supports exact files, extensionless files, directory `index` files and `.js` to `.ts`/`.tsx` or `.jsx` to `.tsx` source fallbacks. Extension precedence is ts, tsx, js, jsx, mjs, cjs, mts, cts. Exact files take precedence. tsconfig paths, workspace packages, package exports and framework-generated modules remain unsupported; a shadowed local `require` can be a false positive.
- Language rules, examples and limitations are documented in [LANGUAGES.md](../../docs/LANGUAGES.md). In particular, Go package edges deliberately include multiple production files, while Java wildcard imports and C# namespace usings do not create blanket file dependencies. Python distinguishes package values from submodules. Type-only references intentionally contribute to potential impact.
- Duplicate importer/dependency pairs form one edge; the first parsed import kind is retained. Edge kinds are `static`, `export`, `dynamic`, `require`, `include`, `module` and `use`. `externalCount` counts distinct specifier/kind occurrences per module, not unique packages or every external type reference.

## Algorithms and score

Edges point **importer → dependency**. Iterative petgraph Kosaraju finds SCCs; components are deterministically sorted by smallest module path. Multi-node components and self-loops receive stable cycle IDs. Kahn traversal from dependency sinks computes maximum dependency depth; all modules in a cycle have the same depth.

Exact transitive importer bitsets propagate over the condensed DAG in topological order. SCC sizes weight the set bits. A module's blast radius includes every reachable importer but excludes itself. This avoids a separate BFS for every module; the 10,000-module bound uses at most approximately 12.6 MB of bitsets. Interactive impact uses reverse-edge breadth-first traversal, giving minimum-distance waves, and skips previously removed nodes entirely. Wave 0 contains only the pulled module. Ratios divide by the original number of modules, including the pulled one in the denominator.

The experimental stability score is rounded to one decimal:

`100 × (1 − 0.30 × averageBlastRatio − 0.30 × maxBlastRatio − 0.25 × cyclicModuleRatio − 0.15 × maxFanIn / moduleCount)`

It is clamped to 0–100. Empty graphs have zero ratios and score 100; the UI should show the empty state. The score measures a chosen structural heuristic, not correctness, code quality or security. Entry-like means no internal importers; this does not prove a runtime entry point.

## Verification and fixtures

Run `cargo test --workspace`; Windows portable toolchain setup is in `scripts/rust-env.ps1`. Fixtures cover linear, branching, cycles, missing dependencies and a curated 23-module demo with 24 edges and one isolated cycle. Tests assert config's exact six propagation steps, 13 affected dependents, 9 unaffected modules, real preceding-wave dependencies and cycle deduplication. They compare graph blast counts with BFS for every demo module and verify a 1,200-module deep chain without recursive traversal. `python -I -B fixtures/generate.py large --count 1000` creates a chain with shortcut imports for performance measurement; `node fixtures/generate-broad.mjs` creates a separate 1,000-module, 12-layer fixture. The curated demo is never regenerated by these commands.

Language integration tests build temporary source-only projects under the repository's `.tmp/` directory, then exercise the public analysis/impact API. They check actual target files and cascade paths, plus ambiguous names, external dependencies, ignored files and syntax recovery. They never import, compile or execute the fixture source.

The 2D circuit displays each internal edge in the reverse direction, **dependency → consumer**, matching impact propagation. Its shortest causal edges satisfy `wave(importer) = wave(dependency) + 1`; other real edges are contextual and do not trigger repeat propagation. This presentation does not change the analyzer's `source=importer, target=dependency` contract.

Primary API references: [Tree-sitter TypeScript](https://docs.rs/tree-sitter-typescript/0.23.2/tree_sitter_typescript/), [Tree-sitter JavaScript](https://docs.rs/tree-sitter-javascript/0.25.0/tree_sitter_javascript/), [petgraph iterative Kosaraju](https://docs.rs/petgraph/0.8.3/petgraph/algo/fn.kosaraju_scc.html).
