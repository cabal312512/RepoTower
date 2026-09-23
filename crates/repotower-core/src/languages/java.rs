//! Java source-level type dependencies, without running a compiler or a build.
use super::{Context, Extracted, ResolvedImport};
use crate::{model::DependencyKind, resolver::Resolution};
use std::collections::{BTreeSet, HashMap, HashSet};
use tree_sitter::{Node, Parser};

const AMBIGUOUS: &str = "Java type has multiple possible source files; build configuration and classpath selection were not guessed.";
const MISSING: &str = "Java project type was not found; it may be ignored, generated, or outside the scanned source set.";

#[derive(Default)]
struct Facts {
    package: String,
    declarations: Vec<(String, String)>,
    imports: Vec<Import>,
    references: BTreeSet<String>,
    local_types: HashSet<String>,
    has_errors: bool,
}
struct Import {
    name: String,
    wildcard: bool,
    is_static: bool,
}

pub struct Analyzer<'a> {
    _context: &'a Context<'a>,
    facts: HashMap<String, Facts>,
    types: HashMap<String, Vec<String>>,
    packages: HashSet<String>,
}

impl<'a> Analyzer<'a> {
    pub fn new(context: &'a Context<'a>) -> Result<Self, String> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
            .map_err(|e| e.to_string())?;
        let mut facts = HashMap::new();
        let mut types: HashMap<String, Vec<String>> = HashMap::new();
        let mut packages = HashSet::new();
        for (id, source) in context.sources {
            if !id.to_ascii_lowercase().ends_with(".java") {
                continue;
            }
            let tree = parser.parse(source, None).ok_or("Java 语法树解析失败。")?;
            let info = inspect(tree.root_node(), source);
            packages.insert(info.package.clone());
            for (qualified, _) in &info.declarations {
                types.entry(qualified.clone()).or_default().push(id.clone());
            }
            facts.insert(id.clone(), info);
        }
        for files in types.values_mut() {
            files.sort();
            files.dedup();
        }
        Ok(Self {
            _context: context,
            facts,
            types,
            packages,
        })
    }

    fn lookup(&self, qualified: &str) -> Option<Resolution> {
        self.types.get(qualified).map(|files| {
            if files.len() == 1 {
                Resolution::Internal(files[0].clone())
            } else {
                Resolution::Unresolved(AMBIGUOUS)
            }
        })
    }

    fn local_name(&self, name: &str) -> bool {
        let mut prefix = name;
        while let Some((parent, _)) = prefix.rsplit_once('.') {
            if self.packages.contains(parent) || self.types.contains_key(parent) {
                return true;
            }
            prefix = parent;
        }
        false
    }

    fn explicit(&self, import: &Import) -> Resolution {
        if let Some(found) = self.lookup(&import.name) {
            return found;
        }
        // A static import may name a field or method rather than a nested type.
        // Only strip the last segment: an arbitrary missing owner is not its parent type.
        if import.is_static && !import.wildcard {
            if let Some((owner, _)) = import.name.rsplit_once('.') {
                if let Some(found) = self.lookup(owner) {
                    return found;
                }
            }
        }
        if self.local_name(&import.name) {
            Resolution::Unresolved(MISSING)
        } else {
            Resolution::External
        }
    }

    fn reference(&self, info: &Facts, reference: &str) -> Option<Resolution> {
        let first = reference.split('.').next().unwrap_or(reference);
        if info.local_types.contains(first) {
            return None;
        }
        if let Some(found) = self.lookup(reference) {
            return Some(found);
        }
        // Explicit imports take precedence over package-on-demand imports.
        let mut explicit = BTreeSet::new();
        for import in &info.imports {
            if import.wildcard {
                continue;
            }
            if import.name.rsplit('.').next() == Some(first) {
                explicit.insert(format!("{}{}", import.name, &reference[first.len()..]));
            }
        }
        if !explicit.is_empty() {
            if explicit.len() != 1 {
                return Some(Resolution::Unresolved(AMBIGUOUS));
            }
            let name = explicit.first().unwrap();
            return self.lookup(name).or_else(|| {
                if self.local_name(name) {
                    Some(Resolution::Unresolved(MISSING))
                } else {
                    None
                }
            });
        }
        let same_package = qualify(&info.package, reference);
        if let Some(found) = self.lookup(&same_package) {
            return Some(found);
        }
        let mut candidates = BTreeSet::new();
        for import in info.imports.iter().filter(|i| i.wildcard) {
            let name = qualify(&import.name, reference);
            if self.types.contains_key(&name) {
                candidates.insert(name);
            }
        }
        if candidates.len() > 1 {
            return Some(Resolution::Unresolved(AMBIGUOUS));
        }
        if let Some(name) = candidates.first() {
            return self.lookup(name);
        }
        // Unknown unqualified types may be java.lang, inherited, generic, or external.
        // Do not manufacture local links solely from matching file basenames.
        if reference.contains('.') && self.local_name(reference) {
            Some(Resolution::Unresolved(MISSING))
        } else {
            None
        }
    }

    pub fn analyze(
        &mut self,
        importer: &str,
        _extension: &str,
        _source: &str,
    ) -> Result<Extracted, String> {
        let info = self.facts.get(importer).ok_or("Java 文件未建立索引。")?;
        let mut imports = Vec::new();
        for import in &info.imports {
            // A package wildcard is a lookup rule, not a dependency on every file.
            // A static wildcard does depend on its owner class.
            if !import.wildcard || import.is_static {
                push(&mut imports, importer, &import.name, self.explicit(import));
            } else if !self.packages.contains(&import.name)
                && !self.types.contains_key(&import.name)
            {
                push(
                    &mut imports,
                    importer,
                    &format!("{}.*", import.name),
                    Resolution::External,
                );
            }
        }
        for reference in &info.references {
            if let Some(resolution) = self.reference(info, reference) {
                push(&mut imports, importer, reference, resolution);
            }
        }
        Ok(Extracted {
            imports,
            has_errors: info.has_errors,
        })
    }
}

