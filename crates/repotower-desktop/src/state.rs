use repotower_core::{
    impact,
    model::{ImpactResult, RepositoryAnalysis},
};
use std::collections::BTreeSet;

#[derive(Clone, Default)]
struct Snapshot {
    unavailable: BTreeSet<String>,
    origins: Vec<String>,
    reports: Vec<ImpactResult>,
    review: Option<String>,
    selected: Option<String>,
}
#[derive(Default)]
pub struct Simulation {
    pub unavailable: BTreeSet<String>,
    pub origins: Vec<String>,
    pub reports: Vec<ImpactResult>,
    pub review: Option<String>,
    pub selected: Option<String>,
    pub live: Option<ImpactResult>,
    history: Vec<Snapshot>,
}
impl Simulation {
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            unavailable: self.unavailable.clone(),
            origins: self.origins.clone(),
            reports: self.reports.clone(),
            review: self.review.clone(),
            selected: self.selected.clone(),
        }
    }
    pub fn report(&self) -> Option<&ImpactResult> {
        self.live.as_ref().or_else(|| {
            self.review
                .as_ref()
                .and_then(|id| self.reports.iter().find(|r| r.removed_module_id == *id))
        })
    }
    pub fn preview(&self, a: &RepositoryAnalysis, id: &str) -> Option<ImpactResult> {
        if self.unavailable.contains(id) || self.live.is_some() {
            return None;
        }
        impact(
            &a.modules,
            id,
            &self.unavailable.iter().cloned().collect::<Vec<_>>(),
        )
        .ok()
    }
    pub fn begin(&mut self, a: &RepositoryAnalysis, id: &str) -> bool {
        let Some(result) = self.preview(a, id) else {
            return false;
        };
        self.history.push(self.snapshot());
        self.selected = Some(id.into());
        self.live = Some(result);
        true
    }
    pub fn finish(&mut self) {
        if let Some(result) = self.live.take() {
            if self.selected.as_ref() == Some(&result.removed_module_id) {
                self.selected = None;
            }
            self.unavailable
                .extend(result.waves.iter().flatten().cloned());
            self.origins.push(result.removed_module_id.clone());
            self.review = Some(result.removed_module_id.clone());
            self.reports.push(result);
        }
    }
    pub fn restore(&mut self, a: &RepositoryAnalysis, id: &str) -> bool {
        if self.live.is_some() {
            let is_origin = self.origins.iter().any(|origin| origin == id)
                || self
                    .live
                    .as_ref()
                    .is_some_and(|report| report.removed_module_id == id);
            if !is_origin {
                return false;
            }
            self.finish();
        }
        if self.live.is_some() || !self.origins.iter().any(|s| s == id) {
            return false;
        }
        // Recompute every remaining cut. Shared failures must stay unavailable.
        let remaining: Vec<_> = self
            .origins
            .iter()
            .filter(|s| s.as_str() != id)
            .cloned()
            .collect();
        let mut unavailable = BTreeSet::new();
        let mut reports = Vec::new();
        for origin in &remaining {
            let excluded: Vec<String> = unavailable
                .iter()
                .filter(|s| *s != origin)
                .cloned()
                .collect();
            let Ok(report) = impact(&a.modules, origin, &excluded) else {
                return false;
            };
            unavailable.extend(report.waves.iter().flatten().cloned());
            reports.push(report);
        }
        self.history.push(self.snapshot());
        self.origins = remaining;
        self.unavailable = unavailable;
        self.reports = reports;
        if !self
            .reports
            .iter()
            .any(|r| Some(&r.removed_module_id) == self.review.as_ref())
        {
            self.review = self.reports.last().map(|r| r.removed_module_id.clone());
        }
        true
    }
    pub fn can_undo(&self) -> bool {
        !self.history.is_empty() && self.live.is_none()
    }
    pub fn undo(&mut self) -> bool {
        if self.live.is_some() {
            return false;
        }
        if let Some(s) = self.history.pop() {
            self.unavailable = s.unavailable;
            self.origins = s.origins;
            self.reports = s.reports;
            self.review = s.review;
            self.selected = s.selected;
            true
        } else {
            false
        }
    }
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn demo() -> RepositoryAnalysis {
        repotower_core::analyze(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/demo-project"),
            |_| {},
        )
        .unwrap()
    }
    #[test]
    fn restore_an_earlier_cut_while_another_cut_is_still_playing() {
        let a = demo();
        let mut s = Simulation::default();
        s.begin(&a, "src/core/config.ts");
        s.finish();
        s.begin(&a, "src/ui/palette.ts");
        s.selected = Some("src/core/config.ts".into());
        assert!(s.restore(&a, "src/core/config.ts"));
        assert!(s.live.is_none());
        assert_eq!(s.origins, vec!["src/ui/palette.ts"]);
        assert!(s.unavailable.contains("src/app/main.ts"));
        assert!(!s.unavailable.contains("src/core/auth.ts"));
        assert!(s.undo());
        assert_eq!(s.origins.len(), 2);
    }
    #[test]
    fn cut_survives_selection_changes_and_can_be_restored() {
        let a = demo();
        let mut s = Simulation::default();
        assert!(s.begin(&a, "src/core/config.ts"));
        s.finish();
        s.selected = Some("src/utils/format.ts".into());
        assert_eq!(s.report().unwrap().total_affected, 13);
        s.selected = Some("src/core/config.ts".into());
        assert!(s.restore(&a, "src/core/config.ts"));
        assert!(s.unavailable.is_empty());
        assert!(s.undo());
        assert_eq!(s.unavailable.len(), 14);
        assert!(s.undo());
        assert!(s.unavailable.is_empty());
    }
    #[test]
    fn restoration_preserves_shared_failures_and_history() {
        let a = demo();
        let mut s = Simulation::default();
        assert!(s.begin(&a, "src/core/config.ts"));
        s.finish();
        assert!(s.begin(&a, "src/ui/palette.ts"));
        s.finish();
        assert!(s.restore(&a, "src/core/config.ts"));
        assert!(!s.unavailable.contains("src/core/auth.ts"));
        assert!(s.unavailable.contains("src/ui/theme.ts"));
        assert!(s.unavailable.contains("src/app/main.ts"));
        assert_eq!(s.reports.len(), 1);
        assert!(s.restore(&a, "src/ui/palette.ts"));
        assert!(s.unavailable.is_empty());
    }
    #[test]
    fn live_cut_can_be_restored_without_waiting_for_animation() {
        let a = demo();
        let mut s = Simulation::default();
        s.begin(&a, "src/core/config.ts");
        assert!(s.restore(&a, "src/core/config.ts"));
        assert!(s.live.is_none());
        assert!(s.unavailable.is_empty());
    }
}
