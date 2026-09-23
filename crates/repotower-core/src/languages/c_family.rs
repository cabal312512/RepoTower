use super::{Context, Extracted, ResolvedImport};
use crate::{model::DependencyKind, resolver::Resolution};
use std::collections::{HashMap, HashSet};
use tree_sitter::{Node, Parser};

const DYNAMIC: &str = "C/C++ 包含目标由宏计算，未执行预处理。";
const MISSING: &str = "在已扫描的 C/C++ 文件中未找到包含目标。";
const SEARCH_PATH: &str = "包含目标需要明确且一致的编译器搜索路径；未按同名文件猜测连线。";
const OUTSIDE: &str = "路径位于所选项目之外。";
const UNSUPPORTED: &str = "包含路径使用不支持的格式。";

pub struct Analyzer<'a> {
    context: &'a Context<'a>,
    c: Parser,
    cpp: Parser,
    has_cpp: bool,
    suffixes: HashSet<String>,
    profiles: HashMap<String, Vec<Profile>>,
}

#[derive(Default)]
struct Profile {
    quote: Vec<Option<String>>,
    include: Vec<Option<String>>,
    system: Vec<Option<String>>,
    unsupported: bool,
}

fn is_source(id: &str) -> bool {
    id.rsplit_once('.').is_some_and(|(_, ext)| {
        matches!(
            ext.to_ascii_lowercase().as_str(),
            "c" | "h" | "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" | "inl" | "ipp"
        )
    })
}
fn text<'a>(node: Node<'_>, source: &'a str) -> &'a str {
    source.get(node.byte_range()).unwrap_or("")
}

impl<'a> Analyzer<'a> {
    pub fn new(context: &'a Context<'a>) -> Result<Self, String> {
        let mut c = Parser::new();
        let mut cpp = Parser::new();
        c.set_language(&tree_sitter_c::LANGUAGE.into())
            .map_err(|error| error.to_string())?;
        cpp.set_language(&tree_sitter_cpp::LANGUAGE.into())
            .map_err(|error| error.to_string())?;
        let mut suffixes = HashSet::new();
        for id in context.known.iter().filter(|id| is_source(id)) {
            suffixes.insert(id.clone());
            for (index, _) in id.match_indices('/') {
                suffixes.insert(id[index + 1..].into());
            }
        }
        let profiles = compilation_profiles(context);
        let has_cpp = context.known.iter().any(|id| {
            id.ends_with(".C")
                || id.rsplit_once('.').is_some_and(|(_, ext)| {
                    matches!(
                        ext.to_ascii_lowercase().as_str(),
                        "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx"
                    )
                })
        });
        Ok(Self {
            context,
            c,
            cpp,
            has_cpp,
            suffixes,
            profiles,
        })
    }

    pub fn analyze(
        &mut self,
        importer: &str,
        extension: &str,
        source: &str,
    ) -> Result<Extracted, String> {
        let parser = if (extension == "c" && !importer.ends_with(".C"))
            || (extension == "h" && !self.has_cpp)
        {
            &mut self.c
        } else {
            &mut self.cpp
        };
        let tree = parser.parse(source, None).ok_or("解析被中止。")?;
        let mut imports = Vec::new();
        let mut cursor = tree.walk();
        loop {
            let node = cursor.node();
            if node.kind() == "preproc_include" {
                if let Some(path) = node.child_by_field_name("path") {
                    let raw = text(path, source);
                    let quoted = path.kind() == "string_literal"
                        && raw.starts_with('"')
                        && raw.ends_with('"');
                    let angled = path.kind() == "system_lib_string"
                        && raw.starts_with('<')
                        && raw.ends_with('>');
                    let (specifier, resolution) = if (quoted || angled) && raw.len() >= 2 {
                        let specifier = raw[1..raw.len() - 1].to_owned();
                        let result = self.resolve(importer, &specifier, quoted);
                        (specifier, result)
                    } else {
                        (
                            raw.chars().take(160).collect(),
                            Resolution::Unresolved(DYNAMIC),
                        )
                    };
                    imports.push(ResolvedImport {
                        specifier,
                        kind: DependencyKind::Include,
                        resolution,
                    });
                }
            } else if node.kind() == "preproc_call"
                && node
                    .child_by_field_name("directive")
                    .is_some_and(|directive| text(directive, source).trim() == "#include_next")
            {
                imports.push(ResolvedImport {
                    specifier: text(node, source).trim().chars().take(160).collect(),
                    kind: DependencyKind::Include,
                    resolution: Resolution::Unresolved(
                        "#include_next 需要编译器搜索路径，尚未解析。",
                    ),
                });
            }
            if cursor.goto_first_child() {
                continue;
            }
            loop {
                if cursor.goto_next_sibling() {
                    break;
                }
                if !cursor.goto_parent() {
                    return Ok(Extracted {
                        imports,
                        has_errors: tree.root_node().has_error(),
                    });
                }
            }
        }
    }

