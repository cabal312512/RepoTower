use crate::model::DependencyKind;
use tree_sitter::{Node, Parser};

#[derive(Debug)]
pub struct Import {
    pub specifier: String,
    pub kind: DependencyKind,
    pub literal: bool,
}
pub struct Parsed {
    pub imports: Vec<Import>,
    pub has_errors: bool,
}

pub struct Parsers {
    js: Parser,
    ts: Parser,
    tsx: Parser,
}
impl Parsers {
    pub fn new() -> Result<Self, String> {
        fn parser(language: tree_sitter::Language) -> Result<Parser, String> {
            let mut parser = Parser::new();
            parser
                .set_language(&language)
                .map_err(|error| error.to_string())?;
            Ok(parser)
        }
        Ok(Self {
            js: parser(tree_sitter_javascript::LANGUAGE.into())?,
            ts: parser(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())?,
            tsx: parser(tree_sitter_typescript::LANGUAGE_TSX.into())?,
        })
    }

    pub fn parse(&mut self, extension: &str, source: &str) -> Result<Parsed, String> {
        let parser = match extension {
            "ts" | "mts" | "cts" => &mut self.ts,
            "tsx" => &mut self.tsx,
            _ => &mut self.js,
        };
        let tree = parser.parse(source, None).ok_or("解析被中止。")?;
        let mut imports = Vec::new();
        let mut cursor = tree.walk();
        loop {
            let node = cursor.node();
            match node.kind() {
                "import_statement" | "export_statement" => {
                    if let Some(source_node) = node.child_by_field_name("source") {
                        add_import(
                            &mut imports,
                            source_node,
                            source,
                            if node.kind() == "export_statement" {
                                DependencyKind::Export
                            } else {
                                DependencyKind::Static
                            },
                        );
                    }
                }
                "import_require_clause" => {
                    if let Some(source_node) = node.child_by_field_name("source") {
                        add_import(&mut imports, source_node, source, DependencyKind::Require);
                    }
                }
                "call_expression" => {
                    if let Some(function) = node.child_by_field_name("function") {
                        let name = text(function, source);
                        let kind = if function.kind() == "import" {
                            Some(DependencyKind::Dynamic)
                        } else if function.kind() == "identifier" && name == "require" {
                            Some(DependencyKind::Require)
                        } else {
                            None
                        };
                        if let Some(kind) = kind {
                            if let Some(argument) =
                                node.child_by_field_name("arguments").and_then(|arguments| {
                                    let mut cursor = arguments.walk();
                                    let first = arguments
                                        .named_children(&mut cursor)
                                        .find(|child| child.kind() != "comment");
                                    first
                                })
                            {
                                add_import(&mut imports, argument, source, kind);
                            }
                        }
                    }
                }
                _ => {}
            }
            if cursor.goto_first_child() {
                continue;
            }
            loop {
                if cursor.goto_next_sibling() {
                    break;
                }
                if !cursor.goto_parent() {
                    return Ok(Parsed {
                        imports,
                        has_errors: tree.root_node().has_error(),
                    });
                }
            }
        }
    }
}

fn text<'a>(node: Node<'_>, source: &'a str) -> &'a str {
    source.get(node.byte_range()).unwrap_or("")
}

fn add_import(imports: &mut Vec<Import>, node: Node<'_>, source: &str, kind: DependencyKind) {
    let raw = text(node, source);
    let mut cursor = node.walk();
    let static_template = node.kind() == "template_string"
        && !node
            .named_children(&mut cursor)
            .any(|child| child.kind() == "template_substitution");
    let literal = node.kind() == "string" || static_template;
    let decoded = if literal { decode_string(raw) } else { None };
    imports.push(Import {
        specifier: decoded
            .clone()
            .unwrap_or_else(|| raw.chars().take(160).collect()),
        kind,
        literal: decoded.is_some(),
    });
}

