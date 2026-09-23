//! Python's source-file import graph. Never imports or executes the project.
//!
//! Absolute imports use the selected folder and its conventional `src` folder.
//! Runtime path changes, import hooks, and dynamically generated exports cannot
//! be reproduced by a read-only source analyzer.
use super::{Context, Extracted, ResolvedImport};
use crate::{model::DependencyKind, resolver::Resolution};
use std::collections::{HashMap, HashSet};
use tree_sitter::{Node, Parser};

const MISSING: &str = "Python 项目内模块未找到；目标可能不存在、被忽略或使用自定义导入路径。";
const AMBIGUOUS: &str = "Python 导入在项目根目录与 src 目录中存在多个候选；未猜测 sys.path 顺序。";
const OUTSIDE: &str = "Python 相对导入超出可确定的包范围。";
const DYNAMIC_EXPORT: &str = "Python 包导出可能由 __getattr__ 动态生成；未猜测同名子模块。";
const UNKNOWN_MEMBER: &str = "Python 包成员未在静态导出或已扫描子模块中找到；未猜测文件依赖。";
const DYNAMIC_ALL: &str = "Python __all__ 不是静态字符串列表；通配导入的子模块可能不完整。";

#[derive(Clone, Copy, PartialEq, Eq)]
enum ModuleKind {
    File,
    Package,
    Namespace,
}

#[derive(Clone)]
struct Found {
    /// All initializers loaded en route, followed by the concrete target.
    files: Vec<String>,
    base: String,
    /// A namespace package may span both supported source roots.
    alternatives: Vec<String>,
    kind: ModuleKind,
}

#[derive(Default)]
struct Bindings {
    names: HashMap<String, usize>,
    dynamic: bool,
    all: Option<Vec<String>>,
    all_position: Option<usize>,
}

pub struct Analyzer<'a> {
    context: &'a Context<'a>,
    parser: Parser,
    directories: HashSet<String>,
    roots: Vec<&'static str>,
    bindings: HashMap<String, Bindings>,
}

