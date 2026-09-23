use repotower_core::{analyze, impact, model::DependencyKind};
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
            .join("../../.tmp/c-tests")
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
fn c_header_chain_has_real_impact_and_ignores_comments_and_strings() {
    let project = Project::new(&[
        ("include/config.h", "#pragma once\n#define PORT 8080\n"),
        ("src/api.h", "#include \"../include/config.h\"\nint port(void);\n"),
        ("src/main.c", "#include \"api.h\"\n#include <stdio.h>\n// #include \"ghost.h\"\nconst char *s = \"#include missing\";\nint main(void) {return port();}\n"),
        ("src/independent.c", "int independent(void) {return 0;}\n"),
    ]);
    let result = analyze(&project.0, |_| {}).unwrap();
    assert_eq!(
        (
            result.total_modules,
            result.total_edges,
            result.unresolved_count,
            result.external_count
        ),
        (4, 2, 0, 1)
    );
    assert!(result
        .edges
        .iter()
        .all(|edge| edge.kind == DependencyKind::Include));
    assert_eq!(
        impact(&result.modules, "include/config.h", &[])
            .unwrap()
            .waves,
        vec![
            vec!["include/config.h"],
            vec!["src/api.h"],
            vec!["src/main.c"]
        ]
    );
}

#[test]
fn cpp_templates_local_angle_paths_and_header_cycles() {
    let project = Project::new(&[
        ("include/lib/a.hpp", "#pragma once\n#include \"b.hpp\"\ntemplate<typename T> struct A { T value; };\n"),
        ("include/lib/b.hpp", "#pragma once\n#include \"a.hpp\"\n"),
        ("src/main.cpp", "#include <lib/a.hpp>\n#include <vector>\nint main() { A<int> a{1}; return a.value; }\n"),
        ("compile_commands.json", r#"[{"directory":".","file":"src/main.cpp","arguments":["c++","-Iinclude","-c","src/main.cpp"]}]"#),
    ]);
    let result = analyze(&project.0, |_| {}).unwrap();
    assert_eq!(
        (result.total_modules, result.total_edges, result.cycle_count),
        (3, 3, 1)
    );
    assert_eq!(result.unresolved_count, 0);
    assert!(!result
        .warnings
        .iter()
        .any(|message| message.contains("包含解析错误")));
    assert_eq!(
        impact(&result.modules, "include/lib/a.hpp", &[])
            .unwrap()
            .total_affected,
        2
    );
}

#[test]
fn sibling_quotes_take_precedence_but_ambiguous_search_is_not_guessed() {
    let project = Project::new(&[
        ("one/config.h", "#pragma once\n"), ("two/config.h", "#pragma once\n"),
        ("one/main.c", "#include \"config.h\"\n"),
        ("main.cpp", "#include <config.h>\n#include \"missing.h\"\n#include HEADER_PATH\n#include \"../outside.h\"\n"),
        ("config.py", "answer = 42\n"),
    ]);
    let result = analyze(&project.0, |_| {}).unwrap();
    assert_eq!(result.total_edges, 1);
    assert_eq!(result.edges[0].target, "one/config.h");
    assert_eq!(result.unresolved_count, 4);
    let main = result
        .modules
        .iter()
        .find(|module| module.id == "main.cpp")
        .unwrap();
    assert!(main
        .unresolved_imports
        .iter()
        .any(|import| import.reason.contains("编译器搜索路径")));
    assert!(main
        .unresolved_imports
        .iter()
        .any(|import| import.reason.contains("宏计算")));
}

#[test]
fn conditional_includes_remain_potential_and_js_does_not_guess_python_files() {
    let project = Project::new(&[
        (
            "main.c",
            "#if TARGET_A\n#include \"a.h\"\n#else\n#include \"b.h\"\n#endif\n",
        ),
        ("a.h", "#pragma once\n"),
        ("b.h", "#pragma once\n"),
        ("main.ts", "import './helper';"),
        ("helper.py", "value = 1\n"),
        ("types.mts", "export type Value = string;"),
    ]);
    let result = analyze(&project.0, |_| {}).unwrap();
    assert_eq!(result.total_modules, 6);
    assert_eq!(result.total_edges, 2);
    assert_eq!(result.unresolved_count, 1);
    assert!(result.edges.iter().all(|edge| edge.source == "main.c"));
}

#[test]
fn unrelated_headers_never_replace_system_headers_by_matching_basename() {
    let project = Project::new(&[
        ("main.c", "#include <stdio.h>\n#include <stdlib.h>\n"),
        ("quoted.c", "#include \"stdio.h\"\n"),
        ("mock/stdio.h", "#pragma once\n"),
    ]);
    let result = analyze(&project.0, |_| {}).unwrap();
    assert_eq!(
        (
            result.total_edges,
            result.unresolved_count,
            result.external_count
        ),
        (0, 2, 1)
    );
}

#[test]
fn compile_database_respects_search_order_and_reports_variant_conflicts() {
    let project = Project::new(&[
        ("one/config.h", "#pragma once\n"),
        ("two/config.h", "#pragma once\n"),
        ("ordered.c", "#include <config.h>\n"),
        ("variants.c", "#include <config.h>\n"),
        (
            "compile_commands.json",
            r#"[
          {"directory":".","file":"ordered.c","arguments":["cc","-Ione","-Itwo"]},
          {"directory":".","file":"variants.c","arguments":["cc","-Ione","-Itwo"]},
          {"directory":".","file":"variants.c","arguments":["cc","-Itwo","-Ione"]}
        ]"#,
        ),
    ]);
    let result = analyze(&project.0, |_| {}).unwrap();
    assert_eq!(result.total_edges, 1);
    assert_eq!(
        (&*result.edges[0].source, &*result.edges[0].target),
        ("ordered.c", "one/config.h")
    );
    assert_eq!(result.unresolved_count, 1);
}

