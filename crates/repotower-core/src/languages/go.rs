//! Go import paths identify packages. A package edge therefore conservatively
//! includes every scanned production file in that package, never its test files.
use super::{Context, Extracted, ResolvedImport};
use crate::{model::DependencyKind, resolver::Resolution};
use std::collections::{HashMap, HashSet};
use tree_sitter::{Node, Parser};

#[derive(Default)]
struct FileInfo {
    package: String,
    imports: Vec<Result<String, String>>,
    declarations: HashSet<String>,
    references: HashSet<String>,
    shadows: HashSet<String>,
    has_errors: bool,
}
struct Module {
    directory: String,
    name: String,
}
pub struct Analyzer<'a> {
    _context: &'a Context<'a>,
    files: HashMap<String, FileInfo>,
    modules: Vec<Module>,
    packages: HashMap<String, Vec<String>>,
    symbols: HashMap<(String, String, String), Vec<String>>,
}

impl<'a> Analyzer<'a> {
    pub fn new(context: &'a Context<'a>) -> Result<Self, String> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_go::LANGUAGE.into())
            .map_err(|e| e.to_string())?;
        let mut files = HashMap::new();
        let mut directories = HashSet::new();
        for (id, source) in context
            .sources
            .iter()
            .filter(|(id, _)| id.to_ascii_lowercase().ends_with(".go"))
        {
            let tree = parser
                .parse(source, None)
                .ok_or("Go parser returned no syntax tree")?;
            let mut info = FileInfo {
                has_errors: tree.root_node().has_error(),
                ..Default::default()
            };
            inspect(tree.root_node(), source, false, &mut info);
            files.insert(id.clone(), info);
            let mut directory = parent(id);
            loop {
                directories.insert(directory.to_string());
                if directory.is_empty() {
                    break;
                }
                directory = parent(directory);
            }
        }
        let mut modules = Vec::new();
        for directory in directories {
            if let Some(config) = context.read_config(&join(&directory, "go.mod")) {
                if let Some(name) = module_name(&config) {
                    modules.push(Module { directory, name });
                }
            }
        }
        modules.sort_by(|a, b| {
            b.name
                .len()
                .cmp(&a.name.len())
                .then_with(|| a.directory.cmp(&b.directory))
        });
        let mut packages: HashMap<String, Vec<String>> = HashMap::new();
        let mut symbols: HashMap<(String, String, String), Vec<String>> = HashMap::new();
        for (id, info) in &files {
            if !id.ends_with("_test.go") && !info.package.is_empty() {
                packages
                    .entry(parent(id).into())
                    .or_default()
                    .push(id.clone());
            }
            for name in &info.declarations {
                symbols
                    .entry((parent(id).into(), info.package.clone(), name.clone()))
                    .or_default()
                    .push(id.clone());
            }
        }
        for files in packages.values_mut() {
            files.sort();
        }
        Ok(Self {
            _context: context,
            files,
            modules,
            packages,
            symbols,
        })
    }

    pub fn analyze(
        &mut self,
        importer: &str,
        _extension: &str,
        _source: &str,
    ) -> Result<Extracted, String> {
        let info = self.files.get(importer).ok_or("Go file was not indexed")?;
        let mut imports = Vec::new();
        for specifier in &info.imports {
            match specifier {
                Err(raw) => imports.push(resolved(
                    raw,
                    DependencyKind::Static,
                    Resolution::Unresolved("Go import string could not be decoded."),
                )),
                Ok(specifier) => {
                    for resolution in self.resolve_package(specifier) {
                        if !matches!(&resolution, Resolution::Internal(id) if id == importer) {
                            imports.push(resolved(specifier, DependencyKind::Static, resolution));
                        }
                    }
                }
            }
        }
        // Package-level names do not need imports. Use only uniquely declared
        // names and suppress any name shadowed locally in this file. This is a
        // conservative lexical index, not Go's type checker or method resolver.
        for name in info.references.difference(&info.shadows) {
            if info.declarations.contains(name) {
                continue;
            }
            let key = (
                parent(importer).to_string(),
                info.package.clone(),
                name.clone(),
            );
            if let Some(candidates) = self.symbols.get(&key) {
                let candidates: Vec<_> = candidates
                    .iter()
                    .filter(|id| importer.ends_with("_test.go") || !id.ends_with("_test.go"))
                    .collect();
                if candidates.len() == 1 && candidates[0] != importer {
                    imports.push(resolved(
                        &format!("{}::{name}", info.package),
                        DependencyKind::Use,
                        Resolution::Internal(candidates[0].clone()),
                    ));
                } else if candidates.len() > 1 {
                    imports.push(resolved(
                        &format!("{}::{name}", info.package),
                        DependencyKind::Use,
                        Resolution::Unresolved(
                            "Go package symbol has multiple possible declaration files.",
                        ),
                    ));
                }
            }
        }
        Ok(Extracted {
            imports,
            has_errors: info.has_errors,
        })
    }

    fn resolve_package(&self, specifier: &str) -> Vec<Resolution> {
        if specifier.starts_with('.')
            || specifier.starts_with('/')
            || specifier.contains(['\\', '\0'])
        {
            return vec![Resolution::Unresolved(
                "Go relative or absolute imports are not resolved in module mode.",
            )];
        }
        let matching: Vec<_> = self
            .modules
            .iter()
            .filter(|module| {
                specifier == module.name
                    || specifier
                        .strip_prefix(&module.name)
                        .is_some_and(|tail| tail.starts_with('/'))
            })
            .collect();
        let Some(first) = matching.first() else {
            return vec![Resolution::External];
        };
        let longest = first.name.len();
        let matching: Vec<_> = matching
            .into_iter()
            .filter(|module| module.name.len() == longest)
            .collect();
        if matching.len() != 1 {
            return vec![Resolution::Unresolved(
                "Go module path matches multiple local modules.",
            )];
        }
        let module = matching[0];
        let suffix = specifier[module.name.len()..].trim_start_matches('/');
        if suffix.split('/').any(|part| matches!(part, "." | "..")) {
            return vec![Resolution::Unresolved(
                "Go import path contains an invalid path component.",
            )];
        }
        let directory = join(&module.directory, suffix);
        // A nested go.mod owns its subtree even when its module name differs.
        if self.modules.iter().any(|nested| {
            nested.directory != module.directory
                && within(&nested.directory, &module.directory)
                && within(&directory, &nested.directory)
        }) {
            return vec![Resolution::Unresolved(
                "Go import crosses a nested module boundary.",
            )];
        }
        let Some(files) = self.packages.get(&directory) else {
            return vec![Resolution::Unresolved(
                "Go local package has no scanned production source files.",
            )];
        };
        let package_names: HashSet<_> = files
            .iter()
            .filter_map(|id| self.files.get(id).map(|info| info.package.as_str()))
            .collect();
        if package_names.len() != 1 {
            return vec![Resolution::Unresolved(
                "Go directory contains multiple production package names.",
            )];
        }
        files.iter().cloned().map(Resolution::Internal).collect()
    }
}

