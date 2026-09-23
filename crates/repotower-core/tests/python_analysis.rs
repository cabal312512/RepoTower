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
            .join("../../.tmp/python-tests")
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

fn imports<'a>(analysis: &'a RepositoryAnalysis, id: &str) -> Vec<&'a str> {
    analysis
        .modules
        .iter()
        .find(|module| module.id == id)
        .unwrap()
        .imports
        .iter()
        .map(String::as_str)
        .collect()
}

#[test]
fn relative_imports_load_packages_and_produce_real_cascade_waves() {
    let result = Project::new(&[
        ("pkg/__init__.py", "VERSION = 1\n"),
        ("pkg/base.py", "value = 42\n"),
        (
            "pkg/service.py",
            "from .base import value as base_value\ndef load(): return base_value\n",
        ),
        (
            "main.py",
            "from pkg.service import (\n    load as start,\n)\n",
        ),
        ("unrelated.py", "value = 0\n"),
    ])
    .analyze();
    assert_eq!(
        imports(&result, "pkg/service.py"),
        vec!["pkg/__init__.py", "pkg/base.py"]
    );
    assert_eq!(
        imports(&result, "main.py"),
        vec!["pkg/__init__.py", "pkg/service.py"]
    );
    assert_eq!(result.unresolved_count, 0);
    assert_eq!(
        impact(&result.modules, "pkg/base.py", &[]).unwrap().waves,
        vec![vec!["pkg/base.py"], vec!["pkg/service.py"], vec!["main.py"]]
    );
    assert_eq!(
        impact(&result.modules, "pkg/__init__.py", &[])
            .unwrap()
            .total_affected,
        2
    );
}

#[test]
fn src_layout_nested_packages_and_python_stubs_are_resolved() {
    let result = Project::new(&[
        ("src/tool/__init__.py", ""),
        ("src/tool/core/__init__.py", ""),
        ("src/tool/core/value.py", "value = 1\n"),
        ("src/tool/core/value.pyi", "value: int\n"),
        ("src/tool/api.pyi", "from .core.value import value\n"),
        (
            "main.py",
            "import tool.core.value as value\nfrom tool.api import call\n",
        ),
    ])
    .analyze();
    assert_eq!(
        imports(&result, "main.py"),
        vec![
            "src/tool/__init__.py",
            "src/tool/api.pyi",
            "src/tool/core/__init__.py",
            "src/tool/core/value.py"
        ]
    );
    assert_eq!(
        imports(&result, "src/tool/api.pyi"),
        vec![
            "src/tool/__init__.py",
            "src/tool/core/__init__.py",
            "src/tool/core/value.py"
        ]
    );
    assert_eq!(result.unresolved_count, 0);
}

#[test]
fn package_values_do_not_turn_into_same_named_file_dependencies() {
    let result = Project::new(&[
        (
            "pkg/__init__.py",
            "class Settings: pass\nhelper = 7\nfrom . import child\nfrom .origin import exported\n",
        ),
        ("pkg/Settings.py", "# Not the class exported by pkg\n"),
        ("pkg/helper.py", "# Not the value exported by pkg\n"),
        ("pkg/child.py", "child = 1\n"),
        ("pkg/origin.py", "exported = 4\n"),
        ("pkg/worker.py", "value = 9\n"),
        (
            "main.py",
            "from pkg import Settings, helper, child, exported, worker\n",
        ),
    ])
    .analyze();
    assert_eq!(
        imports(&result, "main.py"),
        vec!["pkg/__init__.py", "pkg/worker.py"]
    );
    assert_eq!(
        imports(&result, "pkg/__init__.py"),
        vec!["pkg/child.py", "pkg/origin.py"]
    );
    assert_eq!(result.unresolved_count, 0);
    assert_eq!(result.cycle_count, 0);
    assert_eq!(
        impact(&result.modules, "pkg/helper.py", &[])
            .unwrap()
            .total_affected,
        0
    );
    assert_eq!(
        impact(&result.modules, "pkg/child.py", &[]).unwrap().waves,
        vec![
            vec!["pkg/child.py"],
            vec!["pkg/__init__.py"],
            vec!["main.py"]
        ]
    );
}

#[test]
fn namespace_packages_can_span_supported_source_roots() {
    let result = Project::new(&[
        ("ns/left.py", "value = 1\n"),
        ("src/ns/right.py", "value = 2\n"),
        ("main.py", "from ns import left, right\nimport ns.left\n"),
    ])
    .analyze();
    assert_eq!(
        imports(&result, "main.py"),
        vec!["ns/left.py", "src/ns/right.py"]
    );
    assert_eq!((result.unresolved_count, result.external_count), (0, 0));
}