#[test]
fn compile_database_tokenizes_quotes_msvc_and_quote_only_paths_without_execution() {
    let project = Project::new(&[
        ("include space/item.h", "#pragma once\n"),
        ("quotes/item.h", "#pragma once\n"),
        ("src/quoted.cpp", "#include \"item.h\"\n#include <item.h>\n"),
        ("src/msvc.cpp", "#include <item.h>\n"),
        ("src/bad.cpp", "#include <item.h>\n"),
        (
            "compile_commands.json",
            r#"[
          {"directory":"src","file":"quoted.cpp","command":"c++ -I '../include space' -iquote ../quotes -c quoted.cpp"},
          {"directory":".","file":"src/msvc.cpp","command":"cl /I\"include space\" /c src/msvc.cpp"},
          {"directory":".","file":"src/bad.cpp","command":"echo fake && cc -I\"include space\""}
        ]"#,
        ),
    ]);
    let result = analyze(&project.0, |_| {}).unwrap();
    assert_eq!(result.total_edges, 3);
    let quoted = result
        .modules
        .iter()
        .find(|m| m.id == "src/quoted.cpp")
        .unwrap();
    assert_eq!(
        quoted.imports,
        vec!["include space/item.h", "quotes/item.h"]
    );
    assert_eq!(result.unresolved_count, 1);
}

#[test]
fn external_include_directories_and_ignored_headers_cannot_be_silently_bypassed() {
    let project = Project::new(&[
        ("local/item.h", "#pragma once\n"),
        ("ignored/item.h", "#pragma once\n"),
        (".repotowerignore", "ignored/\n"),
        ("external.c", "#include <item.h>\n"),
        ("ignored.c", "#include <item.h>\n"),
        ("local/main.c", "#include \"item.h\"\n"),
        (
            "compile_commands.json",
            r#"[
          {"directory":".","file":"external.c","arguments":["cc","-I../outside","-Ilocal"]},
          {"directory":".","file":"ignored.c","arguments":["cc","-Iignored","-Ilocal"]},
          {"directory":".","file":"local/main.c","arguments":["cc","-I../outside"]}
        ]"#,
        ),
    ]);
    let result = analyze(&project.0, |_| {}).unwrap();
    assert_eq!((result.total_edges, result.unresolved_count), (1, 2));
    assert_eq!(
        (&*result.edges[0].source, &*result.edges[0].target),
        ("local/main.c", "local/item.h")
    );
}

#[test]
fn headers_do_not_inherit_an_arbitrary_translation_unit_profile() {
    let project = Project::new(&[
        ("include/api.h", "#include <detail.h>\n"),
        ("include/detail.h", "#pragma once\n"),
        ("main.c", "#include <api.h>\n"),
        (
            "compile_commands.json",
            r#"[{"directory":".","file":"main.c","arguments":["cc","-isystem","include"]}]"#,
        ),
    ]);
    let result = analyze(&project.0, |_| {}).unwrap();
    assert_eq!((result.total_edges, result.unresolved_count), (1, 1));
    assert_eq!(result.edges[0].target, "include/api.h");
}

#[test]
fn absolute_compile_database_paths_remain_project_confined() {
    let project = Project::new(&[
        ("include/item.h", "#pragma once\n"),
        ("main.c", "#include <item.h>\n"),
    ]);
    let root = project.0.canonicalize().unwrap();
    let database = serde_json::json!([{
        "directory":root,
        "file":root.join("main.c"),
        "arguments":["cc","-I",root.join("include")]
    }]);
    fs::write(
        project.0.join("compile_commands.json"),
        database.to_string(),
    )
    .unwrap();
    let result = analyze(&project.0, |_| {}).unwrap();
    assert_eq!((result.total_edges, result.unresolved_count), (1, 0));
    assert_eq!(result.edges[0].target, "include/item.h");
}
