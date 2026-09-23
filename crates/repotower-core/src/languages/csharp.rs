//! C# type/namespace resolution over source files; never executes MSBuild or .NET.
use super::{Context, Extracted, ResolvedImport};
use crate::{model::DependencyKind, resolver::Resolution};
use std::collections::{BTreeSet, HashMap, HashSet};
use tree_sitter::{Node, Parser};

const AMBIGUOUS: &str = "C# type has multiple possible source declarations; project and assembly selection was not guessed.";
const MISSING: &str = "C# project type was not found; it may be ignored, generated, or outside the scanned source set.";

#[derive(Clone, Default)]
struct Scope {
    namespace: String,
    containers: Vec<String>,
    generics: HashSet<String>,
}
#[derive(Clone)]
struct Using {
    name: String,
    alias: Option<String>,
    scope: String,
    is_static: bool,
    global: bool,
}
#[derive(Clone)]
struct Reference {
    name: String,
    scope: Scope,
    attribute: bool,
}
#[derive(Default)]
struct Facts {
    declarations: Vec<(String, bool)>,
    namespaces: HashSet<String>,
    usings: Vec<Using>,
    references: Vec<Reference>,
    has_errors: bool,
}
struct Declaration {
    file: String,
    partial: bool,
}

pub struct Analyzer<'a> {
    _context: &'a Context<'a>,
    facts: HashMap<String, Facts>,
    types: HashMap<String, Vec<Declaration>>,
    namespaces: HashSet<String>,
    globals: Vec<Using>,
}
impl<'a> Analyzer<'a> {
    pub fn new(context: &'a Context<'a>) -> Result<Self, String> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_c_sharp::LANGUAGE.into())
            .map_err(|e| e.to_string())?;
        let mut facts = HashMap::new();
        let mut types: HashMap<String, Vec<Declaration>> = HashMap::new();
        let mut namespaces = HashSet::new();
        for (file, source) in context.sources {
            if !file.to_ascii_lowercase().ends_with(".cs") {
                continue;
            }
            let tree = parser
                .parse(source, None)
                .ok_or("C# syntax tree could not be parsed.")?;
            let info = inspect(tree.root_node(), source);
            namespaces.extend(info.namespaces.iter().cloned());
            for (name, partial) in &info.declarations {
                types.entry(name.clone()).or_default().push(Declaration {
                    file: file.clone(),
                    partial: *partial,
                });
            }
            facts.insert(file.clone(), info);
        }
        for declarations in types.values_mut() {
            declarations.sort_by(|a, b| a.file.cmp(&b.file));
        }
        // Namespace parents exist even when no type is declared directly in them.
        for name in namespaces.clone() {
            let mut parent = name.as_str();
            while let Some((prefix, _)) = parent.rsplit_once('.') {
                namespaces.insert(prefix.into());
                parent = prefix;
            }
        }
        for info in facts.values_mut() {
            for using in &mut info.usings {
                if using.name.starts_with("global::") {
                    continue;
                }
                let mut scope = using.scope.as_str();
                while !scope.is_empty() {
                    let candidate = qualify(scope, &using.name);
                    if namespaces.contains(&candidate) || types.contains_key(&candidate) {
                        using.name = candidate;
                        break;
                    }
                    scope = scope
                        .rsplit_once('.')
                        .map(|(parent, _)| parent)
                        .unwrap_or("");
                }
            }
        }
        let globals = facts
            .values()
            .flat_map(|info| info.usings.iter().filter(|u| u.global).cloned())
            .collect();
        Ok(Self {
            _context: context,
            facts,
            types,
            namespaces,
            globals,
        })
    }

    fn lookup(&self, name: &str) -> Option<Vec<Resolution>> {
        self.types.get(name).map(|declarations| {
            if declarations.len() > 1 && !declarations.iter().all(|d| d.partial) {
                vec![Resolution::Unresolved(AMBIGUOUS)]
            } else {
                declarations
                    .iter()
                    .map(|d| Resolution::Internal(d.file.clone()))
                    .collect()
            }
        })
    }

    fn local_name(&self, name: &str) -> bool {
        let mut prefix = name;
        while let Some((parent, _)) = prefix.rsplit_once('.') {
            if self.namespaces.contains(parent) || self.types.contains_key(parent) {
                return true;
            }
            prefix = parent;
        }
        false
    }

    fn applicable<'b>(
        &'b self,
        info: &'b Facts,
        scope: &'b Scope,
    ) -> impl Iterator<Item = &'b Using> {
        info.usings
            .iter()
            .filter(|u| !u.global)
            .chain(self.globals.iter())
            .filter(move |u| {
                u.global
                    || u.scope.is_empty()
                    || u.scope == scope.namespace
                    || scope.namespace.starts_with(&format!("{}.", u.scope))
            })
    }

    fn resolve(&self, info: &Facts, reference: &Reference) -> Vec<Resolution> {
        let raw = reference
            .name
            .strip_prefix("global::")
            .unwrap_or(&reference.name);
        let first = raw.split('.').next().unwrap_or(raw);
        if reference.scope.generics.contains(first) || raw.is_empty() || raw == "var" {
            return vec![];
        }
        if reference.name.starts_with("global::") {
            return self.lookup(raw).unwrap_or_else(|| {
                if self.local_name(raw) {
                    vec![Resolution::Unresolved(MISSING)]
                } else {
                    vec![]
                }
            });
        }
        let aliases: Vec<_> = self
            .applicable(info, &reference.scope)
            .filter(|u| u.alias.as_deref() == Some(first))
            .collect();
        if !aliases.is_empty() {
            let names: BTreeSet<_> = aliases
                .iter()
                .map(|u| {
                    format!(
                        "{}{}",
                        u.name.trim_start_matches("global::"),
                        &raw[first.len()..]
                    )
                })
                .collect();
            if names.len() > 1 {
                return vec![Resolution::Unresolved(AMBIGUOUS)];
            }
            let name = names.first().unwrap();
            return self.lookup(name).unwrap_or_else(|| {
                if self.local_name(name) {
                    vec![Resolution::Unresolved(MISSING)]
                } else {
                    vec![]
                }
            });
        }
        // Search the lexical type and namespace scopes before imported namespaces.
        for depth in (0..=reference.scope.containers.len()).rev() {
            let prefix = qualify(
                &reference.scope.namespace,
                &reference.scope.containers[..depth].join("."),
            );
            if let Some(found) = self.lookup(&qualify(&prefix, raw)) {
                return found;
            }
        }
        let mut namespace = reference.scope.namespace.as_str();
        while let Some((parent, _)) = namespace.rsplit_once('.') {
            if let Some(found) = self.lookup(&qualify(parent, raw)) {
                return found;
            }
            namespace = parent;
        }
        if let Some(found) = self.lookup(raw) {
            return found;
        }
        let mut candidates = BTreeSet::new();
        for using in self
            .applicable(info, &reference.scope)
            .filter(|u| u.alias.is_none())
        {
            let candidate = qualify(using.name.trim_start_matches("global::"), raw);
            if self.types.contains_key(&candidate) {
                candidates.insert(candidate);
            }
        }
        if candidates.len() > 1 {
            return vec![Resolution::Unresolved(AMBIGUOUS)];
        }
        if let Some(name) = candidates.first() {
            return self.lookup(name).unwrap_or_default();
        }
        if reference.attribute && !raw.ends_with("Attribute") {
            let mut suffixed = reference.clone();
            suffixed.name.push_str("Attribute");
            suffixed.attribute = false;
            return self.resolve(info, &suffixed);
        }
        if raw.contains('.') && self.local_name(raw) {
            vec![Resolution::Unresolved(MISSING)]
        } else {
            vec![]
        }
    }

    pub fn analyze(
        &mut self,
        importer: &str,
        _extension: &str,
        _source: &str,
    ) -> Result<Extracted, String> {
        let info = self
            .facts
            .get(importer)
            .ok_or("C# source was not indexed.")?;
        let mut imports = Vec::new();
        for using in &info.usings {
            let name = using.name.trim_start_matches("global::");
            if let Some(found) = self.lookup(name) {
                // Explicit type aliases/static imports name a concrete type. Ordinary namespace
                // imports only affect name lookup and must not create a namespace-wide fan-out.
                if using.alias.is_some() || using.is_static {
                    for resolution in found {
                        push(&mut imports, importer, name, resolution);
                    }
                }
            } else if !self.namespaces.contains(name) {
                let resolution = if self.local_name(name) {
                    Resolution::Unresolved(MISSING)
                } else {
                    Resolution::External
                };
                push(&mut imports, importer, name, resolution);
            }
        }
        for reference in &info.references {
            for resolution in self.resolve(info, reference) {
                push(&mut imports, importer, &reference.name, resolution);
            }
        }
        Ok(Extracted {
            imports,
            has_errors: info.has_errors,
        })
    }
}

