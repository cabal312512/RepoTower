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
            .join("../../.tmp/rust-language-tests")
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
fn declared_modules_grouped_imports_aliases_and_use_cycles() {
    let project = Project::new(&[
        ("Cargo.toml", "[package]\nname = \"sample-crate\"\nversion = \"0.1.0\""),
        ("src/lib.rs", "pub mod config; pub mod service; pub mod entry;"),
        ("src/config.rs", "pub struct Settings; pub const VALUE: u8 = 1;"),
        ("src/service/mod.rs", "use crate::config::{self, Settings as Config, *}; pub mod child; pub fn run() {}"),
        ("src/service/child.rs", "use super::run; use crate::{config::Settings, entry::start};"),
        ("src/entry.rs", "use crate::service::{self, child::*}; use std::collections::HashMap; pub fn start() {}"),
        ("src/main.rs", "use sample_crate::entry::start; fn main() { start(); }"),
        ("src/unused.rs", "pub fn unused() {}"),
    ]);
    let result = project.analyze();
    for (source, target) in [
        ("src/lib.rs", "src/config.rs"),
        ("src/service/mod.rs", "src/config.rs"),
        ("src/service/mod.rs", "src/service/child.rs"),
        ("src/service/child.rs", "src/service/mod.rs"),
        ("src/service/child.rs", "src/config.rs"),
        ("src/entry.rs", "src/service/child.rs"),
        ("src/main.rs", "src/entry.rs"),
    ] {
        assert!(has(&result, source, target), "missing {source} → {target}");
    }
    assert_eq!(
        result.unresolved_count,
        0,
        "{:?}",
        result
            .modules
            .iter()
            .flat_map(|m| &m.unresolved_imports)
            .collect::<Vec<_>>()
    );
    assert_eq!(result.external_count, 1);
    assert_eq!(result.cycle_count, 1);
    let impact = impact(&result.modules, "src/config.rs", &[]).unwrap();
    assert_eq!(impact.total_affected, 5);
    assert!(!impact
        .waves
        .iter()
        .flatten()
        .any(|id| id == "src/unused.rs"));
}

#[test]
fn inline_modules_and_path_attributes_follow_physical_location() {
    let project = Project::new(&[
        (
            "src/lib.rs",
            "mod outer; #[path = \"special/custom.rs\"] pub mod custom;",
        ),
        (
            "src/outer.rs",
            "mod inline { #[path = \"chosen.rs\"] mod nested; use self::nested::Item; }",
        ),
        (
            "src/outer/inline/chosen.rs",
            "use crate::custom::Special; pub struct Item;",
        ),
        ("src/special/custom.rs", "pub struct Special; mod deep;"),
        ("src/special/custom/deep.rs", "pub fn deep() {}"),
    ]);
    let result = project.analyze();
    assert!(has(&result, "src/outer.rs", "src/outer/inline/chosen.rs"));
    assert!(has(
        &result,
        "src/outer/inline/chosen.rs",
        "src/special/custom.rs"
    ));
    assert!(has(
        &result,
        "src/special/custom.rs",
        "src/special/custom/deep.rs"
    ));
    assert_eq!(
        result.unresolved_count,
        0,
        "{:?}",
        result
            .modules
            .iter()
            .flat_map(|m| &m.unresolved_imports)
            .collect::<Vec<_>>()
    );
}

#[test]
fn missing_ambiguous_and_escaping_module_paths_stay_unresolved() {
    let project = Project::new(&[
        ("lib.rs", "mod duplicate; mod missing; #[path = \"../outside.rs\"] mod outside; use crate::nonexistent::Thing; use super::Invalid;"),
        ("duplicate.rs", "pub struct Item;"),
        ("duplicate/mod.rs", "pub struct Item;"),
        ("outside.rs", "pub struct Item;"),
    ]);
    let result = project.analyze();
    assert_eq!(result.total_edges, 0);
    assert_eq!(
        result.unresolved_count,
        5,
        "{:?}",
        result
            .modules
            .iter()
            .flat_map(|m| &m.unresolved_imports)
            .collect::<Vec<_>>()
    );
}

#[test]
fn comments_strings_and_macro_tokens_are_not_imports() {
    let project = Project::new(&[
        ("lib.rs", "// mod hidden;\nconst TEXT: &str = \"use crate::hidden::Item;\";\nmacro_rules! generate { () => { mod hidden; use crate::hidden::Item; } }\nmod real;"),
        ("real.rs", "pub struct Item;"),
        ("hidden.rs", "pub struct Hidden;"),
    ]);
    let result = project.analyze();
    assert_eq!(result.total_edges, 1);
    assert!(has(&result, "lib.rs", "real.rs"));
    assert_eq!(result.unresolved_count, 0);
}

#[test]
fn explicit_expression_and_type_paths_resolve_without_use_declarations() {
    let project = Project::new(&[
        ("src/lib.rs", "pub mod config; pub mod service; pub mod view;"),
        ("src/config.rs", "pub struct Settings; pub const VALUE: u8 = 1;"),
        ("src/service.rs", "pub fn run(_: crate::config::Settings) -> u8 { super::config::VALUE } mod inner { fn run() { let _: super::super::config::Settings; } }"),
        ("src/view.rs", "pub fn render() { let callback = crate::service::run; }"),
        ("src/unrelated.rs", "fn ignored() { std::mem::drop(1); } macro_rules! hidden { () => { crate::config::VALUE } }"),
    ]);
    let result = project.analyze();
    assert!(has(&result, "src/service.rs", "src/config.rs"));
    assert!(has(&result, "src/view.rs", "src/service.rs"));
    assert!(!has(&result, "src/unrelated.rs", "src/config.rs"));
    assert_eq!(result.unresolved_count, 0);
    assert_eq!(
        impact(&result.modules, "src/config.rs", &[]).unwrap().waves,
        vec![
            vec!["src/config.rs"],
            vec!["src/lib.rs", "src/service.rs"],
            vec!["src/view.rs"]
        ]
    );
}