impl<'a> Analyzer<'a> {
    pub fn new(context: &'a Context<'a>) -> Result<Self, String> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .map_err(|error| format!("Python 解析器初始化失败：{error}"))?;
        let mut directories = HashSet::new();
        for id in context.known {
            if !is_python(id) {
                continue;
            }
            let mut path = id.as_str();
            while let Some((parent, _)) = path.rsplit_once('/') {
                directories.insert(parent.to_string());
                path = parent;
            }
        }
        let mut roots = vec![""];
        if directories.contains("src")
            && !context.known.contains("src/__init__.py")
            && !context.known.contains("src/__init__.pyi")
        {
            roots.push("src");
        }
        Ok(Self {
            context,
            parser,
            directories,
            roots,
            bindings: HashMap::new(),
        })
    }

    pub fn analyze(
        &mut self,
        importer: &str,
        _extension: &str,
        source: &str,
    ) -> Result<Extracted, String> {
        let tree = self
            .parser
            .parse(source, None)
            .ok_or_else(|| "Python 解析未完成。".to_string())?;
        let mut imports = Vec::new();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            match node.kind() {
                "import_statement" => {
                    let mut cursor = node.walk();
                    for name in node.children_by_field_name("name", &mut cursor) {
                        let name = import_name(name, source);
                        if name.is_empty() {
                            continue;
                        }
                        match self.absolute(&name) {
                            Ok(found) => emit(&mut imports, importer, &name, &found, true),
                            Err(resolution) => push(&mut imports, &name, resolution),
                        }
                    }
                }
                "import_from_statement" => {
                    if let Some(module) = node.child_by_field_name("module_name") {
                        let specifier = compact_text(module, source);
                        let base = if specifier.starts_with('.') {
                            self.relative(importer, &specifier)
                        } else {
                            self.absolute(&specifier)
                        };
                        match base {
                            Ok(found) => {
                                emit(&mut imports, importer, &specifier, &found, false);
                                if found.kind != ModuleKind::File {
                                    let mut cursor = node.walk();
                                    for name in node.children_by_field_name("name", &mut cursor) {
                                        let name = import_name(name, source);
                                        self.from_member(
                                            importer,
                                            &specifier,
                                            &found,
                                            &name,
                                            node.start_byte(),
                                            &mut imports,
                                        );
                                    }
                                    let wildcard = node
                                        .named_children(&mut cursor)
                                        .any(|child| child.kind() == "wildcard_import");
                                    if wildcard {
                                        self.from_star(
                                            importer,
                                            &specifier,
                                            &found,
                                            node.start_byte(),
                                            &mut imports,
                                        );
                                    }
                                }
                            }
                            Err(resolution) => push(&mut imports, &specifier, resolution),
                        }
                    }
                }
                // Future directives never refer to a same-named local file.
                "future_import_statement" => {
                    push(&mut imports, "__future__", Resolution::External);
                }
                _ => {
                    let mut cursor = node.walk();
                    stack.extend(node.named_children(&mut cursor));
                }
            }
        }
        Ok(Extracted {
            imports,
            has_errors: tree.root_node().has_error(),
        })
    }

    fn concrete(&self, stem: &str) -> Option<String> {
        // A source module is preferred to its optional typing stub. Stub-only
        // modules remain analyzable without installing a Python environment.
        ["py", "pyi"]
            .into_iter()
            .map(|extension| format!("{stem}.{extension}"))
            .find(|candidate| self.context.known.contains(candidate))
    }

    fn at(&self, base: &str) -> Option<(ModuleKind, Option<String>)> {
        if let Some(initializer) = self.concrete(&join(base, "__init__")) {
            Some((ModuleKind::Package, Some(initializer)))
        } else if let Some(module) = self.concrete(base) {
            Some((ModuleKind::File, Some(module)))
        } else if self.directories.contains(base) {
            Some((ModuleKind::Namespace, None))
        } else {
            None
        }
    }

    fn lookup(&self, root: &str, parts: &[&str]) -> Option<Found> {
        let mut path = root.to_string();
        let mut files = Vec::new();
        let mut kind = ModuleKind::Namespace;
        for (index, part) in parts.iter().enumerate() {
            path = join(&path, part);
            let (resolved_kind, file) = self.at(&path)?;
            if resolved_kind == ModuleKind::File && index + 1 != parts.len() {
                return None;
            }
            kind = resolved_kind;
            files.extend(file);
        }
        Some(Found {
            files,
            base: path,
            alternatives: vec![],
            kind,
        })
    }

    fn absolute(&self, specifier: &str) -> Result<Found, Resolution> {
        let parts: Vec<_> = specifier
            .split('.')
            .filter(|part| !part.is_empty())
            .collect();
        let Some(first) = parts.first() else {
            return Err(Resolution::Unresolved(MISSING));
        };
        let candidates: Vec<_> = self
            .roots
            .iter()
            .filter_map(|root| self.lookup(root, &parts))
            .collect();
        match candidates.len() {
            1 => Ok(candidates.into_iter().next().unwrap()),
            0 => {
                if self
                    .roots
                    .iter()
                    .any(|root| self.at(&join(root, first)).is_some())
                {
                    Err(Resolution::Unresolved(MISSING))
                } else {
                    Err(Resolution::External)
                }
            }
            _ if candidates
                .iter()
                .all(|found| found.kind == ModuleKind::Namespace && found.files.is_empty()) =>
            {
                let mut candidates = candidates.into_iter();
                let mut first = candidates.next().unwrap();
                first.alternatives = candidates.map(|found| found.base).collect();
                Ok(first)
            }
            _ => Err(Resolution::Unresolved(AMBIGUOUS)),
        }
    }

    fn relative(&self, importer: &str, specifier: &str) -> Result<Found, Resolution> {
        let level = specifier.bytes().take_while(|byte| *byte == b'.').count();
        let directory = importer
            .rsplit_once('/')
            .map(|(parent, _)| parent)
            .unwrap_or("");
        let root = if self.roots.contains(&"src")
            && (directory.starts_with("src/") || directory == "src")
        {
            "src"
        } else {
            ""
        };
        let local = directory
            .strip_prefix(root)
            .unwrap_or(directory)
            .trim_start_matches('/');
        let mut parts: Vec<_> = local.split('/').filter(|part| !part.is_empty()).collect();
        if level > parts.len() {
            return Err(Resolution::Unresolved(OUTSIDE));
        }
        parts.truncate(parts.len() + 1 - level);
        parts.extend(
            specifier[level..]
                .split('.')
                .filter(|part| !part.is_empty()),
        );
        self.lookup(root, &parts)
            .ok_or(Resolution::Unresolved(MISSING))
    }

    fn from_member(
        &mut self,
        importer: &str,
        specifier: &str,
        base: &Found,
        name: &str,
        statement_start: usize,
        imports: &mut Vec<ResolvedImport>,
    ) {
        if name.is_empty() || name == "*" {
            return;
        }
        if base.kind == ModuleKind::Package {
            if let Some(initializer) = base.files.last() {
                self.ensure_bindings(initializer);
                if let Some(bindings) = self.bindings.get(initializer) {
                    if let Some(position) = bindings.names.get(name) {
                        // A `from . import child` inside __init__ must not
                        // suppress itself merely because it binds `child`.
                        if initializer != importer || *position < statement_start {
                            return;
                        }
                    }
                    if bindings.dynamic {
                        push(
                            imports,
                            &member_specifier(specifier, name),
                            Resolution::Unresolved(DYNAMIC_EXPORT),
                        );
                        return;
                    }
                }
            }
        }
        let candidates: Vec<_> = std::iter::once(&base.base)
            .chain(base.alternatives.iter())
            .filter_map(|parent| {
                let child = join(parent, &name.replace('.', "/"));
                self.at(&child).map(|(kind, file)| Found {
                    files: file.into_iter().collect(),
                    base: child,
                    alternatives: vec![],
                    kind,
                })
            })
            .collect();
        if candidates.len() == 1 {
            let found = &candidates[0];
            emit(
                imports,
                importer,
                &member_specifier(specifier, name),
                &found,
                true,
            );
        } else if candidates.len() > 1 {
            push(
                imports,
                &member_specifier(specifier, name),
                Resolution::Unresolved(AMBIGUOUS),
            );
        } else if base.kind == ModuleKind::Namespace {
            // A namespace has no initializer that could supply this attribute.
            push(
                imports,
                &member_specifier(specifier, name),
                Resolution::Unresolved(MISSING),
            );
        } else {
            push(
                imports,
                &member_specifier(specifier, name),
                Resolution::Unresolved(UNKNOWN_MEMBER),
            );
        }
        // A regular package may export ordinary values. Missing member names
        // are not guessed to be missing files or added to the file graph.
    }

    fn ensure_bindings(&mut self, initializer: &str) {
        if self.bindings.contains_key(initializer) {
            return;
        }
        let mut bindings = Bindings::default();
        if let Some(source) = self.context.sources.get(initializer) {
            if let Some(tree) = self.parser.parse(source, None) {
                collect_bindings(
                    tree.root_node(),
                    source,
                    &mut bindings,
                    initializer.ends_with(".pyi"),
                );
            }
        }
        self.bindings.insert(initializer.to_string(), bindings);
    }

    fn from_star(
        &mut self,
        importer: &str,
        specifier: &str,
        base: &Found,
        statement_start: usize,
        imports: &mut Vec<ResolvedImport>,
    ) {
        if base.kind != ModuleKind::Package {
            return;
        }
        let Some(initializer) = base.files.last() else {
            return;
        };
        self.ensure_bindings(initializer);
        let Some(bindings) = self.bindings.get(initializer) else {
            return;
        };
        let Some(position) = bindings.all_position else {
            return;
        };
        if initializer == importer && position >= statement_start {
            return;
        }
        if let Some(names) = bindings.all.clone() {
            for name in names {
                self.from_member(importer, specifier, base, &name, statement_start, imports);
            }
        } else {
            push(
                imports,
                &member_specifier(specifier, "*"),
                Resolution::Unresolved(DYNAMIC_ALL),
            );
        }
    }
}

