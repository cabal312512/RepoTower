// Source lineage: rt-origin-77f1f64c-6a08-44e8-87a1-19728239c117
// 代码全是AI生成的 连提示词也是ai写的
use eframe::egui::{pos2, vec2, Pos2, Vec2};
use repotower_core::model::{ImpactResult, RepositoryAnalysis};
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub const NODE_SIZE: Vec2 = vec2(158.0, 44.0);
pub const MAX_VISIBLE: usize = 250;
pub const WAVE_SECONDS: f32 = 0.65;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum View {
    #[default]
    All,
    Nearby,
    Impact,
}

#[derive(Clone, Debug)]
pub struct Node {
    pub id: String,
    pub module: usize,
    pub pos: Pos2,
    pub wave: Option<usize>,
}
#[derive(Clone, Debug)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub causal: bool,
}
#[derive(Clone, Debug, Default)]
pub struct Layout {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub columns: Vec<(usize, f32)>,
    pub size: Vec2,
    pub hidden: usize,
    pub untouched: usize,
}

impl Layout {
    pub fn build(
        analysis: &RepositoryAnalysis,
        view: View,
        selected: Option<&str>,
        neighbor: Option<&str>,
        impact: Option<&ImpactResult>,
        unavailable: &BTreeSet<String>,
    ) -> Self {
        let modules: HashMap<&str, usize> = analysis
            .modules
            .iter()
            .enumerate()
            .map(|(i, m)| (m.id.as_str(), i))
            .collect();
        let waves: HashMap<&str, usize> = impact
            .into_iter()
            .flat_map(|r| {
                r.waves
                    .iter()
                    .enumerate()
                    .flat_map(|(wave, ids)| ids.iter().map(move |id| (id.as_str(), wave)))
            })
            .collect();
        let mut ranks: HashMap<&str, usize> = HashMap::new();
        let mut candidates: Vec<&str>;
        if view == View::Impact && impact.is_some() {
            ranks.clone_from(&waves);
            candidates = waves.keys().copied().collect();
            candidates.sort_by_key(|id| (waves[id], *id));
        } else if view == View::Nearby && neighbor.is_some_and(|id| modules.contains_key(id)) {
            let id = neighbor.unwrap();
            ranks.insert(id, 1);
            for edge in &analysis.edges {
                if edge.source == id && edge.target != id {
                    ranks.insert(&edge.target, 0);
                }
            }
            for edge in &analysis.edges {
                if edge.target == id && edge.source != id {
                    ranks.insert(&edge.source, 2);
                }
            }
            candidates = ranks.keys().copied().collect();
            candidates.sort_by_key(|other| (*other != id, *other));
        } else {
            ranks.extend(
                analysis
                    .modules
                    .iter()
                    .map(|m| (m.id.as_str(), m.dependency_depth)),
            );
            candidates = modules.keys().copied().collect();
            candidates.sort();
            let mut anchors = Vec::new();
            for id in [impact.map(|r| r.removed_module_id.as_str()), selected]
                .into_iter()
                .flatten()
            {
                if modules.contains_key(id) && !anchors.contains(&id) {
                    anchors.push(id);
                }
            }
            candidates.retain(|id| !anchors.contains(id));
            anchors.extend(candidates);
            candidates = anchors;
        }
        let hidden = candidates.len().saturating_sub(MAX_VISIBLE);
        candidates.truncate(MAX_VISIBLE);
        let visible: BTreeSet<&str> = candidates.iter().copied().collect();
        // Dependency -> consumer. No edge is inferred by the renderer.
        let real_edges: BTreeSet<(&str, &str)> = analysis
            .edges
            .iter()
            .filter(|e| visible.contains(e.source.as_str()) && visible.contains(e.target.as_str()))
            .map(|e| (e.target.as_str(), e.source.as_str()))
            .collect();
        let mut layers: BTreeMap<usize, Vec<&str>> = BTreeMap::new();
        candidates.sort();
        for id in &candidates {
            layers.entry(ranks[id]).or_default().push(id);
        }
        let indices: Vec<usize> = layers.keys().copied().collect();
        let mut incoming: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut outgoing: HashMap<&str, Vec<&str>> = HashMap::new();
        for &(from, to) in &real_edges {
            incoming.entry(to).or_default().push(from);
            outgoing.entry(from).or_default().push(to);
        }
        for pass in 0..4 {
            let mut rows: HashMap<&str, f32> = layers
                .values()
                .flat_map(|l| {
                    l.iter()
                        .enumerate()
                        .map(|(i, id)| (*id, i as f32 - (l.len() as f32 - 1.0) / 2.0))
                })
                .collect();
            let mut order = indices.clone();
            if pass % 2 == 1 {
                order.reverse();
            }
            let neighbors = if pass % 2 == 0 { &incoming } else { &outgoing };
            for index in order {
                let score = |id: &str| {
                    let adjacent: Vec<_> = neighbors
                        .get(id)
                        .into_iter()
                        .flatten()
                        .filter(|other| ranks[**other] != index)
                        .collect();
                    if adjacent.is_empty() {
                        rows[id]
                    } else {
                        adjacent.iter().map(|id| rows[**id]).sum::<f32>() / adjacent.len() as f32
                    }
                };
                let layer = layers.get_mut(&index).unwrap();
                layer.sort_by(|a, b| score(a).total_cmp(&score(b)).then_with(|| a.cmp(b)));
                for (row, id) in layer.iter().enumerate() {
                    rows.insert(id, row as f32 - (layer.len() as f32 - 1.0) / 2.0);
                }
            }
        }
        let height =
            (layers.values().map(Vec::len).max().unwrap_or(1) as f32 * 66.0 + 72.0).max(160.0);
        let mut nodes = Vec::new();
        let mut columns = Vec::new();
        for (column, (index, layer)) in layers.iter().enumerate() {
            let x = 44.0 + column as f32 * 210.0;
            columns.push((*index, x));
            for (row, id) in layer.iter().enumerate() {
                nodes.push(Node {
                    id: (*id).to_owned(),
                    module: modules[id],
                    wave: waves.get(id).copied(),
                    pos: pos2(
                        x,
                        height / 2.0 + (row as f32 - (layer.len() as f32 - 1.0) / 2.0) * 66.0 - 6.0,
                    ),
                });
            }
        }
        let by_id: HashMap<&str, usize> = nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id.as_str(), i))
            .collect();
        let edges = real_edges
            .iter()
            .map(|&(a, b)| Edge {
                from: by_id[a],
                to: by_id[b],
                causal: waves
                    .get(a)
                    .zip(waves.get(b))
                    .is_some_and(|(a, b)| *b == *a + 1),
            })
            .collect();
        let untouched = analysis
            .modules
            .iter()
            .filter(|m| !waves.contains_key(m.id.as_str()) && !unavailable.contains(&m.id))
            .count();
        Self {
            nodes,
            edges,
            columns,
            size: vec2(
                (88.0 + indices.len().saturating_sub(1) as f32 * 210.0 + NODE_SIZE.x).max(250.0),
                height,
            ),
            hidden,
            untouched,
        }
    }
}