fn push(
    imports: &mut Vec<ResolvedImport>,
    importer: &str,
    specifier: &str,
    resolution: Resolution,
) {
    if matches!(&resolution, Resolution::Internal(target) if target == importer) {
        return;
    }
    imports.push(ResolvedImport {
        specifier: specifier.into(),
        kind: DependencyKind::Static,
        resolution,
    });
}
fn qualify(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.into()
    } else if name.is_empty() {
        prefix.into()
    } else {
        format!("{prefix}.{name}")
    }
}
fn text<'a>(node: Node<'_>, source: &'a str) -> &'a str {
    node.utf8_text(source.as_bytes()).unwrap_or("")
}
fn children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut c = node.walk();
    node.named_children(&mut c).collect()
}
fn has_token(node: Node<'_>, token: &str, source: &str) -> bool {
    let mut c = node.walk();
    let found = node
        .children(&mut c)
        .any(|n| n.kind() == token || (n.kind() == "modifier" && text(n, source) == token));
    found
}
fn named_type(node: Node<'_>, source: &str) -> String {
    match node.kind() {
        "identifier" => text(node, source).trim_start_matches('@').into(),
        "generic_name" => {
            let parts = children(node);
            let name = parts
                .iter()
                .find(|n| n.kind() == "identifier")
                .map(|n| text(*n, source))
                .unwrap_or("");
            let arity = parts
                .iter()
                .find(|n| n.kind() == "type_argument_list")
                .map(|n| n.named_child_count())
                .unwrap_or(0);
            format!("{name}`{arity}")
        }
        "qualified_name" => qualify(
            &node
                .child_by_field_name("qualifier")
                .map(|n| named_type(n, source))
                .unwrap_or_default(),
            &node
                .child_by_field_name("name")
                .map(|n| named_type(n, source))
                .unwrap_or_default(),
        ),
        "alias_qualified_name" => {
            let alias = node
                .child_by_field_name("alias")
                .map(|n| text(n, source))
                .unwrap_or("");
            let name = node
                .child_by_field_name("name")
                .map(|n| named_type(n, source))
                .unwrap_or_default();
            if alias == "global" {
                format!("global::{name}")
            } else {
                qualify(alias.trim_start_matches('@'), &name)
            }
        }
        "member_access_expression" => qualify(
            &node
                .child_by_field_name("expression")
                .map(|n| named_type(n, source))
                .unwrap_or_default(),
            &node
                .child_by_field_name("name")
                .map(|n| named_type(n, source))
                .unwrap_or_default(),
        ),
        _ => String::new(),
    }
}
fn add_types(
    node: Node<'_>,
    source: &str,
    scope: &Scope,
    attribute: bool,
    references: &mut Vec<Reference>,
) {
    let name = named_type(node, source);
    if !name.is_empty() {
        references.push(Reference {
            name,
            scope: scope.clone(),
            attribute,
        });
    }
    // A qualified type's qualifier/name are one reference, but generic arguments
    // within either component remain independent dependencies.
    let mut nodes = vec![node];
    while let Some(current) = nodes.pop() {
        for child in children(current) {
            if child.kind() == "type_argument_list" {
                for argument in children(child) {
                    add_types(argument, source, scope, false, references);
                }
            } else if matches!(
                node.kind(),
                "nullable_type" | "array_type" | "pointer_type" | "tuple_type" | "ref_type"
            ) {
                add_types(child, source, scope, false, references);
            } else if matches!(
                child.kind(),
                "qualified_name" | "generic_name" | "alias_qualified_name"
            ) {
                nodes.push(child);
            }
        }
    }
}
fn declaration(kind: &str) -> bool {
    matches!(
        kind,
        "class_declaration"
            | "struct_declaration"
            | "interface_declaration"
            | "record_declaration"
            | "enum_declaration"
            | "delegate_declaration"
    )
}
fn inspect(root: Node<'_>, source: &str) -> Facts {
    let mut info = Facts {
        has_errors: root.has_error(),
        ..Facts::default()
    };
    let file_namespace = children(root)
        .into_iter()
        .find(|n| n.kind() == "file_scoped_namespace_declaration")
        .and_then(|n| n.child_by_field_name("name"))
        .map(|n| named_type(n, source))
        .unwrap_or_default();
    info.namespaces.insert(file_namespace.clone());
    let mut nodes = vec![(
        root,
        Scope {
            namespace: file_namespace,
            ..Scope::default()
        },
    )];
    let mut receivers = Vec::new();
    let mut value_names = HashSet::new();
    while let Some((node, mut scope)) = nodes.pop() {
        if node.kind() == "file_scoped_namespace_declaration" {
            continue;
        }
        if node.kind() == "namespace_declaration" {
            if let Some(name) = node.child_by_field_name("name") {
                scope.namespace = qualify(&scope.namespace, &named_type(name, source));
            }
            info.namespaces.insert(scope.namespace.clone());
            if let Some(body) = node.child_by_field_name("body") {
                nodes.push((body, scope));
            }
            continue;
        }
        if node.kind() == "using_directive" {
            let alias_node = node.child_by_field_name("name");
            let target = children(node).into_iter().find(|n| Some(*n) != alias_node);
            if let Some(target) = target {
                // A using alias can itself contain generic type arguments.
                let mut args = Vec::new();
                add_types(target, source, &scope, false, &mut args);
                let target_name = named_type(target, source);
                info.references.extend(
                    args.into_iter()
                        .filter(|reference| reference.name != target_name),
                );
                info.usings.push(Using {
                    name: named_type(target, source),
                    alias: alias_node.map(|n| text(n, source).trim_start_matches('@').into()),
                    scope: scope.namespace.clone(),
                    is_static: has_token(node, "static", source),
                    global: has_token(node, "global", source),
                });
            }
            continue;
        }
        let parameters = children(node)
            .into_iter()
            .find(|n| n.kind() == "type_parameter_list");
        if let Some(parameters) = parameters {
            for parameter in children(parameters) {
                if let Some(name) = parameter.child_by_field_name("name") {
                    scope.generics.insert(text(name, source).into());
                }
            }
        }
        if declaration(node.kind()) {
            if let Some(name) = node.child_by_field_name("name") {
                let mut name = text(name, source).trim_start_matches('@').to_string();
                if let Some(parameters) = parameters {
                    name.push_str(&format!("`{}", parameters.named_child_count()));
                }
                scope.containers.push(name);
                info.declarations.push((
                    qualify(&scope.namespace, &scope.containers.join(".")),
                    has_token(node, "partial", source),
                ));
            }
        }
        if matches!(
            node.kind(),
            "variable_declarator"
                | "parameter"
                | "property_declaration"
                | "foreach_statement"
                | "catch_declaration"
        ) {
            if let Some(name) = node.child_by_field_name("name") {
                value_names.insert(text(name, source).trim_start_matches('@').to_string());
            }
        }
        for field in ["type", "returns"] {
            if let Some(ty) = node.child_by_field_name(field) {
                add_types(ty, source, &scope, false, &mut info.references);
            }
        }
        match node.kind() {
            "base_list" | "type_argument_list" => {
                for ty in children(node) {
                    add_types(ty, source, &scope, false, &mut info.references);
                }
            }
            "attribute" => {
                if let Some(name) = node.child_by_field_name("name") {
                    add_types(name, source, &scope, true, &mut info.references);
                }
            }
            "member_access_expression" => {
                if let Some(receiver) = node.child_by_field_name("expression") {
                    let name = named_type(receiver, source);
                    if !name.is_empty() {
                        receivers.push(Reference {
                            name,
                            scope: scope.clone(),
                            attribute: false,
                        });
                    }
                }
            }
            _ => {}
        }
        nodes.extend(
            children(node)
                .into_iter()
                .rev()
                .map(|child| (child, scope.clone())),
        );
    }
    info.references.extend(
        receivers.into_iter().filter(|reference| {
            !value_names.contains(reference.name.split('.').next().unwrap_or(""))
        }),
    );
    info
}
