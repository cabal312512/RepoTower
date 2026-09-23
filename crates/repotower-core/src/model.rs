use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "lowercase")]
pub enum DependencyKind {
    Static,
    Export,
    Dynamic,
    Require,
    Include,
    Module,
    Use,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnresolvedImport {
    pub specifier: String,
    pub kind: DependencyKind,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleAnalysis {
    pub id: String,
    pub relative_path: String,
    pub file_name: String,
    pub extension: String,
    pub directory: String,
    pub lines_of_code: usize,
    pub imports: Vec<String>,
    pub imported_by: Vec<String>,
    pub fan_in: usize,
    pub fan_out: usize,
    pub dependency_depth: usize,
    pub blast_radius: usize,
    pub blast_ratio: f64,
    pub cycle_id: Option<usize>,
    pub is_entry_like: bool,
    pub unresolved_imports: Vec<UnresolvedImport>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyEdge {
    pub source: String,
    pub target: String,
    pub kind: DependencyKind,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StabilityBreakdown {
    pub average_blast_ratio: f64,
    pub max_blast_ratio: f64,
    pub cycle_ratio: f64,
    pub concentration: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryAnalysis {
    pub repository_name: String,
    pub root_display_name: String,
    pub total_modules: usize,
    pub total_edges: usize,
    pub unresolved_count: usize,
    pub external_count: usize,
    pub cycle_count: usize,
    pub stability_score: f64,
    pub stability_breakdown: StabilityBreakdown,
    pub modules: Vec<ModuleAnalysis>,
    pub edges: Vec<DependencyEdge>,
    pub critical_modules: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImpactResult {
    pub removed_module_id: String,
    pub total_affected: usize,
    pub affected_ratio: f64,
    pub direct_affected: usize,
    pub transitive_affected: usize,
    pub max_cascade_depth: usize,
    pub waves: Vec<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub stage: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<usize>,
}

impl ScanProgress {
    pub fn new(stage: &str, completed: usize, total: Option<usize>) -> Self {
        Self {
            stage: stage.to_string(),
            completed: Some(completed),
            total,
        }
    }
}
