//! Resolve the declared Rust module tree without running Cargo, build scripts or
//! macros. `use` edges point at the longest known module prefix of an item path.
use super::{Context, Extracted, ResolvedImport};
use crate::{model::DependencyKind, resolver::Resolution};
use std::collections::{HashMap, HashSet};
use tree_sitter::{Node, Parser, Tree};

type ModuleKey = (String, Vec<String>);
struct UsePath {
    file: String,
    root: String,
    scope: Vec<String>,
    path: Vec<String>,
}
#[derive(Clone)]
struct Location {
    file: String,
    root: String,
    scope: Vec<String>,
    children_dir: String,
    attribute_dir: String,
}
pub struct Analyzer<'a> {
    context: &'a Context<'a>,
    trees: HashMap<String, Tree>,
    modules: HashMap<ModuleKey, Vec<String>>,
    symbols: HashMap<ModuleKey, HashSet<String>>,
    uses: Vec<UsePath>,
    imports: HashMap<String, Vec<ResolvedImport>>,
    visited: HashSet<(String, Vec<String>, String)>,
    libraries: HashMap<String, Vec<String>>,
}

impl<'a> Analyzer<'a> {
    pub fn new(context: &'a Context<'a>) -> Result<Self, String> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .map_err(|error| error.to_string())?;
        let mut trees = HashMap::new();
        for (id, source) in context
            .sources
            .iter()
            .filter(|(id, _)| id.to_ascii_lowercase().ends_with(".rs"))
        {
            trees.insert(
                id.clone(),
                parser
                    .parse(source, None)
                    .ok_or("Rust parser returned no syntax tree")?,
            );
        }
        let mut analyzer = Self {
            context,
            trees,
            modules: HashMap::new(),
            symbols: HashMap::new(),
            uses: vec![],
            imports: HashMap::new(),
            visited: HashSet::new(),
            libraries: HashMap::new(),
        };
        let mut roots: Vec<_> = analyzer
            .trees
            .keys()
            .filter(|id| is_crate_root(id))
            .cloned()
            .collect();
        roots.sort();
        for root in &roots {
            if root.ends_with("/lib.rs") || root == "lib.rs" {
                if let Some(name) = analyzer.package_name(root) {
                    analyzer
                        .libraries
                        .entry(name)
                        .or_default()
                        .push(root.clone());
                }
            }
            analyzer.walk_root(root);
        }
        // A selected folder may itself contain standalone Rust files. Keep
        // those nodes useful while avoiding guesses based on file basenames.
        let claimed: HashSet<_> = analyzer.modules.values().flatten().cloned().collect();
        let mut unclaimed: Vec<_> = analyzer
            .trees
            .keys()
            .filter(|id| !claimed.contains(*id))
            .cloned()
            .collect();
        unclaimed.sort();
        for root in unclaimed {
            if !analyzer.modules.values().any(|files| files.contains(&root)) {
                analyzer.walk_root(&root);
            }
        }
        let uses = std::mem::take(&mut analyzer.uses);
        for usage in uses {
            let resolution = analyzer.resolve_use(&usage);
            if !matches!(&resolution, Resolution::Internal(file) if file == &usage.file) {
                analyzer
                    .imports
                    .entry(usage.file)
                    .or_default()
                    .push(ResolvedImport {
                        specifier: usage.path.join("::"),
                        kind: DependencyKind::Use,
                        resolution,
                    });
            }
        }
        Ok(analyzer)
    }

    pub fn analyze(
        &mut self,
        importer: &str,
        _extension: &str,
        _source: &str,
    ) -> Result<Extracted, String> {
        let tree = self
            .trees
            .get(importer)
            .ok_or("Rust file was not indexed")?;
        // Each source is requested once by the repository's second pass.
        Ok(Extracted {
            imports: self.imports.remove(importer).unwrap_or_default(),
            has_errors: tree.root_node().has_error(),
        })
    }

    fn walk_root(&mut self, file: &str) {
        let location = Location {
            file: file.into(),
            root: file.into(),
            scope: vec![],
            children_dir: parent(file).into(),
            attribute_dir: parent(file).into(),
        };
        self.walk_file(location, &[]);
    }

    fn walk_file(&mut self, location: Location, ancestors: &[String]) {
        self.register_module(&location);
        if ancestors.contains(&location.file)
            || location.scope.len() > 64
            || self.visited.len() > 50_000
        {
            return;
        }
        if !self.visited.insert((
            location.root.clone(),
            location.scope.clone(),
            location.file.clone(),
        )) {
            return;
        }
        let Some(tree) = self.trees.get(&location.file).cloned() else {
            return;
        };
        let mut ancestors = ancestors.to_vec();
        ancestors.push(location.file.clone());
        self.walk_body(tree.root_node(), &location, &ancestors);
    }

    fn register_module(&mut self, location: &Location) {
        let files = self
            .modules
            .entry((location.root.clone(), location.scope.clone()))
            .or_default();
        if !files.contains(&location.file) {
            files.push(location.file.clone());
        }
    }

    fn walk_body(&mut self, body: Node<'_>, location: &Location, ancestors: &[String]) {
        if location.scope.len() > 64 {
            return;
        }
        let source = &self.context.sources[&location.file];
        let mut path_attribute: Option<Result<String, &'static str>> = None;
        let mut cursor = body.walk();
        for node in body.named_children(&mut cursor) {
            if node.kind() == "attribute_item" {
                if let Some(path) = read_path_attribute(node, source) {
                    path_attribute = Some(path);
                }
                continue;
            }
            if matches!(node.kind(), "line_comment" | "block_comment") {
                continue;
            }
            if node.kind() == "mod_item" {
                let Some(name) = node
                    .child_by_field_name("name")
                    .map(|name| node_text(name, source).trim_start_matches("r#").to_string())
                else {
                    continue;
                };
                let mut scope = location.scope.clone();
                scope.push(name.clone());
                if let Some(inline) = node.child_by_field_name("body") {
                    let directory = match path_attribute.take() {
                        Some(Ok(path)) => normalize(&location.attribute_dir, &path),
                        Some(Err(_)) => None,
                        None => Some(join(&location.children_dir, &name)),
                    };
                    if let Some(directory) = directory {
                        let nested = Location {
                            scope,
                            children_dir: directory.clone(),
                            attribute_dir: directory,
                            ..location.clone()
                        };
                        self.register_module(&nested);
                        self.walk_body(inline, &nested, ancestors);
                    } else {
                        self.add_unresolved(
                            location,
                            &format!("mod {name}"),
                            "Rust module path is outside the selected project or is not a literal.",
                        );
                    }
                } else {
                    let result = match path_attribute.take() {
                        Some(Ok(path)) => match normalize(&location.attribute_dir, &path) {
                            Some(candidate) if self.trees.contains_key(&candidate) => Ok(candidate),
                            Some(_) => Err("Rust module file was not found among scanned sources."),
                            None => Err("Rust module path is outside the selected project or is not a literal."),
                        },
                        Some(Err(reason)) => Err(reason),
                        None => {
                            let base = join(&location.children_dir, &name);
                            let candidates: Vec<_> = [format!("{base}.rs"), format!("{base}/mod.rs")].into_iter().filter(|candidate| self.trees.contains_key(candidate)).collect();
                            match candidates.len() {
                                1 => Ok(candidates[0].clone()),
                                0 => Err("Rust module file was not found among scanned sources."),
                                _ => Err("Rust module has both name.rs and name/mod.rs candidates."),
                            }
                        }
                    };
                    match result {
                        Ok(file) => {
                            if file != location.file {
                                self.imports.entry(location.file.clone()).or_default().push(
                                    ResolvedImport {
                                        specifier: format!("mod {name}"),
                                        kind: DependencyKind::Module,
                                        resolution: Resolution::Internal(file.clone()),
                                    },
                                );
                            }
                            let directory = if file.ends_with("/mod.rs") || file == "mod.rs" {
                                parent(&file).into()
                            } else {
                                file.strip_suffix(".rs").unwrap_or(&file).into()
                            };
                            self.walk_file(
                                Location {
                                    attribute_dir: parent(&file).into(),
                                    children_dir: directory,
                                    file,
                                    root: location.root.clone(),
                                    scope,
                                },
                                ancestors,
                            );
                        }
                        Err(reason) => {
                            self.add_unresolved(location, &format!("mod {name}"), reason)
                        }
                    }
                }
            } else {
                path_attribute = None;
                if node.kind() == "use_declaration" {
                    if let Some(argument) = node.child_by_field_name("argument") {
                        let mut paths = Vec::new();
                        expand_use(argument, source, &[], &mut paths);
                        for path in paths {
                            self.uses.push(UsePath {
                                file: location.file.clone(),
                                root: location.root.clone(),
                                scope: location.scope.clone(),
                                path,
                            });
                        }
                        // A pub re-export is a name defined by this module;
                        // callers depend on its containing file in the graph.
                        let mut names = Vec::new();
                        collect_use_names(argument, source, &mut names);
                        self.symbols
                            .entry((location.root.clone(), location.scope.clone()))
                            .or_default()
                            .extend(names);
                    }
                } else if matches!(
                    node.kind(),
                    "function_item"
                        | "struct_item"
                        | "enum_item"
                        | "type_item"
                        | "trait_item"
                        | "const_item"
                        | "static_item"
                        | "union_item"
                ) {
                    if let Some(name) = node.child_by_field_name("name") {
                        self.symbols
                            .entry((location.root.clone(), location.scope.clone()))
                            .or_default()
                            .insert(node_text(name, source).trim_start_matches("r#").into());
                    }
                    // Function-local `use` follows the enclosing module scope.
                    self.collect_nested_uses(node, source, location);
                } else if !matches!(node.kind(), "macro_definition" | "macro_invocation") {
                    self.collect_nested_uses(node, source, location);
                }
            }
        }
    }

    fn collect_nested_uses(&mut self, node: Node<'_>, source: &str, location: &Location) {
        if matches!(
            node.kind(),
            "macro_definition" | "macro_invocation" | "mod_item"
        ) {
            return;
        }
        if node.kind() == "use_declaration" {
            if let Some(argument) = node.child_by_field_name("argument") {
                let mut paths = Vec::new();
                expand_use(argument, source, &[], &mut paths);
                for path in paths {
                    self.uses.push(UsePath {
                        file: location.file.clone(),
                        root: location.root.clone(),
                        scope: location.scope.clone(),
                        path,
                    });
                }
            }
            return;
        }
        if matches!(node.kind(), "scoped_identifier" | "scoped_type_identifier") {
            let path = path_segments(node, source);
            if path
                .first()
                .is_some_and(|segment| matches!(segment.as_str(), "crate" | "self" | "super"))
            {
                self.uses.push(UsePath {
                    file: location.file.clone(),
                    root: location.root.clone(),
                    scope: location.scope.clone(),
                    path,
                });
                // The outer path already contains its prefixes. Do not create
                // additional shorter edges for the same expression or type.
                return;
            }
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.collect_nested_uses(child, source, location);
        }
    }

    fn resolve_use(&self, usage: &UsePath) -> Resolution {
        let mut parts = usage.path.clone();
        if parts.last().is_some_and(|part| part == "*") {
            parts.pop();
        }
        if parts.last().is_some_and(|part| part == "self") && parts.len() > 1 {
            parts.pop();
        }
        if parts.is_empty() {
            return Resolution::Unresolved("Rust use path could not be resolved.");
        }
        let mut root = usage.root.clone();
        let explicit = matches!(parts[0].as_str(), "crate" | "self" | "super");
        let mut prefix = Vec::new();
        if parts[0] == "crate" {
            parts.remove(0);
        } else if parts[0] == "self" {
            prefix = usage.scope.clone();
            parts.remove(0);
        } else if parts[0] == "super" {
            prefix = usage.scope.clone();
            while parts.first().is_some_and(|part| part == "super") {
                if prefix.pop().is_none() {
                    return Resolution::Unresolved("Rust super path escapes the crate root.");
                }
                parts.remove(0);
            }
        } else {
            // Prefer a declared module in the current lexical module.
            let mut candidate = usage.scope.clone();
            candidate.push(parts[0].clone());
            if self.modules.contains_key(&(root.clone(), candidate)) {
                prefix = usage.scope.clone();
            } else if let Some(libraries) = self.libraries.get(&parts[0]) {
                let local: Vec<_> = libraries
                    .iter()
                    .filter(|library| {
                        self.manifest_directory(library) == self.manifest_directory(&usage.file)
                    })
                    .collect();
                let choices = if local.is_empty() {
                    libraries.iter().collect::<Vec<_>>()
                } else {
                    local
                };
                if choices.len() != 1 {
                    return Resolution::Unresolved(
                        "Rust crate name matches multiple local libraries.",
                    );
                }
                root = choices[0].clone();
                parts.remove(0);
            }
        }
        prefix.extend(parts);
        for length in (0..=prefix.len()).rev() {
            let key = (root.clone(), prefix[..length].to_vec());
            let Some(files) = self.modules.get(&key) else {
                continue;
            };
            if length == 0
                && !prefix.is_empty()
                && !self
                    .symbols
                    .get(&key)
                    .is_some_and(|names| names.contains(&prefix[0]))
            {
                continue;
            }
            if files.len() != 1 {
                return Resolution::Unresolved(
                    "Rust module path has multiple possible source files.",
                );
            }
            return Resolution::Internal(files[0].clone());
        }
        if explicit {
            Resolution::Unresolved("Rust use path was not found in the declared local module tree.")
        } else {
            Resolution::External
        }
    }

    fn add_unresolved(&mut self, location: &Location, specifier: &str, reason: &'static str) {
        self.imports
            .entry(location.file.clone())
            .or_default()
            .push(ResolvedImport {
                specifier: specifier.into(),
                kind: DependencyKind::Module,
                resolution: Resolution::Unresolved(reason),
            });
    }
    fn manifest_directory(&self, file: &str) -> String {
        let mut directory = parent(file);
        loop {
            if self
                .context
                .read_config(&join(directory, "Cargo.toml"))
                .is_some()
            {
                return directory.into();
            }
            if directory.is_empty() {
                return String::new();
            }
            directory = parent(directory);
        }
    }
    fn package_name(&self, root: &str) -> Option<String> {
        let directory = self.manifest_directory(root);
        let config = self.context.read_config(&join(&directory, "Cargo.toml"))?;
        let mut section = "";
        let mut package = None;
        let mut library = None;
        for line in config.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                section = line;
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            if key.trim() != "name" {
                continue;
            }
            let value = value.trim();
            let name = if value.starts_with('"') {
                value[1..].split('"').next()
            } else if value.starts_with('\'') {
                value[1..].split('\'').next()
            } else {
                None
            };
            match section {
                "[package]" => package = name.map(str::to_string),
                "[lib]" => library = name.map(str::to_string),
                _ => {}
            }
        }
        library.or(package).map(|name| name.replace('-', "_"))
    }
}

