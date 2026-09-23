use repotower_core::{analyze, impact};
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
            .join("../../.tmp/mixed-tests")
            .join(format!(
                "{}-{}",
                std::process::id(),
                SERIAL.fetch_add(1, Ordering::SeqCst)
            ));
        for (id, source) in files {
            let path = root.join(id);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, source).unwrap();
        }
        Self(root)
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn mixed_repository_retains_packages_sources_and_separates_type_systems() {
    let project = Project::new(&[
        ("java/Config.java", "package demo; public class Config {}"),
        (
            "java/Service.java",
            "package demo; public class Service { Config config; }",
        ),
        ("dotnet/Config.cs", "namespace demo; public class Config {}"),
        (
            "dotnet/Service.cs",
            "namespace demo; public class Service { Config config; }",
        ),
        ("packages/client/config.ts", "export const port = 8080;"),
        (
            "packages/client/service.ts",
            "import { port } from './config'; export const service = port;",
        ),
        ("config.py", "port = 8080\n"),
        ("service.py", "from config import port\n"),
    ]);
    let result = analyze(&project.0, |_| {}).unwrap();
    assert_eq!((result.total_modules, result.total_edges), (8, 4));
    for (source, target) in [
        ("java/Service.java", "java/Config.java"),
        ("dotnet/Service.cs", "dotnet/Config.cs"),
        ("packages/client/service.ts", "packages/client/config.ts"),
        ("service.py", "config.py"),
    ] {
        assert!(result
            .edges
            .iter()
            .any(|edge| edge.source == source && edge.target == target));
        let effect = impact(&result.modules, target, &[]).unwrap();
        assert_eq!(effect.total_affected, 1);
        assert_eq!(effect.waves[1], vec![source]);
    }
    let repeated = analyze(&project.0, |_| {}).unwrap();
    assert_eq!(
        serde_json::to_value(result).unwrap(),
        serde_json::to_value(repeated).unwrap()
    );
}

#[test]
fn oversized_build_metadata_is_bounded_and_reported() {
    let manifest = format!("module example.org/large\n{}", " ".repeat(1024 * 1024));
    let project = Project::new(&[
        ("go.mod", &manifest),
        (
            "main.go",
            "package main\nimport \"example.org/large/config\"\nfunc main() { _ = config.Port }\n",
        ),
        ("config/config.go", "package config\nconst Port = 8080\n"),
    ]);
    let result = analyze(&project.0, |_| {}).unwrap();
    assert_eq!(result.total_modules, 2);
    assert!(result
        .warnings
        .iter()
        .any(|warning| warning.contains("项目配置超过读取限制")));
}

#[test]
fn node_next_runtime_suffixes_resolve_only_to_typescript_sources() {
    let project = Project::new(&[
        ("main.mts", "import './esm.mjs'; import './legacy.cjs';"),
        ("esm.mts", "export const a = 1;"),
        ("legacy.cts", "export const b = 2;"),
        ("esm.py", "a = 1\n"),
    ]);
    let result = analyze(&project.0, |_| {}).unwrap();
    assert_eq!(result.total_edges, 2);
    assert_eq!(result.unresolved_count, 0);
    assert!(result
        .edges
        .iter()
        .all(|edge| edge.target.ends_with(".mts") || edge.target.ends_with(".cts")));
}