pub fn route(from: Pos2, to: Pos2) -> [Pos2; 4] {
    let forward = to.x > from.x;
    let start = from + vec2(NODE_SIZE.x, NODE_SIZE.y / 2.0);
    let end = to
        + vec2(
            if forward { -5.0 } else { NODE_SIZE.x + 5.0 },
            NODE_SIZE.y / 2.0,
        );
    let reach = if forward {
        ((end.x - start.x) * 0.48).max(36.0)
    } else if to.x == from.x {
        38.0 + (to.y - from.y).abs() * 0.1
    } else {
        72.0
    };
    [
        start,
        start + vec2(reach, 0.0),
        end + vec2(if forward { -reach } else { reach }, 0.0),
        end,
    ]
}
pub fn curve_point(p: [Pos2; 4], t: f32) -> Pos2 {
    let u = 1.0 - t;
    (p[0].to_vec2() * u.powi(3)
        + p[1].to_vec2() * 3.0 * u * u * t
        + p[2].to_vec2() * 3.0 * u * t * t
        + p[3].to_vec2() * t.powi(3))
    .to_pos2()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub offset: Vec2,
    pub scale: f32,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            scale: 1.0,
        }
    }
}
impl Camera {
    pub fn fit(layout: &Layout, offsets: &HashMap<String, Vec2>, size: Vec2, report: bool) -> Self {
        let mut min = Pos2::ZERO;
        let mut max = layout.size.to_pos2();
        for n in &layout.nodes {
            let p = n.pos + offsets.get(&n.id).copied().unwrap_or_default();
            min = min.min(p - vec2(30.0, 30.0));
            max = max.max(p + NODE_SIZE + vec2(30.0, 30.0));
        }
        let bounds = max - min;
        let available = (size.y - if report { 116.0 } else { 86.0 }).max(140.0);
        let scale = 1.35_f32
            .min((size.x - 40.0) / bounds.x)
            .min(available / bounds.y)
            .max(0.01);
        Self {
            scale,
            offset: vec2(
                (size.x - bounds.x * scale) / 2.0 - min.x * scale,
                48.0 + (available - bounds.y * scale) / 2.0 - min.y * scale,
            ),
        }
    }
    pub fn zoom(&mut self, factor: f32, at: Vec2) {
        let new = (self.scale * factor).clamp(0.12, 3.5);
        self.offset = at - (at - self.offset) * (new / self.scale);
        self.scale = new;
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
    fn every_line_is_a_real_dependency_and_layout_is_stable() {
        let a = demo();
        let l = Layout::build(&a, View::All, None, None, None, &BTreeSet::new());
        assert_eq!(l.nodes.len(), 23);
        assert_eq!(l.edges.len(), 24);
        for e in &l.edges {
            assert!(a
                .edges
                .iter()
                .any(|real| real.target == l.nodes[e.from].id && real.source == l.nodes[e.to].id));
        }
        let again = Layout::build(&a, View::All, None, None, None, &BTreeSet::new());
        assert_eq!(
            l.nodes.iter().map(|n| (&n.id, n.pos)).collect::<Vec<_>>(),
            again
                .nodes
                .iter()
                .map(|n| (&n.id, n.pos))
                .collect::<Vec<_>>()
        );
    }
    #[test]
    fn impact_shows_bfs_waves_and_only_causal_edges_pulse() {
        let a = demo();
        let impact = repotower_core::impact(&a.modules, "src/core/config.ts", &[]).unwrap();
        let l = Layout::build(
            &a,
            View::Impact,
            None,
            None,
            Some(&impact),
            &BTreeSet::new(),
        );
        assert_eq!(l.nodes.len(), 14);
        assert_eq!(l.untouched, 9);
        for e in l.edges.iter().filter(|e| e.causal) {
            assert_eq!(
                l.nodes[e.to].wave.unwrap(),
                l.nodes[e.from].wave.unwrap() + 1
            );
        }
    }
    #[test]
    fn zoom_keeps_world_point_under_pointer() {
        let mut c = Camera {
            offset: vec2(40.0, 30.0),
            scale: 0.8,
        };
        let at = vec2(320.0, 170.0);
        let world = (at - c.offset) / c.scale;
        c.zoom(1.4, at);
        assert!(((at - c.offset) / c.scale - world).length() < 0.001);
    }
}