#[test]
fn ambiguous_roots_ignored_targets_and_package_boundary_are_reported() {
    let result = Project::new(&[
        (".repotowerignore", "pkg/hidden.py\n"),
        ("common.py", "value = 1\n"),
        ("src/common.py", "value = 2\n"),
        ("pkg/__init__.py", ""),
        ("pkg/hidden.py", "secret = 1\n"),
        ("pkg/main.py", "import common\nfrom .hidden import secret\nfrom ..outside import value\nfrom . import missing\n"),
    ]).analyze();
    assert_eq!(result.unresolved_count, 4);
    assert_eq!(imports(&result, "pkg/main.py"), vec!["pkg/__init__.py"]);
    assert!(result
        .modules
        .iter()
        .all(|module| module.id != "pkg/hidden.py"));
    assert!(result
        .modules
        .iter()
        .find(|module| module.id == "pkg/main.py")
        .unwrap()
        .unresolved_imports
        .iter()
        .any(|import| import.reason.contains("sys.path")));
}

#[test]
fn comments_strings_and_external_modules_do_not_invent_edges() {
    let result = Project::new(&[
        ("main.py", "# import ghost\ntext = '''from fake import ghost'''\nimport os.path, requests as http\nfrom __future__ import annotations\ndef later():\n    import actual as alias\n"),
        ("ghost.py", ""),
        ("fake.py", ""),
        ("__future__.py", ""),
        ("actual.py", ""),
    ]).analyze();
    assert_eq!(imports(&result, "main.py"), vec!["actual.py"]);
    assert_eq!(result.external_count, 3);
    assert_eq!(result.unresolved_count, 0);
}

#[test]
fn cycles_and_partial_syntax_errors_keep_valid_dependencies() {
    let result = Project::new(&[
        ("a.py", "import b\nx =\n"),
        ("b.py", "import a\n"),
        ("entry.py", "from a import value\n"),
    ])
    .analyze();
    assert_eq!(result.cycle_count, 1);
    assert_eq!(
        impact(&result.modules, "b.py", &[]).unwrap().waves,
        vec![vec!["b.py"], vec!["a.py"], vec!["entry.py"]]
    );
    assert!(result
        .warnings
        .iter()
        .any(|warning| warning.contains("a.py") && warning.contains("解析错误")));
}

#[test]
fn dynamic_package_exports_are_reported_instead_of_guessed() {
    let result = Project::new(&[
        ("pkg/__init__.py", "def __getattr__(name):\n    return 42\n"),
        ("pkg/value.py", "value = 7\n"),
        ("main.py", "from pkg import value\n"),
    ])
    .analyze();
    assert_eq!(imports(&result, "main.py"), vec!["pkg/__init__.py"]);
    assert_eq!(result.unresolved_count, 1);
    assert!(result
        .modules
        .iter()
        .find(|module| module.id == "main.py")
        .unwrap()
        .unresolved_imports[0]
        .reason
        .contains("__getattr__"));
}

#[test]
fn wildcard_imports_use_literal_all_instead_of_every_file_in_the_package() {
    let result = Project::new(&[
        (
            "pkg/__init__.py",
            "__all__ = ['worker', 'value']\nvalue = 3\n",
        ),
        ("pkg/worker.py", ""),
        (
            "pkg/value.py",
            "# A value in __init__, not this submodule\n",
        ),
        ("pkg/unused.py", ""),
        ("main.py", "from pkg import *\n"),
        ("dynamic/__init__.py", "__all__ = make_names()\n"),
        ("dynamic_main.py", "from dynamic import *\n"),
    ])
    .analyze();
    assert_eq!(
        imports(&result, "main.py"),
        vec!["pkg/__init__.py", "pkg/worker.py"]
    );
    assert_eq!(result.unresolved_count, 1);
    assert!(result
        .modules
        .iter()
        .find(|module| module.id == "dynamic_main.py")
        .unwrap()
        .unresolved_imports[0]
        .reason
        .contains("__all__"));
}

#[test]
fn initialized_src_package_is_not_mistaken_for_an_import_search_root() {
    let result = Project::new(&[
        ("src/__init__.py", "from . import core\n"),
        ("src/core.py", "value = 1\n"),
        ("main.py", "from src import core\n"),
    ])
    .analyze();
    assert_eq!(imports(&result, "src/__init__.py"), vec!["src/core.py"]);
    assert_eq!(imports(&result, "main.py"), vec!["src/__init__.py"]);
    assert_eq!(result.unresolved_count, 0);
}

#[test]
fn annotation_only_runtime_exports_do_not_hide_a_real_submodule() {
    let result = Project::new(&[
        ("pkg/__init__.py", "config: object\n"),
        ("pkg/config.py", "value = 1\n"),
        ("main.py", "from pkg import config\n"),
        ("typed/__init__.pyi", "config: object\n"),
        ("typed/config.py", ""),
        ("typed_main.py", "from typed import config\n"),
    ])
    .analyze();
    assert_eq!(
        imports(&result, "main.py"),
        vec!["pkg/__init__.py", "pkg/config.py"]
    );
    assert_eq!(
        imports(&result, "typed_main.py"),
        vec!["typed/__init__.pyi"]
    );
    assert_eq!(result.unresolved_count, 0);
}