    fn resolve(&self, importer: &str, specifier: &str, quoted: bool) -> Resolution {
        if specifier.is_empty() || specifier.contains(['\\', '\0', ':']) {
            return Resolution::Unresolved(UNSUPPORTED);
        }
        if specifier.starts_with('/') {
            return Resolution::Unresolved(OUTSIDE);
        }
        let directory = importer
            .rsplit_once('/')
            .map(|(directory, _)| directory)
            .unwrap_or("");
        let profiles = self.profiles.get(importer);
        if profiles.is_some_and(|profiles| profiles.iter().any(|profile| profile.unsupported)) {
            return Resolution::Unresolved(SEARCH_PATH);
        }
        if quoted {
            let Some(relative) = normalize(directory, specifier) else {
                return Resolution::Unresolved(OUTSIDE);
            };
            if self.context.known.contains(&relative) && is_source(&relative) {
                return Resolution::Internal(relative);
            }
            // An ignored or unreadable file can shadow later include directories.
            if self.context.root.join(&relative).symlink_metadata().is_ok() {
                return Resolution::Unresolved(MISSING);
            }
        }
        if let Some(profiles) = profiles {
            let mut results = profiles
                .iter()
                .map(|profile| self.search(profile, specifier, quoted));
            if let Some(first) = results.next() {
                return if results.all(|other| other == first) {
                    first
                } else {
                    Resolution::Unresolved(SEARCH_PATH)
                };
            }
        }
        self.unknown(specifier, quoted)
    }

    fn unknown(&self, specifier: &str, quoted: bool) -> Resolution {
        if normalize("", specifier).is_some_and(|path| self.suffixes.contains(&path)) {
            Resolution::Unresolved(SEARCH_PATH)
        } else if quoted {
            Resolution::Unresolved(MISSING)
        } else {
            Resolution::External
        }
    }

    fn search(&self, profile: &Profile, specifier: &str, quoted: bool) -> Resolution {
        let mut external_before = false;
        for directory in profile
            .quote
            .iter()
            .filter(|_| quoted)
            .chain(&profile.include)
            .chain(&profile.system)
        {
            let Some(directory) = directory else {
                external_before = true;
                continue;
            };
            let Some(candidate) = normalize(directory, specifier) else {
                external_before = true;
                continue;
            };
            if self.context.known.contains(&candidate) && is_source(&candidate) {
                return if external_before {
                    Resolution::Unresolved(SEARCH_PATH)
                } else {
                    Resolution::Internal(candidate)
                };
            }
            if self
                .context
                .root
                .join(&candidate)
                .symlink_metadata()
                .is_ok()
            {
                return Resolution::Unresolved(MISSING);
            }
        }
        self.unknown(specifier, quoted)
    }
}

