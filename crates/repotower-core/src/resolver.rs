use crate::scanner::JS_EXTENSIONS as EXTENSIONS;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Resolution {
    Internal(String),
    External,
    Unresolved(&'static str),
}

pub fn resolve(importer: &str, specifier: &str, known: &HashSet<String>) -> Resolution {
    if specifier.is_empty() {
        return Resolution::Unresolved("空模块路径。");
    }
    if !specifier.starts_with('.') {
        if specifier.starts_with('/')
            || specifier.starts_with('\\')
            || (specifier.as_bytes().get(1) == Some(&b':')
                && specifier.as_bytes()[0].is_ascii_alphabetic())
            || specifier.starts_with('#')
            || specifier.starts_with("@/")
            || specifier.starts_with("~/")
        {
            return Resolution::Unresolved("尚不解析绝对路径或项目路径别名。");
        }
        return Resolution::External;
    }
    if specifier != "."
        && specifier != ".."
        && !specifier.starts_with("./")
        && !specifier.starts_with("../")
    {
        return Resolution::Unresolved("不支持的相对路径格式。");
    }
    if specifier.contains(['?', '#', '\0', '\\']) {
        return Resolution::Unresolved("带查询参数、片段、空字符或反斜杠的导入未解析。");
    }
    let directory = importer
        .rsplit_once('/')
        .map(|(directory, _)| directory)
        .unwrap_or("");
    let mut components: Vec<&str> = directory
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    for component in specifier.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                if components.pop().is_none() {
                    return Resolution::Unresolved("路径位于所选项目之外。");
                }
            }
            other => components.push(other),
        }
    }
    let base = components.join("/");
    let directory_only = specifier.ends_with('/')
        || matches!(specifier, "." | "..")
        || specifier.ends_with("/.")
        || specifier.ends_with("/..");
    if !directory_only && known.contains(&base) {
        return Resolution::Internal(base);
    }
    // TypeScript projects commonly write a .js runtime suffix in imports of .ts source files.
    if let Some(stem) = base.strip_suffix(".js").filter(|_| !directory_only) {
        for extension in ["ts", "tsx"] {
            let candidate = format!("{stem}.{extension}");
            if known.contains(&candidate) {
                return Resolution::Internal(candidate);
            }
        }
    }
    if let Some(stem) = base.strip_suffix(".jsx").filter(|_| !directory_only) {
        let candidate = format!("{stem}.tsx");
        if known.contains(&candidate) {
            return Resolution::Internal(candidate);
        }
    }
    for (runtime, source) in [(".mjs", ".mts"), (".cjs", ".cts")] {
        if let Some(stem) = base.strip_suffix(runtime).filter(|_| !directory_only) {
            let candidate = format!("{stem}{source}");
            if known.contains(&candidate) {
                return Resolution::Internal(candidate);
            }
        }
    }
    // A dot may be part of an extensionless basename (e.g. user.model.ts).
    // Directory-only imports must not accidentally resolve a neighboring file.
    if !directory_only {
        for extension in EXTENSIONS {
            let candidate = format!("{base}.{extension}");
            if known.contains(&candidate) {
                return Resolution::Internal(candidate);
            }
        }
    }
    for extension in EXTENSIONS {
        let candidate = if base.is_empty() {
            format!("index.{extension}")
        } else {
            format!("{base}/index.{extension}")
        };
        if known.contains(&candidate) {
            return Resolution::Internal(candidate);
        }
    }
    Resolution::Unresolved(
        "在已扫描文件中未找到模块；它可能不存在、被忽略或使用尚不支持的解析规则。",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extensions_indexes_and_boundary() {
        let known = [
            "src/a.ts",
            "src/utils/index.ts",
            "index.ts",
            "src/view.tsx",
            "src/user.model.ts",
            "src/utils.ts",
        ]
        .map(str::to_string)
        .into_iter()
        .collect();
        for (importer, path, expected) in [
            ("src/main.ts", "./a.js", "src/a.ts"),
            ("src/main.ts", "..", "index.ts"),
            ("src/main.ts", "./view.jsx", "src/view.tsx"),
            ("src/main.ts", "./user.model", "src/user.model.ts"),
            ("src/main.ts", "./utils/", "src/utils/index.ts"),
            ("src/main.ts", "./utils", "src/utils.ts"),
        ] {
            assert!(
                matches!(resolve(importer, path, &known), Resolution::Internal(actual) if actual == expected)
            );
        }
        assert!(matches!(
            resolve("index.ts", "../escape", &known),
            Resolution::Unresolved(_)
        ));
        assert!(matches!(
            resolve("index.ts", "@/alias", &known),
            Resolution::Unresolved(_)
        ));
        assert!(matches!(
            resolve("index.ts", "C:/outside", &known),
            Resolution::Unresolved(_)
        ));
        assert!(matches!(
            resolve("index.ts", "@scope/package", &known),
            Resolution::External
        ));
    }
}
