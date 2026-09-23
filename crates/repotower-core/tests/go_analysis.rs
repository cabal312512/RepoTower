use repotower_core::{analyze, impact, model::RepositoryAnalysis};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
static SERIAL: AtomicUsize = AtomicUsize::new(0);
struct Project(PathBuf);
impl Project {
    fn new(files: &[(&str, &str)]) -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../.tmp/go-tests")
            .join(format!(
                "{}-{}",
                std::process::id(),
                SERIAL.fetch_add(1, Ordering::SeqCst)
            ));
        fs::create_dir_all(&root).unwrap();
        for (name, source) in files {
            let path = root.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, source).unwrap();
        }
        Self(root)
    }
    fn analyze(&self) -> RepositoryAnalysis {
        analyze(&self.0, |_| {}).unwrap()
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn has(result: &RepositoryAnalysis, source: &str, target: &str) -> bool {
    result
        .edges
        .iter()
        .any(|edge| edge.source == source && edge.target == target)
}

#[test]
fn package_imports_expand_production_files_but_not_tests() {
    let project = Project::new(&[
        ("go.mod", "module example.com/app\n\ngo 1.23\n"),
        ("cmd/main.go", "package main\nimport (\n cfg `example.com/app/config`\n _ \"example.com/app/service\"\n \"fmt\"\n)\nfunc main() { fmt.Println(cfg.Value) }"),
        ("config/a.go", "package config\nconst Value = 1"),
        ("config/b.go", "package config\nconst Other = 2"),
        ("config/a_test.go", "package config\nimport \"testing\"\nfunc Test(t *testing.T) { _ = Value }"),
        ("service/service.go", "package service\nimport \"example.com/app/config\"\nfunc Run() int { return config.Value }"),
        ("alone/alone.go", "package alone\nfunc Standalone() {}"),
    ]);
    let result = project.analyze();
    assert_eq!(result.total_modules, 6);
    for target in ["config/a.go", "config/b.go", "service/service.go"] {
        assert!(has(&result, "cmd/main.go", target), "missing {target}");
    }
    assert!(!has(&result, "cmd/main.go", "config/a_test.go"));
    assert!(has(&result, "config/a_test.go", "config/a.go"));
    assert_eq!(result.unresolved_count, 0);
    assert_eq!(result.external_count, 2);
    let impact = impact(&result.modules, "config/a.go", &[]).unwrap();
    assert_eq!(impact.total_affected, 3);
    assert!(!impact
        .waves
        .iter()
        .flatten()
        .any(|id| id == "config/b.go" || id == "alone/alone.go"));
}

#[test]
fn same_package_symbols_have_real_edges_and_no_complete_clique() {
    let project = Project::new(&[
        ("go.mod", "module local/app"),
        ("app.go", "package app\nfunc Entry() { Work() }"),
        ("work.go", "package app\nfunc Work() { Save() }"),
        ("save.go", "package app\nfunc Save() {}"),
        ("unused.go", "package app\nfunc Unused() {}"),
        (
            "shadow.go",
            "package app\nfunc Shadow(Work func()) { Work() }",
        ),
    ]);
    let result = project.analyze();
    assert_eq!(result.total_edges, 2, "{:?}", result.edges);
    assert_eq!(
        impact(&result.modules, "save.go", &[]).unwrap().waves,
        vec![vec!["save.go"], vec!["work.go"], vec!["app.go"]]
    );
}

#[test]
fn nested_modules_and_ambiguous_local_packages_are_not_guessed() {
    let project = Project::new(&[
        ("go.mod", "/* module fake */\nmodule \"example.com/root\" // comment"),
        ("main.go", "package main\nimport (\n _ \"example.com/root/child/pkg\"\n _ \"example.com/child/pkg\"\n _ \"example.com/root/mixed\"\n _ \"../outside\"\n)"),
        ("child/go.mod", "module example.com/child"),
        ("child/pkg/a.go", "package pkg\nconst A = 1"),
        ("mixed/a.go", "package a\nconst A = 1"),
        ("mixed/b.go", "package b\nconst B = 1"),
    ]);
    let result = project.analyze();
    assert_eq!(result.total_edges, 1, "{:?}", result.edges);
    assert!(has(&result, "main.go", "child/pkg/a.go"));
    assert_eq!(result.unresolved_count, 3);
}

#[test]
fn package_cycles_and_comments_are_distinguished() {
    let project = Project::new(&[
        ("go.mod", "module example.com/cycle"),
        ("a/a.go", "package a\nimport _ \"example.com/cycle/b\"\n// import \"example.com/cycle/fake\"\nconst Text = `import \"example.com/cycle/fake\"`"),
        ("b/b.go", "package b\nimport _ \"example.com/cycle/a\""),
        ("fake/fake.go", "package fake"),
    ]);
    let result = project.analyze();
    assert_eq!(result.total_edges, 2);
    assert_eq!(result.cycle_count, 1);
    assert_eq!(
        impact(&result.modules, "a/a.go", &[])
            .unwrap()
            .total_affected,
        1
    );
}