fn compilation_profiles(context: &Context<'_>) -> HashMap<String, Vec<Profile>> {
    let mut profiles: HashMap<String, Vec<Profile>> = HashMap::new();
    let Some(config) = context.read_config("compile_commands.json") else {
        return profiles;
    };
    let Ok(serde_json::Value::Array(entries)) = serde_json::from_str(&config) else {
        return profiles;
    };
    for entry in entries.into_iter().take(10_000) {
        let directory = entry
            .get("directory")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
        let Some(directory) = project_path(context, "", directory) else {
            continue;
        };
        let Some(file) = entry
            .get("file")
            .and_then(|v| v.as_str())
            .and_then(|file| project_path(context, &directory, file))
        else {
            continue;
        };
        if !context.known.contains(&file) || !is_source(&file) {
            continue;
        }
        let arguments = match entry.get("arguments").and_then(|v| v.as_array()) {
            Some(values) => values
                .iter()
                .map(|v| v.as_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>(),
            None => entry
                .get("command")
                .and_then(|v| v.as_str())
                .and_then(split_command),
        };
        let mut profile = Profile::default();
        let Some(arguments) = arguments else {
            profile.unsupported = true;
            profiles.entry(file).or_default().push(profile);
            continue;
        };
        let mut index = 0;
        while index < arguments.len() {
            let argument = &arguments[index];
            // Response files and compiler-specific search modifiers need more context.
            if argument.starts_with('@')
                || argument == "-I-"
                || argument.starts_with("-idirafter")
                || argument.starts_with("-iprefix")
                || argument.starts_with("-iwithprefix")
                || argument.starts_with("--sysroot")
                || argument.starts_with("-isysroot")
            {
                profile.unsupported = true;
            }
            for (flag, kind) in [("-iquote", 0), ("-isystem", 2), ("-I", 1), ("/I", 1)] {
                if let Some(rest) = argument.strip_prefix(flag) {
                    let value = if rest.is_empty() {
                        index += 1;
                        arguments.get(index).map(String::as_str)
                    } else {
                        Some(rest)
                    };
                    let Some(value) = value else {
                        profile.unsupported = true;
                        break;
                    };
                    if value.contains(['$', '`', '%']) {
                        profile.unsupported = true;
                    }
                    let directory = project_path(context, &directory, value);
                    match kind {
                        0 => profile.quote.push(directory),
                        2 => profile.system.push(directory),
                        _ => profile.include.push(directory),
                    }
                    break;
                }
            }
            index += 1;
        }
        // GCC treats a directory specified by both -I and -isystem as a system
        // directory; keeping the early -I occurrence would change lookup order.
        profile
            .include
            .retain(|path| path.is_none() || !profile.system.contains(path));
        profiles.entry(file).or_default().push(profile);
    }
    profiles
}

// Lexical, project-confined conversion. No external directory or header is opened.
fn project_path(context: &Context<'_>, directory: &str, value: &str) -> Option<String> {
    if value.is_empty() || value.contains('\0') {
        return None;
    }
    let root = context.root.to_string_lossy().replace('\\', "/");
    let root = root.strip_prefix("//?/").unwrap_or(&root);
    let value = value.replace('\\', "/");
    let absolute = if value.starts_with('/') || value.as_bytes().get(1) == Some(&b':') {
        if value.as_bytes().get(1) == Some(&b':') && value.as_bytes().get(2) != Some(&b'/') {
            return None;
        }
        value
    } else {
        format!("{root}/{directory}/{value}")
    };
    let absolute = absolute.strip_prefix("//?/").unwrap_or(&absolute);
    let mut parts = Vec::new();
    for part in absolute.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.last().is_some_and(|s: &&str| s.ends_with(':')) {
                    return None;
                }
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    let normalized = if root.starts_with('/') {
        format!("/{}", parts.join("/"))
    } else {
        parts.join("/")
    };
    let same_root = if cfg!(windows) {
        normalized
            .to_lowercase()
            .starts_with(&format!("{}/", root.to_lowercase()))
            || normalized.eq_ignore_ascii_case(root)
    } else {
        normalized.starts_with(&format!("{root}/")) || normalized == root
    };
    if !same_root {
        return None;
    }
    Some(normalized[root.len()..].trim_start_matches('/').to_string())
}

fn split_command(command: &str) -> Option<Vec<String>> {
    let mut result = Vec::new();
    let mut token = String::new();
    let mut quote = None;
    let mut chars = command.chars().peekable();
    let mut started = false;
    while let Some(ch) = chars.next() {
        if ch == '\0' || matches!(ch, '$' | '`' | '\n' | '\r') {
            return None;
        }
        if let Some(delimiter) = quote {
            if ch == delimiter {
                quote = None;
            } else if ch == '\\' && delimiter == '"' && chars.peek() == Some(&'"') {
                token.push(chars.next()?);
            } else {
                token.push(ch);
            }
        } else {
            match ch {
                '\'' | '"' => {
                    quote = Some(ch);
                    started = true;
                }
                ';' | '|' | '&' | '<' | '>' => return None,
                '\\' if chars
                    .peek()
                    .is_some_and(|next| next.is_whitespace() || matches!(next, '\'' | '"')) =>
                {
                    token.push(chars.next()?);
                    started = true;
                }
                ch if ch.is_whitespace() => {
                    if started {
                        result.push(std::mem::take(&mut token));
                        started = false;
                    }
                }
                _ => {
                    token.push(ch);
                    started = true;
                }
            }
        }
    }
    if quote.is_some() {
        return None;
    }
    if started {
        result.push(token);
    }
    Some(result)
}

fn normalize(directory: &str, path: &str) -> Option<String> {
    let mut parts: Vec<&str> = directory.split('/').filter(|s| !s.is_empty()).collect();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    Some(parts.join("/"))
}
