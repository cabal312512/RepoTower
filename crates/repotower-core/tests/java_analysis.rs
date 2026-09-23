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
            .join("../../.tmp/core-tests")
            .join(format!(
                "java-{}-{}",
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
fn imports<'a>(result: &'a RepositoryAnalysis, id: &str) -> &'a [String] {
    &result.modules.iter().find(|m| m.id == id).unwrap().imports
}

#[test]
fn java_packages_and_same_package_types_have_exact_impact_waves() {
    let project = Project::new(&[
        (
            "backend/src/main/java/demo/core/Config.java",
            "package demo.core; public record Config(String url) {}",
        ),
        (
            "unusual/Service.java",
            "package demo.core; public class Service { Config config; }",
        ),
        (
            "ui/Main.java",
            "package demo.ui; import demo.core.Service; class Main { Service service; }",
        ),
        (
            "other/Unused.java",
            "package demo.core; public interface Unused {}",
        ),
        ("foreign/Config.ts", "export class Config {}"),
    ]);
    let result = project.analyze();
    assert_eq!(result.total_edges, 2);
    assert_eq!(
        imports(&result, "unusual/Service.java"),
        &["backend/src/main/java/demo/core/Config.java"]
    );
    assert_eq!(
        impact(
            &result.modules,
            "backend/src/main/java/demo/core/Config.java",
            &[]
        )
        .unwrap()
        .waves,
        vec![
            vec!["backend/src/main/java/demo/core/Config.java"],
            vec!["unusual/Service.java"],
            vec!["ui/Main.java"]
        ]
    );
    assert_eq!(result.unresolved_count, 0);
}

#[test]
fn java_wildcards_link_only_referenced_types_and_report_collisions() {
    let project = Project::new(&[
        ("a/One.java", "package a; public class One {}"),
        ("a/Unused.java", "package a; public class Unused {}"),
        ("a/Clash.java", "package a; public class Clash {}"),
        ("b/Clash.java", "package b; public class Clash {}"),
        ("Main.java", "import a.*; import b.*; import java.util.List; class Main { One one; Clash ambiguous; List<String> names; String s = \"Unused\"; /* Unused falseEdge; */ }"),
    ]);
    let result = project.analyze();
    assert_eq!(imports(&result, "Main.java"), &["a/One.java"]);
    assert_eq!(result.unresolved_count, 1);
    assert_eq!(result.external_count, 1);
}

#[test]
fn java_nested_static_annotation_and_qualified_types() {
    let project = Project::new(&[
        ("Outer.java", "package core; public class Outer { public static class Inner {} public static int VALUE; public static void run() {} }"),
        ("Marker.java", "package core; public @interface Marker {}"),
        ("Record.java", "package core; public record Record(int value) {}"),
        ("Use.java", "package app; import core.Outer.Inner; import static core.Outer.VALUE; @core.Marker class Use { Inner inner; core.Record record; int n = VALUE; }"),
        ("Static.java", "package core; class Static { void run() { Outer.run(); } }"),
    ]);
    let result = project.analyze();
    assert_eq!(
        imports(&result, "Use.java"),
        &["Marker.java", "Outer.java", "Record.java"]
    );
    assert_eq!(imports(&result, "Static.java"), &["Outer.java"]);
    assert_eq!(result.unresolved_count, 0);
}

#[test]
fn java_duplicate_classpaths_are_unresolved_and_generic_parameters_are_not_classes() {
    let project = Project::new(&[
        ("main/Thing.java", "package demo; public class Thing {}"),
        ("test/Thing.java", "package demo; public class Thing {}"),
        ("Other.java", "package demo; class Other {}"),
        ("Use.java", "package app; import demo.Thing; import demo.Missing; class Use<Other> { Thing thing; Other generic; }"),
        ("Generic.java", "package demo; class Generic<Other> { Other generic; }"),
    ]);
    let result = project.analyze();
    assert_eq!(result.total_edges, 0);
    assert!(result
        .modules
        .iter()
        .find(|m| m.id == "Use.java")
        .unwrap()
        .unresolved_imports
        .iter()
        .any(|i| i.specifier == "demo.Missing"));
    assert!(result
        .modules
        .iter()
        .find(|m| m.id == "Use.java")
        .unwrap()
        .unresolved_imports
        .iter()
        .any(|i| i.specifier == "demo.Thing"));
}

#[test]
fn java_type_parameters_are_lexical_and_value_receivers_do_not_fake_static_dependencies() {
    let project = Project::new(&[
        ("Type.java", "package app; class Type {}"),
        ("Receiver.java", "package app; class Receiver { static void run() {} }"),
        ("Use.java", "package app; class Generic<Type> { Type value; } class Use { Type concrete; void call(Object Receiver) { Receiver.toString(); } }"),
    ]);
    let result = project.analyze();
    assert_eq!(imports(&result, "Use.java"), &["Type.java"]);
    assert_eq!(result.total_edges, 1);
}