fn push(imports: &mut Vec<ResolvedImport>, importer: &str, name: &str, resolution: Resolution) {
    if matches!(&resolution, Resolution::Internal(target) if target == importer) {
        return;
    }
    imports.push(ResolvedImport {
        specifier: name.into(),
        kind: DependencyKind::Static,
        resolution,
    });
}

fn qualify(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.into()
    } else {
        format!("{prefix}.{name}")
    }
}

fn node_text<'a>(node: Node<'_>, source: &'a str) -> &'a str {
    node.utf8_text(source.as_bytes()).unwrap_or("")
}

fn identifiers(node: Node<'_>, source: &str) -> String {
    let mut names = Vec::new();
    let mut nodes = vec![node];
    while let Some(node) = nodes.pop() {
        if node.kind() == "identifier" {
            names.push(node_text(node, source));
            continue;
        }
        let mut cursor = node.walk();
        let children: Vec<_> = node.named_children(&mut cursor).collect();
        nodes.extend(children.into_iter().rev());
    }
    names.join(".")
}

fn type_name(node: Node<'_>, source: &str) -> String {
    let mut depth = 0usize;
    node_text(node, source)
        .chars()
        .filter(|ch| match *ch {
            '<' => {
                depth += 1;
                false
            }
            '>' => {
                depth = depth.saturating_sub(1);
                false
            }
            c => depth == 0 && !c.is_whitespace(),
        })
        .collect()
}

fn is_declaration(kind: &str) -> bool {
    matches!(
        kind,
        "class_declaration"
            | "interface_declaration"
            | "enum_declaration"
            | "record_declaration"
            | "annotation_type_declaration"
    )
}

