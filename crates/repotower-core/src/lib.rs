//! Offline, read-only source dependency analysis. All public JSON uses camelCase.
mod graph;
mod languages;
pub mod model;
mod parser;
mod resolver;
mod scanner;

pub use graph::impact;
use model::{DependencyEdge, ModuleAnalysis, RepositoryAnalysis, ScanProgress, UnresolvedImport};
use resolver::Resolution;
use scanner::Warnings;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::Read,
    path::Path,
};

const MAX_EDGES: usize = 100_000;

pub fn analyze(
    root: impl AsRef<Path>,
    mut progress: impl FnMut(ScanProgress),
) -> Result<RepositoryAnalysis, String> {
    let root = root
        .as_ref()
        .canonicalize()
        .map_err(|error| format!("无法打开项目文件夹：{error}"))?;
    let mut warnings = Warnings::default();
    let files = scanner::scan(&root, &mut progress, &mut warnings)?;
    let mut modules = Vec::new();
    let mut sources = HashMap::new();
    let mut total_read_bytes = 0u64;
    let total = files.len();
    progress(ScanProgress::new("parse", 0, Some(total)));
    for file in files {
        let mut bytes = Vec::new();
        let read = File::open(&file.path).and_then(|input| {
            input
                .take(scanner::MAX_FILE_BYTES + 1)
                .read_to_end(&mut bytes)
        });
        if let Err(error) = read {
            warnings.push(format!("无法读取 {}：{error}", file.id));
            continue;
        }
        if bytes.len() as u64 > scanner::MAX_FILE_BYTES {
            warnings.push(format!("文件扫描后变大，已跳过：{}", file.id));
            continue;
        }
        // Recheck actual bytes: files may grow after the scanner reads metadata.
        if total_read_bytes + bytes.len() as u64 > scanner::MAX_TOTAL_BYTES {
            warnings.push("读取的源码达到 128 MiB 安全限制；文件可能在扫描期间发生变化，结果仅代表已读取部分。");
            break;
        }
        total_read_bytes += bytes.len() as u64;
        let source = match String::from_utf8(bytes) {
            Ok(source) => source,
            Err(_) => {
                warnings.push(format!("文件不是 UTF-8，已跳过：{}", file.id));
                continue;
            }
        };
        let extension = file
            .path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let directory = file
            .id
            .rsplit_once('/')
            .map(|(directory, _)| directory.to_string())
            .unwrap_or_else(|| ".".into());
        let file_name = file.id.rsplit('/').next().unwrap_or(&file.id).to_string();
        // Non-empty physical lines; comments count. A transparent size proxy, not executable LOC.
        let lines_of_code = source
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count();
        sources.insert(file.id.clone(), source);
        modules.push(ModuleAnalysis {
            id: file.id.clone(),
            relative_path: file.id,
            file_name,
            extension,
            directory,
            lines_of_code,
            imports: vec![],
            imported_by: vec![],
            fan_in: 0,
            fan_out: 0,
            dependency_depth: 0,
            blast_radius: 0,
            blast_ratio: 0.0,
            cycle_id: None,
            is_entry_like: false,
            unresolved_imports: vec![],
        });
    }
    let known: HashSet<String> = modules.iter().map(|module| module.id.clone()).collect();
    let context = languages::Context::new(&root, &known, &sources);
    let mut analyzers = languages::Analyzers::new(&context)?;
    let mut edges = Vec::new();
    let mut edge_pairs = HashSet::new();
    let mut external_count = 0;
    let mut js_external_count = 0;
    let mut edge_limit_warning = false;
    for (index, module) in modules.iter_mut().enumerate() {
        let parsed = match analyzers.analyze(&module.id, &module.extension, &sources[&module.id]) {
            Ok(parsed) => parsed,
            Err(error) => {
                warnings.push(format!("无法解析 {}：{error}", module.id));
                continue;
            }
        };
        if parsed.has_errors {
            warnings.push(format!("{} 包含解析错误，依赖结果可能不完整。", module.id));
        }
        let mut seen_imports = HashSet::new();
        for import in parsed.imports {
            if !seen_imports.insert((
                import.specifier.clone(),
                import.kind,
                import.resolution.clone(),
            )) {
                continue;
            }
            match import.resolution {
                Resolution::Internal(target) => {
                    if !known.contains(&target) {
                        continue;
                    }
                    if edge_pairs.contains(&(module.id.clone(), target.clone())) {
                        continue;
                    }
                    if edges.len() >= MAX_EDGES {
                        if !edge_limit_warning {
                            warnings.push(
                                "内部依赖边超过 100,000，后续边已省略；指标和影响结果可能低估。",
                            );
                            edge_limit_warning = true;
                        }
                        continue;
                    }
                    edge_pairs.insert((module.id.clone(), target.clone()));
                    edges.push(DependencyEdge {
                        source: module.id.clone(),
                        target,
                        kind: import.kind,
                    });
                }
                Resolution::External => {
                    external_count += 1;
                    if scanner::JS_EXTENSIONS.contains(&module.extension.as_str()) {
                        js_external_count += 1;
                    }
                }
                Resolution::Unresolved(reason) => {
                    module.unresolved_imports.push(UnresolvedImport {
                        specifier: import.specifier,
                        kind: import.kind,
                        reason: reason.into(),
                    })
                }
            }
        }
        module.unresolved_imports.sort_by(|a, b| {
            a.specifier
                .cmp(&b.specifier)
                .then_with(|| a.kind.cmp(&b.kind))
                .then_with(|| a.reason.cmp(&b.reason))
        });
        if index % 20 == 0 {
            progress(ScanProgress::new("parse", index + 1, Some(total)));
        }
    }
    progress(ScanProgress::new(
        "parse",
        modules.len(),
        Some(modules.len()),
    ));
    if context.config_limited() {
        warnings
            .push("项目配置超过读取限制（单文件 1 MiB、合计 16 MiB），部分模块路径可能无法解析。");
    }
    edges.sort_by(|a, b| {
        a.source
            .cmp(&b.source)
            .then_with(|| a.target.cmp(&b.target))
    });
    let metrics = graph::calculate(&mut modules, &edges, &mut progress);
    let name = root
        .file_name()
        .unwrap_or(root.as_os_str())
        .to_string_lossy()
        .to_string();
    let unresolved_count = modules
        .iter()
        .map(|module| module.unresolved_imports.len())
        .sum();
    if js_external_count > 0 {
        warnings.push("包名导入视为外部依赖；tsconfig paths、工作区包和 package.json exports 不在本版解析范围。".to_string());
    }
    if modules.is_empty() {
        warnings.push("没有找到支持的源码文件。支持 Java、Python、C/C++、Go、Rust、C# 和 JS/TS。");
    }
    if modules.iter().any(|m| {
        matches!(
            m.extension.as_str(),
            "c" | "h" | "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" | "inl" | "ipp"
        )
    }) {
        warnings.push("C/C++ 分析静态包含和项目根 compile_commands.json 中的显式搜索路径；不运行编译器或预处理器，不继承头文件编译配置，条件分支均保留。无法确定的包含会标为未解析。");
    }
    if modules.iter().any(|m| m.extension == "go") {
        warnings.push("Go 包导入展开为本地包的生产源码文件；不执行 go 命令，也不应用构建标签、平台筛选或 replace 指令。");
    }
    if modules.iter().any(|m| m.extension == "rs") {
        warnings.push("Rust 分析静态模块声明和引用；不展开宏、不执行构建脚本、不应用 cfg 条件或非标准 Cargo 目标配置。");
    }
    if modules
        .iter()
        .any(|m| matches!(m.extension.as_str(), "java" | "cs"))
    {
        warnings.push("Java/C# 根据源码声明和类型引用匹配本地文件；不运行构建工具，不解析外部包、反射或生成代码，重复或不明确的类型不会猜测连线。");
    }
    progress(ScanProgress::new(
        "complete",
        modules.len(),
        Some(modules.len()),
    ));
    Ok(RepositoryAnalysis {
        repository_name: name.clone(),
        root_display_name: name,
        total_modules: modules.len(),
        total_edges: edges.len(),
        unresolved_count,
        external_count,
        cycle_count: metrics.cycles,
        stability_score: metrics.stability,
        stability_breakdown: metrics.breakdown,
        modules,
        edges,
        critical_modules: metrics.critical,
        warnings: warnings.finish(),
    })
}
