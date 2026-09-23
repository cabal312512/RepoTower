pub mod c_family;
pub mod csharp;
pub mod go;
pub mod java;
pub mod python;
pub mod rust;

use crate::{
    model::DependencyKind,
    parser::Parsers,
    resolver::{self, Resolution},
};
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    fs::File,
    io::Read,
    path::{Component, Path},
};

/// Only scanned UTF-8 sources can become graph nodes. Configuration reads are
/// bounded, cached and confined to the chosen repository; no build tool is run.
pub struct Context<'a> {
    pub root: &'a Path,
    pub known: &'a HashSet<String>,
    pub sources: &'a HashMap<String, String>,
    config: RefCell<HashMap<String, Option<String>>>,
    config_bytes: Cell<usize>,
    config_limited: Cell<bool>,
}
impl<'a> Context<'a> {
    pub fn new(
        root: &'a Path,
        known: &'a HashSet<String>,
        sources: &'a HashMap<String, String>,
    ) -> Self {
        Self {
            root,
            known,
            sources,
            config: RefCell::new(HashMap::new()),
            config_bytes: Cell::new(0),
            config_limited: Cell::new(false),
        }
    }
    pub fn read_config(&self, relative: &str) -> Option<String> {
        if let Some(value) = self.config.borrow().get(relative) {
            return value.clone();
        }
        if self.config.borrow().len() >= 20_000 {
            self.config_limited.set(true);
            return None;
        }
        let value = (|| {
            let relative_path = Path::new(relative);
            if relative_path
                .components()
                .any(|part| !matches!(part, Component::Normal(_) | Component::CurDir))
            {
                return None;
            }
            let path = self.root.join(relative_path).canonicalize().ok()?;
            if !path.starts_with(self.root) || !path.is_file() {
                return None;
            }
            let mut bytes = Vec::new();
            File::open(path)
                .ok()?
                .take(1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .ok()?;
            if bytes.len() > 1024 * 1024 || self.config_bytes.get() + bytes.len() > 16 * 1024 * 1024
            {
                self.config_limited.set(true);
                return None;
            }
            self.config_bytes.set(self.config_bytes.get() + bytes.len());
            String::from_utf8(bytes).ok()
        })();
        self.config
            .borrow_mut()
            .insert(relative.into(), value.clone());
        value
    }
    pub fn config_limited(&self) -> bool {
        self.config_limited.get()
    }
}

pub struct ResolvedImport {
    pub specifier: String,
    pub kind: DependencyKind,
    pub resolution: Resolution,
}
pub struct Extracted {
    pub imports: Vec<ResolvedImport>,
    pub has_errors: bool,
}

/// Build only the language indexes present in this repository.
pub struct Analyzers<'a> {
    context: &'a Context<'a>,
    js: Parsers,
    java: Option<java::Analyzer<'a>>,
    python: Option<python::Analyzer<'a>>,
    c_family: Option<c_family::Analyzer<'a>>,
    go: Option<go::Analyzer<'a>>,
    rust: Option<rust::Analyzer<'a>>,
    csharp: Option<csharp::Analyzer<'a>>,
}
impl<'a> Analyzers<'a> {
    pub fn new(context: &'a Context<'a>) -> Result<Self, String> {
        let extensions: HashSet<&str> = context
            .known
            .iter()
            .filter_map(|id| id.rsplit_once('.').map(|(_, extension)| extension))
            .collect();
        let has = |values: &[&str]| {
            extensions
                .iter()
                .any(|ext| values.iter().any(|value| ext.eq_ignore_ascii_case(value)))
        };
        Ok(Self {
            context,
            js: Parsers::new()?,
            java: if has(&["java"]) {
                Some(java::Analyzer::new(context)?)
            } else {
                None
            },
            python: if has(&["py", "pyi"]) {
                Some(python::Analyzer::new(context)?)
            } else {
                None
            },
            c_family: if has(&[
                "c", "h", "cc", "cpp", "cxx", "hpp", "hh", "hxx", "inl", "ipp",
            ]) {
                Some(c_family::Analyzer::new(context)?)
            } else {
                None
            },
            go: if has(&["go"]) {
                Some(go::Analyzer::new(context)?)
            } else {
                None
            },
            rust: if has(&["rs"]) {
                Some(rust::Analyzer::new(context)?)
            } else {
                None
            },
            csharp: if has(&["cs"]) {
                Some(csharp::Analyzer::new(context)?)
            } else {
                None
            },
        })
    }
    pub fn analyze(
        &mut self,
        id: &str,
        extension: &str,
        source: &str,
    ) -> Result<Extracted, String> {
        match extension {
            "java" => self
                .java
                .as_mut()
                .ok_or("Java parser unavailable")?
                .analyze(id, extension, source),
            "py" | "pyi" => self
                .python
                .as_mut()
                .ok_or("Python parser unavailable")?
                .analyze(id, extension, source),
            "c" | "h" | "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" | "inl" | "ipp" => self
                .c_family
                .as_mut()
                .ok_or("C/C++ parser unavailable")?
                .analyze(id, extension, source),
            "go" => self
                .go
                .as_mut()
                .ok_or("Go parser unavailable")?
                .analyze(id, extension, source),
            "rs" => self
                .rust
                .as_mut()
                .ok_or("Rust parser unavailable")?
                .analyze(id, extension, source),
            "cs" => self
                .csharp
                .as_mut()
                .ok_or("C# parser unavailable")?
                .analyze(id, extension, source),
            _ => {
                let parsed = self.js.parse(extension, source)?;
                Ok(Extracted {
                    has_errors: parsed.has_errors,
                    imports: parsed
                        .imports
                        .into_iter()
                        .map(|import| {
                            let resolution = if import.literal {
                                resolver::resolve(id, &import.specifier, self.context.known)
                            } else {
                                Resolution::Unresolved(
                                    "动态表达式无法静态确定目标；未执行项目代码。",
                                )
                            };
                            ResolvedImport {
                                specifier: import.specifier,
                                kind: import.kind,
                                resolution,
                            }
                        })
                        .collect(),
                })
            }
        }
    }
}
