use crate::model::ScanProgress;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

pub const JS_EXTENSIONS: [&str; 8] = ["ts", "tsx", "js", "jsx", "mjs", "cjs", "mts", "cts"];
pub const EXTENSIONS: [&str; 24] = [
    "ts", "tsx", "js", "jsx", "mjs", "cjs", "mts", "cts", "java", "py", "pyi", "c", "h", "cc",
    "cpp", "cxx", "hpp", "hh", "hxx", "inl", "ipp", "go", "rs", "cs",
];
pub const MAX_FILES: usize = 10_000;
pub const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const MAX_TOTAL_BYTES: u64 = 128 * 1024 * 1024;
const MAX_ENTRIES: usize = 200_000;
const EXCLUDED_DIRS: [&str; 28] = [
    ".git",
    "node_modules",
    "dist",
    "build",
    "out",
    "target",
    ".next",
    "coverage",
    ".cache",
    ".tools",
    ".tmp",
    ".turbo",
    ".nuxt",
    ".output",
    "vendor",
    "release",
    ".venv",
    "venv",
    "bower_components",
    ".parcel-cache",
    "__pycache__",
    ".mypy_cache",
    ".pytest_cache",
    ".ruff_cache",
    ".gradle",
    ".idea",
    ".vs",
    "obj",
];

#[derive(Default)]
pub struct Warnings {
    messages: Vec<String>,
    suppressed: usize,
}
impl Warnings {
    pub fn push(&mut self, warning: impl Into<String>) {
        if self.messages.len() < 300 {
            self.messages.push(warning.into());
        } else {
            self.suppressed += 1;
        }
    }
    pub fn finish(mut self) -> Vec<String> {
        if self.suppressed > 0 {
            self.messages
                .push(format!("另有 {} 条同类警告未显示。", self.suppressed));
        }
        self.messages
    }
}

pub struct ScannedFile {
    pub id: String,
    pub path: PathBuf,
}

pub fn scan(
    root: &Path,
    progress: &mut impl FnMut(ScanProgress),
    warnings: &mut Warnings,
) -> Result<Vec<ScannedFile>, String> {
    if !root.is_dir() {
        return Err("请选择存在且可读取的项目文件夹。".into());
    }
    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(false)
        .follow_links(false)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .parents(false)
        .require_git(false)
        .ignore(true)
        .add_custom_ignore_filename(".repotowerignore")
        .sort_by_file_path(|a, b| a.cmp(b))
        .filter_entry(|entry| {
            entry.depth() == 0
                || !entry.file_type().is_some_and(|kind| kind.is_dir())
                || !EXCLUDED_DIRS.iter().any(|name| {
                    entry
                        .file_name()
                        .to_string_lossy()
                        .eq_ignore_ascii_case(name)
                })
        });
    let mut files = Vec::new();
    let mut bytes = 0u64;
    progress(ScanProgress::new("scan", 0, None));
    for (index, entry) in builder.build().enumerate() {
        if index >= MAX_ENTRIES {
            warnings.push(format!(
                "目录项超过 {MAX_ENTRIES}，扫描提前结束；当前结果仅代表已扫描部分。"
            ));
            break;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                warnings.push(format!("无法读取目录项：{error}"));
                continue;
            }
        };
        if let Some(error) = entry.error() {
            warnings.push(format!("忽略规则错误：{error}"));
        }
        if entry.file_type().is_some_and(|kind| kind.is_symlink()) {
            warnings.push(format!(
                "为避免越界和循环，已跳过符号链接：{}",
                display_relative(root, entry.path())
            ));
            continue;
        }
        if !entry.file_type().is_some_and(|kind| kind.is_file()) {
            continue;
        }
        let extension = entry
            .path()
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !EXTENSIONS.contains(&extension.as_str()) {
            continue;
        }
        let size = match entry.metadata() {
            Ok(meta) => meta.len(),
            Err(error) => {
                warnings.push(format!("无法读取文件属性：{error}"));
                continue;
            }
        };
        if size > MAX_FILE_BYTES {
            warnings.push(format!(
                "文件超过 2 MiB，已跳过：{}",
                display_relative(root, entry.path())
            ));
            continue;
        }
        if files.len() >= MAX_FILES || bytes + size > MAX_TOTAL_BYTES {
            warnings.push(
                "达到安全限制（10,000 个模块或 128 MiB 源码），结果仅代表已扫描部分。".to_string(),
            );
            break;
        }
        let relative = entry
            .path()
            .strip_prefix(root)
            .map_err(|_| "无法计算项目相对路径。")?;
        let Some(id) = relative.to_str() else {
            warnings.push("已跳过非 Unicode 文件路径。");
            continue;
        };
        files.push(ScannedFile {
            id: id.replace('\\', "/"),
            path: entry.path().to_path_buf(),
        });
        bytes += size;
        if files.len() % 50 == 0 {
            progress(ScanProgress::new("scan", files.len(), None));
        }
    }
    files.sort_by(|a, b| a.id.cmp(&b.id));
    progress(ScanProgress::new("scan", files.len(), Some(files.len())));
    Ok(files)
}

fn display_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