fn collect_bindings(node: Node<'_>, source: &str, bindings: &mut Bindings, stub: bool) {
    let mut stack = vec![node];
    while let Some(node) = stack.pop() {
        match node.kind() {
            "function_definition" | "class_definition" => {
                if let Some(name) = node.child_by_field_name("name") {
                    let name = compact_text(name, source);
                    bindings.dynamic |= name == "__getattr__";
                    bind(&mut bindings.names, name, node.start_byte());
                }
                // Function locals and class members are not package exports.
            }
            "lambda" => {}
            "assignment" | "augmented_assignment" => {
                // Runtime annotations alone do not bind a package attribute;
                // in a stub they describe an exported value deliberately.
                if !stub && node.child_by_field_name("right").is_none() {
                    continue;
                }
                if let Some(left) = node.child_by_field_name("left") {
                    collect_pattern(left, source, &mut bindings.names);
                    if left.kind() == "identifier"
                        && compact_text(left, source) == "__all__"
                        && bindings
                            .all_position
                            .map(|position| position < node.start_byte())
                            .unwrap_or(true)
                    {
                        bindings.all_position = Some(node.start_byte());
                        bindings.all = if node.kind() == "assignment" {
                            node.child_by_field_name("right")
                                .and_then(|right| literal_names(right, source))
                        } else {
                            None
                        };
                    }
                }
                if let Some(right) = node.child_by_field_name("right") {
                    if right.kind() == "assignment" {
                        stack.push(right);
                    }
                }
            }
            "import_statement" | "import_from_statement" | "future_import_statement" => {
                let mut cursor = node.walk();
                for name in node.children_by_field_name("name", &mut cursor) {
                    let binding = if let Some(alias) = name.child_by_field_name("alias") {
                        compact_text(alias, source)
                    } else {
                        import_name(name, source)
                            .split('.')
                            .next()
                            .unwrap_or("")
                            .to_string()
                    };
                    if !binding.is_empty() {
                        bind(&mut bindings.names, binding, name.start_byte());
                    }
                }
            }
            _ => {
                let mut cursor = node.walk();
                stack.extend(node.named_children(&mut cursor));
            }
        }
    }
}