/// Decode JavaScript string escapes without evaluating any source code.
fn decode_string(raw: &str) -> Option<String> {
    let quote = raw.chars().next()?;
    if !matches!(quote, '\'' | '"' | '`') || !raw.ends_with(quote) || raw.len() < 2 {
        return None;
    }
    let mut chars = raw[1..raw.len() - 1].chars();
    let mut out = String::new();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next()? {
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            'b' => out.push('\u{0008}'),
            'f' => out.push('\u{000c}'),
            'v' => out.push('\u{000b}'),
            '0' => out.push('\0'),
            '\n' => {}
            '\r' => {
                if chars.clone().next() == Some('\n') {
                    chars.next();
                }
            }
            'x' => {
                let value = chars.by_ref().take(2).collect::<String>();
                if value.len() != 2 {
                    return None;
                }
                out.push(char::from_u32(u32::from_str_radix(&value, 16).ok()?)?);
            }
            'u' => {
                let value = if chars.clone().next() == Some('{') {
                    chars.next();
                    let mut value = String::new();
                    loop {
                        let next = chars.next()?;
                        if next == '}' {
                            break;
                        }
                        value.push(next);
                        if value.len() > 6 {
                            return None;
                        }
                    }
                    value
                } else {
                    let value = chars.by_ref().take(4).collect::<String>();
                    if value.len() != 4 {
                        return None;
                    }
                    value
                };
                out.push(char::from_u32(u32::from_str_radix(&value, 16).ok()?)?);
            }
            // Legacy octal escapes are intentionally not guessed.
            '1'..='9' => return None,
            other => out.push(other),
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_import_forms_and_comments() {
        let source = r#"
          // import './not-real';
          const comment = "require('./not-real')";
          import type { X } from './types';
          import './side-effect';
          export { Y } from './barrel';
          export * from './all';
          const a = import('./lazy');
          const b = require('./legacy');
          const c = import(`./template`);
          const d = import(`./${name}`);
          const e = require(variable);
          object.require('./not-real');
        "#;
        let parsed = Parsers::new().unwrap().parse("ts", source).unwrap();
        assert!(!parsed.has_errors);
        assert_eq!(parsed.imports.len(), 9);
        assert_eq!(parsed.imports.iter().filter(|i| !i.literal).count(), 2);
        assert_eq!(parsed.imports[0].specifier, "./types");
    }
    #[test]
    fn parses_tsx_and_jsx() {
        for extension in ["tsx", "jsx"] {
            let parsed = Parsers::new()
                .unwrap()
                .parse(
                    extension,
                    "import React from 'react'; export const View = () => <div>Hello</div>",
                )
                .unwrap();
            assert!(!parsed.has_errors);
            assert_eq!(parsed.imports.len(), 1);
        }
    }
    #[test]
    fn malformed_file_still_returns_visible_imports() {
        let parsed = Parsers::new()
            .unwrap()
            .parse("js", "import './visible'; const = ;")
            .unwrap();
        assert!(parsed.has_errors);
        assert_eq!(parsed.imports[0].specifier, "./visible");
    }
    #[test]
    fn handles_string_escapes() {
        assert_eq!(
            decode_string(r"'./f\x6fo\u002fbar'"),
            Some("./foo/bar".into())
        );
        assert_eq!(decode_string("'broken"), None);
    }
    #[test]
    fn ignores_magic_comments_inside_dynamic_imports() {
        let parsed = Parsers::new()
            .unwrap()
            .parse(
                "ts",
                "import(/* webpackChunkName: 'lazy' */ './lazy'); require(/* note */ './legacy');",
            )
            .unwrap();
        assert_eq!(parsed.imports.len(), 2);
        assert_eq!(parsed.imports[0].specifier, "./lazy");
        assert_eq!(parsed.imports[1].specifier, "./legacy");
        assert!(parsed.imports.iter().all(|import| import.literal));
    }
}