fn inspect(node: Node<'_>, source: &str, local: bool, info: &mut FileInfo) {
    let text = |node: Node<'_>| node.utf8_text(source.as_bytes()).unwrap_or("").to_string();
    if node.kind() == "package_clause" {
        let mut cursor = node.walk();
        if let Some(name) = node
            .named_children(&mut cursor)
            .find(|child| child.kind() == "package_identifier")
        {
            info.package = text(name);
        }
        return;
    }
    if node.kind() == "import_spec" {
        if let Some(path) = node.child_by_field_name("path") {
            let raw = text(path);
            let decoded = unquote(&raw);
            if node.child_by_field_name("name").is_none() {
                if let Some(name) = decoded.as_deref().and_then(|path| path.rsplit('/').next()) {
                    info.shadows.insert(name.to_string());
                }
            }
            info.imports.push(decoded.ok_or(raw));
        }
        if let Some(name) = node.child_by_field_name("name") {
            info.shadows.insert(text(name));
        }
        return;
    }
    if matches!(
        node.kind(),
        "function_declaration" | "type_spec" | "type_alias" | "var_spec" | "const_spec"
    ) {
        let mut cursor = node.walk();
        for name in node
            .children_by_field_name("name", &mut cursor)
            .filter(|child| child.is_named())
        {
            if local {
                info.shadows.insert(text(name));
            } else {
                info.declarations.insert(text(name));
            }
        }
    }
    if matches!(
        node.kind(),
        "parameter_declaration" | "variadic_parameter_declaration" | "type_parameter_declaration"
    ) {
        let mut cursor = node.walk();
        for name in node.children_by_field_name("name", &mut cursor) {
            info.shadows.insert(text(name));
        }
    }
    if node.kind() == "short_var_declaration" || node.kind() == "range_clause" {
        if let Some(left) = node.child_by_field_name("left") {
            let mut cursor = left.walk();
            for name in left
                .named_children(&mut cursor)
                .filter(|child| child.kind() == "identifier")
            {
                info.shadows.insert(text(name));
            }
        }
    }
    if matches!(node.kind(), "identifier" | "type_identifier") {
        let parent = node.parent();
        let is_name = parent.is_some_and(|parent| {
            let mut cursor = parent.walk();
            let found = parent
                .children_by_field_name("name", &mut cursor)
                .any(|name| name == node);
            found
        });
        if !is_name {
            info.references.insert(text(node));
        }
    }
    // Fields after a selector and struct field names are not package symbols.
    if matches!(
        node.kind(),
        "comment" | "interpreted_string_literal" | "raw_string_literal"
    ) {
        return;
    }
    let local = local
        || matches!(
            node.kind(),
            "function_declaration" | "method_declaration" | "func_literal" | "block"
        );
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        inspect(child, source, local, info);
    }
}

