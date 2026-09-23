use repotower_core::{analyze, impact, model::RepositoryAnalysis};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static SERIAL: AtomicUsize = AtomicUsize::new(0);

struct Project {
    root: PathBuf,
}
impl Project {
    fn new(files: &[(&str, &str)]) -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../.tmp/core-tests")
            .join(format!(
                "{}-{}",
                std::process::id(),
                SERIAL.fetch_add(1, Ordering::SeqCst)
            ));
        fs::create_dir_all(&root).unwrap();
        for (name, contents) in files {
            let path = root.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }
        Self { root }
    }
    fn analyze(&self) -> RepositoryAnalysis {
        analyze(&self.root, |_| {}).unwrap()
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn fixture(name: &str) -> RepositoryAnalysis {
    analyze(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(name),
        |_| {},
    )
    .unwrap()
}

#[test]
fn linear_direction_depth_and_waves() {
    let result = fixture("linear");
    assert_eq!((result.total_modules, result.total_edges), (3, 2));
    let leaf = result
        .modules
        .iter()
        .find(|m| m.id == "foundation.ts")
        .unwrap();
    assert_eq!(
        (
            leaf.fan_in,
            leaf.fan_out,
            leaf.dependency_depth,
            leaf.blast_radius
        ),
        (1, 0, 0, 2)
    );
    assert_eq!(leaf.blast_ratio, 2.0 / 3.0);
    let top = result.modules.iter().find(|m| m.id == "main.ts").unwrap();
    assert_eq!((top.dependency_depth, top.blast_radius), (2, 0));
    assert!(top.is_entry_like);
    let pull = impact(&result.modules, "foundation.ts", &[]).unwrap();
    assert_eq!(
        pull.waves,
        vec![vec!["foundation.ts"], vec!["service.ts"], vec!["main.ts"]]
    );
    assert_eq!(
        (
            pull.total_affected,
            pull.direct_affected,
            pull.transitive_affected,
            pull.max_cascade_depth
        ),
        (2, 1, 1, 2)
    );
    let excluded = impact(&result.modules, "foundation.ts", &["service.ts".into()]).unwrap();
    assert_eq!(excluded.total_affected, 0);
    assert!(impact(&result.modules, "main.ts", &["main.ts".into()]).is_err());
}

#[test]
fn branching_shortest_distance_and_deduplication() {
    let result = fixture("branching");
    let pull = impact(&result.modules, "base.ts", &[]).unwrap();
    assert_eq!(
        pull.waves,
        vec![vec!["base.ts"], vec!["left.ts", "right.ts"], vec!["app.ts"]]
    );
    assert_eq!(pull.total_affected, 3);
    let pull = impact(&result.modules, "base.ts", &["left.ts".into()]).unwrap();
    assert_eq!(
        pull.waves,
        vec![vec!["base.ts"], vec!["right.ts"], vec!["app.ts"]]
    );
    assert_eq!(result.total_edges, 4);
}

#[test]
fn cycles_self_loops_and_component_depth() {
    let result = fixture("cycles");
    assert_eq!(result.cycle_count, 2);
    let a = result.modules.iter().find(|m| m.id == "a.ts").unwrap();
    let b = result.modules.iter().find(|m| m.id == "b.ts").unwrap();
    assert_eq!(a.cycle_id, b.cycle_id);
    assert!(a.cycle_id.is_some());
    assert_eq!(a.dependency_depth, b.dependency_depth);
    assert_eq!(a.dependency_depth, 1);
    assert_eq!(a.blast_radius, 2);
    let self_loop = result.modules.iter().find(|m| m.id == "self.ts").unwrap();
    assert!(self_loop.cycle_id.is_some());
    assert_eq!(self_loop.blast_radius, 0);
    assert_eq!(
        impact(&result.modules, "self.ts", &[]).unwrap().waves,
        vec![vec!["self.ts"]]
    );
}

#[test]
fn unresolved_dynamic_external_and_extension_rules() {
    let result = fixture("unresolved");
    assert_eq!(result.unresolved_count, 4);
    assert_eq!(result.external_count, 2);
    assert_eq!(result.total_edges, 2);
    let main = result.modules.iter().find(|m| m.id == "main.ts").unwrap();
    assert_eq!(main.imports, vec!["lib/index.ts", "value.ts"]);
}

#[test]
fn scanner_respects_gitignore_without_git_and_default_exclusions() {
    let project = Project::new(&[
        (
            ".gitignore",
            "ignored/\n*.generated.ts\n!keep.generated.ts\n",
        ),
        (".ignore", "scratch/\n"),
        (".repotowerignore", "analysis-only-exclusion/\n"),
        ("main.ts", "export {}"),
        ("ignored/a.ts", "export {}"),
        ("node_modules/pkg/a.ts", "export {}"),
        ("dist/a.js", "export {}"),
        ("out/bundle.js", "export {}"),
        ("packages/one/out/bundle.js", "export {}"),
        ("scratch/temporary.ts", "export {}"),
        ("analysis-only-exclusion/a.ts", "export {}"),
        ("src/skip.generated.ts", "export {}"),
        ("src/keep.generated.ts", "export {}"),
        (".hidden.ts", "export {}"),
        ("src/.gitignore", "skip.ts\n"),
        ("src/skip.ts", "export {}"),
        ("src/keep.tsx", "export const v = <div/>;"),
        ("src/nested/.ignore", "local.ts\n"),
        ("src/nested/local.ts", "export {}"),
        ("fixtures/.gitignore", "generated-*/\n"),
        ("fixtures/generated-many/a.ts", "export {}"),
    ]);
    let result = project.analyze();
    assert_eq!(
        result
            .modules
            .iter()
            .map(|m| m.id.as_str())
            .collect::<Vec<_>>(),
        vec![
            ".hidden.ts",
            "main.ts",
            "src/keep.generated.ts",
            "src/keep.tsx"
        ]
    );
}

#[test]
fn partial_syntax_errors_and_invalid_tsconfig_do_not_abort_analysis() {
    let project = Project::new(&[
        ("tsconfig.json", "{ invalid config"),
        (
            "broken.ts",
            "import './dependency'; import '@/not-resolved'; const = ;",
        ),
        ("dependency.ts", "export const value = 1;"),
    ]);
    let result = project.analyze();
    assert_eq!(
        (
            result.total_modules,
            result.total_edges,
            result.unresolved_count
        ),
        (2, 1, 1)
    );
    assert!(result
        .warnings
        .iter()
        .any(|warning| warning.contains("broken.ts") && warning.contains("解析错误")));
    assert_eq!(
        impact(&result.modules, "dependency.ts", &[])
            .unwrap()
            .total_affected,
        1
    );
}

#[test]
fn empty_invalid_utf8_and_large_files_are_reported() {
    let project = Project::new(&[("notes.txt", "not source")]);
    let empty = project.analyze();
    assert_eq!(empty.total_modules, 0);
    assert_eq!(empty.stability_score, 100.0);
    assert!(!empty.warnings.is_empty());
    fs::write(project.root.join("bad.ts"), [0xff, 0xfe]).unwrap();
    fs::write(
        project.root.join("huge.js"),
        vec![b'x'; 2 * 1024 * 1024 + 1],
    )
    .unwrap();
    let result = project.analyze();
    assert_eq!(result.total_modules, 0);
    assert!(result.warnings.iter().any(|w| w.contains("UTF-8")));
    assert!(result.warnings.iter().any(|w| w.contains("2 MiB")));
}

#[test]
fn import_equals_require_and_esm_commonjs_extensions() {
    let project = Project::new(&[
        (
            "main.ts",
            "import legacy = require('./legacy.cjs'); import('./async.mjs');",
        ),
        ("legacy.cjs", "module.exports = {};"),
        ("async.mjs", "export default 1;"),
    ]);
    let result = project.analyze();
    assert_eq!(result.total_edges, 2);
}

#[test]
fn deep_graph_is_iterative_and_exact() {
    let project = Project::new(&[]);
    for index in 0..1200 {
        let content = if index == 0 {
            "export const value = 0;".into()
        } else {
            format!("import './m{}.ts';", index - 1)
        };
        fs::write(project.root.join(format!("m{index}.ts")), content).unwrap();
    }
    let result = project.analyze();
    let base = result.modules.iter().find(|m| m.id == "m0.ts").unwrap();
    assert_eq!(base.blast_radius, 1199);
    let top = result.modules.iter().find(|m| m.id == "m1199.ts").unwrap();
    assert_eq!(top.dependency_depth, 1199);
    let pull = impact(&result.modules, "m0.ts", &[]).unwrap();
    assert_eq!(pull.max_cascade_depth, 1199);
    assert!(result.stability_score >= 0.0 && result.stability_score <= 100.0);
}

#[test]
fn graph_blast_metrics_equal_bfs_for_every_demo_module() {
    let result = fixture("demo-project");
    assert_eq!((result.total_modules, result.total_edges), (23, 24));
    assert_eq!(
        (
            result.unresolved_count,
            result.external_count,
            result.cycle_count
        ),
        (0, 0, 1)
    );
    for module in &result.modules {
        let pull = impact(&result.modules, &module.id, &[]).unwrap();
        assert_eq!(module.blast_radius, pull.total_affected, "{}", module.id);
    }
    let again = fixture("demo-project");
    assert_eq!(
        serde_json::to_string(&result).unwrap(),
        serde_json::to_string(&again).unwrap()
    );
}

#[test]
fn demo_has_real_branching_causes_and_leaves_independent_tools_available() {
    let result = fixture("demo-project");
    let pull = impact(&result.modules, "src/core/config.ts", &[]).unwrap();
    assert_eq!(
        pull.waves,
        vec![
            vec!["src/core/config.ts"],
            vec!["src/core/auth.ts", "src/core/http.ts"],
            vec![
                "src/data/activity.ts",
                "src/data/profile.ts",
                "src/data/projects.ts",
                "src/data/session.ts"
            ],
            vec![
                "src/features/account.ts",
                "src/features/board.ts",
                "src/features/feed.ts"
            ],
            vec!["src/views/settings.ts", "src/views/workspace.ts"],
            vec!["src/app/router.ts"],
            vec!["src/app/main.ts"],
        ]
    );
    assert_eq!(
        (
            pull.total_affected,
            pull.direct_affected,
            pull.transitive_affected
        ),
        (13, 2, 11)
    );
    assert_eq!(pull.max_cascade_depth, 6);

    // Every visible propagation can be justified by an actual import in the preceding wave.
    // The two causes of account/workspace converge without counting either file twice.
    for consecutive in pull.waves.windows(2) {
        for id in &consecutive[1] {
            let module = result.modules.iter().find(|m| &m.id == id).unwrap();
            assert!(
                module
                    .imports
                    .iter()
                    .any(|dependency| consecutive[0].contains(dependency)),
                "{id}"
            );
        }
    }
    let unavailable: Vec<_> = pull.waves.iter().flatten().collect();
    let untouched: Vec<_> = result
        .modules
        .iter()
        .filter(|module| !unavailable.contains(&&module.id))
        .map(|module| module.id.as_str())
        .collect();
    assert_eq!(
        untouched,
        vec![
            "src/app/plugins-preview.ts",
            "src/app/preview.ts",
            "src/cli/report.ts",
            "src/plugins/hooks.ts",
            "src/plugins/registry.ts",
            "src/ui/appearance.ts",
            "src/ui/palette.ts",
            "src/ui/theme.ts",
            "src/utils/format.ts",
        ]
    );

    let cycle = impact(&result.modules, "src/plugins/registry.ts", &[]).unwrap();
    assert_eq!(
        cycle.waves,
        vec![
            vec!["src/plugins/registry.ts"],
            vec!["src/app/plugins-preview.ts", "src/plugins/hooks.ts"],
        ]
    );
    assert_eq!(cycle.total_affected, 2);
}