fn collect_pattern(node: Node<'_>, source: &str, names: &mut HashMap<String, usize>) {
    if node.kind() == "identifier" {
        bind(names, compact_text(node, source), node.start_byte());
    } else if matches!(
        node.kind(),
        "pattern_list" | "tuple_pattern" | "list_pattern" | "list_splat_pattern"
    ) {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            collect_pattern(child, source, names);
        }
    }
}

fn literal_names(node: Node<'_>, source: &str) -> Option<Vec<String>> {
    if !matches!(node.kind(), "list" | "tuple") {
        return None;
    }
    let mut cursor = node.walk();
    let names = node
        .named_children(&mut cursor)
        .filter(|child| child.kind() != "comment")
        .map(|child| {
            if child.kind() != "string" {
                return None;
            }
            let raw = child.utf8_text(source.as_bytes()).ok()?;
            let name = raw
                .strip_prefix('"')
                .and_then(|rest| rest.strip_suffix('"'))
                .or_else(|| {
                    raw.strip_prefix('\'')
                        .and_then(|rest| rest.strip_suffix('\''))
                })?;
            if name.is_empty()
                || !name
                    .chars()
                    .all(|character| character == '_' || character.is_alphanumeric())
            {
                return None;
            }
            Some(name.to_string())
        })
        .collect();
    names
}

fn emit(
    imports: &mut Vec<ResolvedImport>,
    importer: &str,
    specifier: &str,
    found: &Found,
    explicit: bool,
) {
    for (index, file) in found.files.iter().enumerate() {
        // Being inside a package initializer is not by itself a self-import.
        if file != importer || (explicit && index + 1 == found.files.len()) {
            push(imports, specifier, Resolution::Internal(file.clone()));
        }
    }
}

fn bind(names: &mut HashMap<String, usize>, name: String, position: usize) {
    names
        .entry(name)
        .and_modify(|existing| *existing = (*existing).min(position))
        .or_insert(position);
}

fn push(imports: &mut Vec<ResolvedImport>, specifier: &str, resolution: Resolution) {
    imports.push(ResolvedImport {
        specifier: specifier.to_string(),
        kind: DependencyKind::Static,
        resolution,
    });
}

fn compact_text(node: Node<'_>, source: &str) -> String {
    node.utf8_text(source.as_bytes())
        .unwrap_or("")
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn import_name(node: Node<'_>, source: &str) -> String {
    compact_text(node.child_by_field_name("name").unwrap_or(node), source)
}

fn join(parent: &str, child: &str) -> String {
    if parent.is_empty() {
        child.to_string()
    } else {
        format!("{parent}/{child}")
    }
}

fn member_specifier(base: &str, member: &str) -> String {
    format!(
        "{base}{}{member}",
        if base.ends_with('.') { "" } else { "." }
    )
}

fn is_python(id: &str) -> bool {
    id.ends_with(".py") || id.ends_with(".pyi")
}