fn module_name(config: &str) -> Option<String> {
    let mut block_comment = false;
    for line in config.lines() {
        let mut cleaned = String::new();
        let mut rest = line;
        while !rest.is_empty() {
            if block_comment {
                let Some(end) = rest.find("*/") else {
                    break;
                };
                rest = &rest[end + 2..];
                block_comment = false;
            } else if rest.starts_with("//") {
                break;
            } else if rest.starts_with("/*") {
                block_comment = true;
                rest = &rest[2..];
            } else {
                let ch = rest.chars().next()?;
                cleaned.push(ch);
                rest = &rest[ch.len_utf8()..];
            }
        }
        let mut parts = cleaned.split_whitespace();
        if parts.next() == Some("module") {
            let value = parts.next()?;
            let value = if value.starts_with(['"', '`']) {
                unquote(value)?
            } else {
                value.into()
            };
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}
fn unquote(value: &str) -> Option<String> {
    if value.starts_with('`') && value.ends_with('`') && value.len() >= 2 {
        Some(value[1..value.len() - 1].replace('\r', ""))
    } else {
        serde_json::from_str(value).ok()
    }
}
fn resolved(specifier: &str, kind: DependencyKind, resolution: Resolution) -> ResolvedImport {
    ResolvedImport {
        specifier: specifier.into(),
        kind,
        resolution,
    }
}
fn parent(path: &str) -> &str {
    path.rsplit_once('/')
        .map(|(parent, _)| parent)
        .unwrap_or("")
}
fn join(directory: &str, name: &str) -> String {
    if directory.is_empty() {
        name.into()
    } else if name.is_empty() {
        directory.into()
    } else {
        format!("{directory}/{name}")
    }
}
fn within(path: &str, directory: &str) -> bool {
    directory.is_empty()
        || path == directory
        || path
            .strip_prefix(directory)
            .is_some_and(|tail| tail.starts_with('/'))
}