fn inspect(root: Node<'_>, source: &str) -> Facts {
    let mut info = Facts {
        has_errors: root.has_error(),
        ..Facts::default()
    };
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        match child.kind() {
            "package_declaration" => {
                // Skip package annotations; only the package's direct name child matters.
                let mut c = child.walk();
                if let Some(name) = child
                    .named_children(&mut c)
                    .find(|n| matches!(n.kind(), "identifier" | "scoped_identifier"))
                {
                    info.package = identifiers(name, source);
                };
            }
            "import_declaration" => {
                let mut c = child.walk();
                let is_static = child.children(&mut c).any(|n| n.kind() == "static");
                let mut c = child.walk();
                let wildcard = child.named_children(&mut c).any(|n| n.kind() == "asterisk");
                info.imports.push(Import {
                    name: identifiers(child, source),
                    is_static,
                    wildcard,
                });
            }
            _ => {}
        }
    }
    let mut nodes = vec![root];
    let mut receivers = BTreeSet::new();
    let mut value_names = HashSet::new();
    while let Some(node) = nodes.pop() {
        if matches!(node.kind(), "package_declaration" | "import_declaration") {
            continue;
        }
        if is_declaration(node.kind()) {
            if let Some(name) = node.child_by_field_name("name") {
                let simple = node_text(name, source).to_string();
                info.local_types.insert(simple.clone());
                let mut path = vec![simple.clone()];
                let mut parent = node.parent();
                let mut local = false;
                while let Some(ancestor) = parent {
                    if matches!(
                        ancestor.kind(),
                        "method_declaration"
                            | "constructor_declaration"
                            | "lambda_expression"
                            | "block"
                    ) {
                        local = true;
                    }
                    if is_declaration(ancestor.kind()) {
                        if let Some(name) = ancestor.child_by_field_name("name") {
                            path.push(node_text(name, source).into());
                        }
                    }
                    parent = ancestor.parent();
                }
                if !local {
                    path.reverse();
                    info.declarations
                        .push((qualify(&info.package, &path.join(".")), simple));
                }
            }
        }
        if matches!(
            node.kind(),
            "variable_declarator"
                | "formal_parameter"
                | "spread_parameter"
                | "catch_formal_parameter"
        ) {
            if let Some(name) = node.child_by_field_name("name") {
                value_names.insert(node_text(name, source).to_string());
            }
        }
        match node.kind() {
            "scoped_type_identifier" => {
                let name = type_name(node, source);
                if !generic_in_scope(node, &name, source) {
                    info.references.insert(name);
                }
            }
            "type_identifier" => {
                if node
                    .parent()
                    .map(|p| p.kind() != "scoped_type_identifier")
                    .unwrap_or(true)
                    && !generic_in_scope(node, node_text(node, source), source)
                {
                    info.references.insert(node_text(node, source).into());
                }
            }
            "annotation" | "marker_annotation" => {
                if let Some(name) = node.child_by_field_name("name") {
                    info.references.insert(identifiers(name, source));
                }
            }
            "method_invocation" | "field_access" | "method_reference" => {
                if let Some(object) = node.child_by_field_name("object") {
                    // Type-qualified static calls are a frequent no-import Java dependency.
                    // Value receivers are filtered below using declarations in this file.
                    if matches!(object.kind(), "identifier" | "field_access") {
                        let name = identifiers(object, source);
                        if !generic_in_scope(node, &name, source) {
                            receivers.insert(name);
                        }
                    }
                }
            }
            _ => {}
        }
        let mut c = node.walk();
        let children: Vec<_> = node.named_children(&mut c).collect();
        nodes.extend(children.into_iter().rev());
    }
    info.references.extend(
        receivers
            .into_iter()
            .filter(|name| !value_names.contains(name.split('.').next().unwrap_or(""))),
    );
    info
}

fn generic_in_scope(node: Node<'_>, reference: &str, source: &str) -> bool {
    let first = reference.split('.').next().unwrap_or(reference);
    let mut ancestor = Some(node);
    while let Some(node) = ancestor {
        let mut c = node.walk();
        for child in node
            .named_children(&mut c)
            .filter(|n| n.kind() == "type_parameters")
        {
            let mut c = child.walk();
            for parameter in child
                .named_children(&mut c)
                .filter(|n| n.kind() == "type_parameter")
            {
                let mut c = parameter.walk();
                if parameter
                    .named_children(&mut c)
                    .find(|n| n.kind() == "type_identifier")
                    .map(|name| node_text(name, source) == first)
                    .unwrap_or(false)
                {
                    return true;
                }
            }
        }
        ancestor = node.parent();
    }
    false
}
