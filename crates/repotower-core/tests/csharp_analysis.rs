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
                "csharp-{}-{}",
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
fn csharp_namespaces_partial_types_and_impact_expand_real_declarations() {
    let project = Project::new(&[
        (
            "Core/Config.cs",
            "namespace App.Core; public partial class Config { public string Url; }",
        ),
        (
            "Core/Config.More.cs",
            "namespace App.Core { public partial class Config { public int Port; } }",
        ),
        (
            "Core/Service.cs",
            "namespace App.Core; public class Service { Config config; }",
        ),
        (
            "Ui/Main.cs",
            "using App.Core; namespace App.Ui; class Main { Service service; }",
        ),
        (
            "Core/Unused.cs",
            "namespace App.Core; public class Unused {}",
        ),
    ]);
    let result = project.analyze();
    assert_eq!(
        imports(&result, "Core/Service.cs"),
        &["Core/Config.More.cs", "Core/Config.cs"]
    );
    assert_eq!(imports(&result, "Ui/Main.cs"), &["Core/Service.cs"]);
    assert_eq!(result.total_edges, 3);
    assert_eq!(
        impact(&result.modules, "Core/Config.cs", &[])
            .unwrap()
            .waves,
        vec![
            vec!["Core/Config.cs"],
            vec!["Core/Service.cs"],
            vec!["Ui/Main.cs"]
        ]
    );
}

#[test]
fn csharp_alias_static_nested_qualified_generic_and_attribute_references() {
    let project = Project::new(&[
        ("Outer.cs", "namespace Core; public class Outer { public class Inner {} public static int Value; }"),
        ("Data.cs", "namespace Core; public record Data(int Count);"),
        ("Box.cs", "namespace Core; public class Box<T> {}"),
        ("Marker.cs", "namespace Core; public class MarkerAttribute : System.Attribute {}"),
        ("Use.cs", "using Alias = Core.Outer.Inner; using C = Core; using static Core.Outer; using Core; [Marker] class Use { Alias inner; C.Box<global::Core.Data> values; int n = Value; }"),
    ]);
    let result = project.analyze();
    assert_eq!(
        imports(&result, "Use.cs"),
        &["Box.cs", "Data.cs", "Marker.cs", "Outer.cs"]
    );
    assert_eq!(result.unresolved_count, 0);
}

#[test]
fn csharp_using_does_not_connect_entire_namespace_or_strings_and_generic_parameters() {
    let project = Project::new(&[
        ("Data.cs", "namespace Core; public class Data {}"),
        ("Unused.cs", "namespace Core; public class Unused {}"),
        ("Use.cs", "using Core; using System; class Use<Unused> { Data data; Unused generic; string text = \"new Unused()\"; /* Unused fake; */ }"),
        ("Foreign.java", "package Core; public class Data {}"),
    ]);
    let result = project.analyze();
    assert_eq!(imports(&result, "Use.cs"), &["Data.cs"]);
    assert_eq!(result.total_edges, 1);
    assert_eq!(result.external_count, 1);
}

#[test]
fn csharp_global_using_resolves_refs_and_duplicate_nonpartial_types_are_unresolved() {
    let project = Project::new(&[
        ("Imports.cs", "global using Core;"),
        ("Data.cs", "namespace Core; public class Data {}"),
        ("One/Clash.cs", "namespace Core; public class Clash {}"),
        ("Two/Clash.cs", "namespace Core; public class Clash {}"),
        ("Use.cs", "class Use { Data data; Clash ambiguous; }"),
    ]);
    let result = project.analyze();
    assert_eq!(imports(&result, "Use.cs"), &["Data.cs"]);
    assert_eq!(imports(&result, "Imports.cs").len(), 0);
    assert_eq!(result.unresolved_count, 1);
}

#[test]
fn csharp_relative_namespace_usings_and_generic_arity_resolve_without_basename_guesses() {
    let project = Project::new(&[
        ("Plain.cs", "namespace App.Core; public class Box {}"),
        ("Generic.cs", "namespace App.Core; public class Box<T> {}"),
        ("Data.cs", "namespace App.Core; public class Data {}"),
        ("RootBox.cs", "public class RootBox<T> {}"),
        ("Use.cs", "namespace App { using Core; class Use { Box<Data> generic; Data? optional; Data[] items; } }"),
        ("Alias.cs", "using Pack = App.Core; using Items = System.Collections.Generic.List<App.Core.Data>; class Alias { Pack::Data item; global::RootBox<Pack::Data> generic; }"),
    ]);
    let result = project.analyze();
    assert_eq!(imports(&result, "Use.cs"), &["Data.cs", "Generic.cs"]);
    assert_eq!(imports(&result, "Alias.cs"), &["Data.cs", "RootBox.cs"]);
    assert_eq!(result.unresolved_count, 0);
}