fn expand_use(node: Node<'_>, source: &str, prefix: &[String], output: &mut Vec<Vec<String>>) {
    match node.kind() {
        "use_as_clause" => {
            if let Some(path) = node.child_by_field_name("path") {
                expand_use(path, source, prefix, output);
            }
        }
        "scoped_use_list" => {
            let mut prefix = prefix.to_vec();
            if let Some(path) = node.child_by_field_name("path") {
                prefix.extend(path_segments(path, source));
            }
            if let Some(list) = node.child_by_field_name("list") {
                expand_use(list, source, &prefix, output);
            }
        }
        "use_list" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                expand_use(child, source, prefix, output);
            }
        }
        "use_wildcard" => {
            let mut path = prefix.to_vec();
            if let Some(child) = node.named_child(0) {
                path.extend(path_segments(child, source));
            }
            path.push("*".into());
            output.push(path);
        }
        "line_comment" | "block_comment" => {}
        _ => {
            let mut path = prefix.to_vec();
            path.extend(path_segments(node, source));
            if !path.is_empty() {
                output.push(path);
            }
        }
    }
}
fn path_segments(node: Node<'_>, source: &str) -> Vec<String> {
    if matches!(
        node.kind(),
        "identifier" | "type_identifier" | "crate" | "self" | "super"
    ) {
        return vec![node_text(node, source).trim_start_matches("r#").into()];
    }
    let mut path = Vec::new();
    if let Some(prefix) = node.child_by_field_name("path") {
        path.extend(path_segments(prefix, source));
    }
    if let Some(name) = node.child_by_field_name("name") {
        path.extend(path_segments(name, source));
    }
    path
}
fn collect_use_names(node: Node<'_>, source: &str, output: &mut Vec<String>) {
    if node.kind() == "use_as_clause" {
        if let Some(alias) = node.child_by_field_name("alias") {
            output.push(node_text(alias, source).into());
        }
        return;
    }
    if matches!(node.kind(), "identifier" | "scoped_identifier") {
        if let Some(last) = path_segments(node, source).last() {
            output.push(last.clone());
        }
        return;
    }
    if matches!(
        node.kind(),
        "use_wildcard" | "line_comment" | "block_comment"
    ) {
        return;
    }
    if let Some(list) = node.child_by_field_name("list") {
        collect_use_names(list, source, output);
        return;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_use_names(child, source, output);
    }
}
fn read_path_attribute(node: Node<'_>, source: &str) -> Option<Result<String, &'static str>> {
    let raw = node_text(node, source).trim();
    let raw = raw.strip_prefix("#[")?.strip_suffix(']')?.trim();
    let (name, value) = raw.split_once('=')?;
    if name.trim() != "path" {
        return None;
    }
    let value = value.trim();
    let parsed = if value.starts_with('r') {
        let first = value.find('"');
        let last = value.rfind('"');
        match (first, last) {
            (Some(first), Some(last)) if first < last => Some(value[first + 1..last].to_string()),
            _ => None,
        }
    } else {
        serde_json::from_str::<String>(value).ok()
    };
    Some(parsed.ok_or("Rust module path is outside the selected project or is not a literal."))
}
fn normalize(directory: &str, relative: &str) -> Option<String> {
    if relative.starts_with(['/', '\\']) || relative.contains([':', '\0', '\\']) {
        return None;
    }
    let mut parts: Vec<_> = directory
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    for part in relative.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            _ => parts.push(part),
        }
    }
    Some(parts.join("/"))
}
fn is_crate_root(id: &str) -> bool {
    let name = id.rsplit('/').next().unwrap_or(id);
    if matches!(name, "lib.rs" | "main.rs") {
        return true;
    }
    let directory = parent(id);
    matches!(
        directory.rsplit('/').next().unwrap_or(directory),
        "bin" | "tests" | "examples"
    )
}
fn parent(path: &str) -> &str {
    path.rsplit_once('/')
        .map(|(parent, _)| parent)
        .unwrap_or("")
}
fn join(directory: &str, name: &str) -> String {
    if directory.is_empty() {
        name.into()
    } else {
        format!("{directory}/{name}")
    }
}
fn node_text<'a>(node: Node<'_>, source: &'a str) -> &'a str {
    node.utf8_text(source.as_bytes()).unwrap_or("")
}
