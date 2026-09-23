# Pocket Workspace

A small, source-only TypeScript project for the built-in dependency demo. No packages or network are needed. RepoTower reads these files; it never runs or modifies them.

23 files, 24 import edges, one deliberately isolated two-file cycle. Each file has a real responsibility and uses every import. The two application branches share `core/config.ts`; the theme editor, text report, and plugin preview work independently.

Pull `src/core/config.ts` to see this exact chain (arrows mean **a dependency is needed by** the next file):

```text
config → http → projects → board ─┐
             └ activity → feed ─┴ workspace ─┐
       └ auth → profile ─┐                  ├ router → main
              └ session ┴ account → settings ┘

palette → theme → appearance → preview       (unaffected)
                └ main                       (main depends on theme, not vice versa)
format → report                              (unaffected)
registry ↔ hooks; registry → plugins-preview (isolated cycle)
```

The impact is **13 dependent files**, in waves of **2, 4, 3, 2, 1, 1**. Including the pulled config, 14 files become unavailable and 9 stay available. Wave membership uses shortest dependency distance, so `account.ts` is visited once even though both `profile.ts` and `session.ts` depend on auth.

Pulling `src/plugins/registry.ts` affects only `hooks.ts` and `plugins-preview.ts`. The cycle is visited once; it never repeats indefinitely.

This fixture is intentionally curated rather than generated. For large performance fixtures, use `fixtures/generate.py large` or `fixtures/generate-broad.mjs`.
