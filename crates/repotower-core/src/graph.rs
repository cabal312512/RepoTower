// Source lineage: rt-origin-77f1f64c-6a08-44e8-87a1-19728239c117
use crate::model::{
    DependencyEdge, ImpactResult, ModuleAnalysis, ScanProgress, StabilityBreakdown,
};
use petgraph::{algo::kosaraju_scc, graph::DiGraph};
use std::collections::{HashMap, HashSet, VecDeque};

pub struct GraphMetrics {
    pub cycles: usize,
    pub stability: f64,
    pub breakdown: StabilityBreakdown,
    pub critical: Vec<String>,
}

/// Every edge is importer -> dependency. No recursion is used in our graph passes.
pub fn calculate(
    modules: &mut [ModuleAnalysis],
    edges: &[DependencyEdge],
    progress: &mut impl FnMut(ScanProgress),
) -> GraphMetrics {
    let count = modules.len();
    let ids: HashMap<String, usize> = modules
        .iter()
        .enumerate()
        .map(|(index, module)| (module.id.clone(), index))
        .collect();
    let mut graph = DiGraph::<(), ()>::new();
    let nodes: Vec<_> = (0..count).map(|_| graph.add_node(())).collect();
    let mut forward = vec![Vec::new(); count];
    let mut reverse = vec![Vec::new(); count];
    for edge in edges {
        let source = ids[&edge.source];
        let target = ids[&edge.target];
        graph.add_edge(nodes[source], nodes[target], ());
        forward[source].push(target);
        reverse[target].push(source);
    }
    progress(ScanProgress::new("graph", 0, Some(count)));
    let mut components: Vec<Vec<usize>> = kosaraju_scc(&graph)
        .into_iter()
        .map(|group| {
            let mut members: Vec<_> = group.into_iter().map(|node| node.index()).collect();
            members.sort_unstable();
            members
        })
        .collect();
    components.sort_by_key(|members| members[0]);
    let component_count = components.len();
    let mut membership = vec![0; count];
    let mut cycle_id = vec![None; count];
    let mut cycles = 0;
    for (index, members) in components.iter().enumerate() {
        let is_cycle = members.len() > 1 || forward[members[0]].contains(&members[0]);
        if is_cycle {
            cycles += 1;
        }
        for &member in members {
            membership[member] = index;
            if is_cycle {
                cycle_id[member] = Some(cycles);
            }
        }
    }
    let mut component_forward = vec![Vec::new(); component_count];
    let mut component_reverse = vec![Vec::new(); component_count];
    let mut component_edges = HashSet::new();
    for (source, dependencies) in forward.iter().enumerate() {
        for &target in dependencies {
            let a = membership[source];
            let b = membership[target];
            if a != b && component_edges.insert((a, b)) {
                component_forward[a].push(b);
                component_reverse[b].push(a);
            }
        }
    }
    // Dependency sinks are depth zero; reverse Kahn order propagates maximum depth upward.
    let mut remaining_dependencies: Vec<_> = component_forward.iter().map(Vec::len).collect();
    let mut depth = vec![0usize; component_count];
    let mut queue: VecDeque<_> = (0..component_count)
        .filter(|&i| remaining_dependencies[i] == 0)
        .collect();
    while let Some(component) = queue.pop_front() {
        for &importer in &component_reverse[component] {
            depth[importer] = depth[importer].max(depth[component] + 1);
            remaining_dependencies[importer] -= 1;
            if remaining_dependencies[importer] == 0 {
                queue.push_back(importer);
            }
        }
    }
    // Exact transitive importer sets on the condensed DAG. At our 10,000-file bound this
    // takes <= 12.6 MB, avoiding a full BFS for every module on long chains.
    let words = component_count.div_ceil(64);
    let mut ancestors = vec![vec![0u64; words]; component_count];
    for (component, bits) in ancestors.iter_mut().enumerate() {
        bits[component / 64] |= 1u64 << (component % 64);
    }
    let mut remaining_importers: Vec<_> = component_reverse.iter().map(Vec::len).collect();
    queue = (0..component_count)
        .filter(|&i| remaining_importers[i] == 0)
        .collect();
    let mut processed = 0;
    while let Some(component) = queue.pop_front() {
        for &dependency in &component_forward[component] {
            // split_at_mut keeps the source bitset immutable while updating a distinct target.
            if component < dependency {
                let (left, right) = ancestors.split_at_mut(dependency);
                for (target, source) in right[0].iter_mut().zip(&left[component]) {
                    *target |= source;
                }
            } else {
                let (left, right) = ancestors.split_at_mut(component);
                for (target, source) in left[dependency].iter_mut().zip(&right[0]) {
                    *target |= source;
                }
            }
            remaining_importers[dependency] -= 1;
            if remaining_importers[dependency] == 0 {
                queue.push_back(dependency);
            }
        }
        processed += 1;
        if processed % 100 == 0 {
            progress(ScanProgress::new(
                "metrics",
                processed,
                Some(component_count),
            ));
        }
    }
    let mut blast = vec![0; component_count];
    for (index, bits) in ancestors.iter().enumerate() {
        let mut reachable = 0usize;
        for (word_index, &word) in bits.iter().enumerate() {
            let mut word = word;
            while word != 0 {
                let bit = word.trailing_zeros() as usize;
                reachable += components[word_index * 64 + bit].len();
                word &= word - 1;
            }
        }
        blast[index] = reachable.saturating_sub(1);
    }
    let paths: Vec<_> = modules.iter().map(|module| module.id.clone()).collect();
    for (index, module) in modules.iter_mut().enumerate() {
        module.imports = forward[index].iter().map(|&i| paths[i].clone()).collect();
        module.imports.sort();
        module.imported_by = reverse[index].iter().map(|&i| paths[i].clone()).collect();
        module.imported_by.sort();
        module.fan_out = forward[index].len();
        module.fan_in = reverse[index].len();
        module.is_entry_like = reverse[index].is_empty();
        module.dependency_depth = depth[membership[index]];
        module.blast_radius = blast[membership[index]];
        module.blast_ratio = ratio(module.blast_radius, count);
        module.cycle_id = cycle_id[index];
    }
    let average_blast_ratio = if count == 0 {
        0.0
    } else {
        modules.iter().map(|module| module.blast_ratio).sum::<f64>() / count as f64
    };
    let max_blast_ratio = modules
        .iter()
        .map(|module| module.blast_ratio)
        .fold(0.0f64, f64::max);
    let cycle_ratio = ratio(
        modules
            .iter()
            .filter(|module| module.cycle_id.is_some())
            .count(),
        count,
    );
    let concentration = ratio(
        modules
            .iter()
            .map(|module| module.fan_in)
            .max()
            .unwrap_or(0),
        count,
    );
    let penalty = 0.30 * average_blast_ratio
        + 0.30 * max_blast_ratio
        + 0.25 * cycle_ratio
        + 0.15 * concentration;
    let stability = ((100.0 * (1.0 - penalty)).clamp(0.0, 100.0) * 10.0).round() / 10.0;
    let mut critical: Vec<_> = modules.iter().collect();
    critical.sort_by(|a, b| {
        b.blast_radius
            .cmp(&a.blast_radius)
            .then_with(|| b.fan_in.cmp(&a.fan_in))
            .then_with(|| a.id.cmp(&b.id))
    });
    progress(ScanProgress::new(
        "metrics",
        component_count,
        Some(component_count),
    ));
    GraphMetrics {
        cycles,
        stability,
        breakdown: StabilityBreakdown {
            average_blast_ratio,
            max_blast_ratio,
            cycle_ratio,
            concentration,
        },
        critical: critical
            .into_iter()
            .take(12)
            .map(|module| module.id.clone())
            .collect(),
    }
}

pub fn impact(
    modules: &[ModuleAnalysis],
    module_id: &str,
    excluded: &[String],
) -> Result<ImpactResult, String> {
    let ids: HashMap<&str, usize> = modules
        .iter()
        .enumerate()
        .map(|(index, module)| (module.id.as_str(), index))
        .collect();
    let &start = ids.get(module_id).ok_or("未找到该模块，请重新分析项目。")?;
    let excluded: HashSet<&str> = excluded.iter().map(String::as_str).collect();
    if excluded.contains(module_id) {
        return Err("该模块已被移除，请选择仍在塔中的模块。".into());
    }
    let mut visited = vec![false; modules.len()];
    visited[start] = true;
    let mut frontier = vec![start];
    let mut waves = vec![vec![module_id.to_string()]];
    loop {
        let mut next = Vec::new();
        for &current in &frontier {
            for importer in &modules[current].imported_by {
                if excluded.contains(importer.as_str()) {
                    continue;
                }
                let &index = ids
                    .get(importer.as_str())
                    .ok_or("依赖图不一致，请重新分析项目。")?;
                if !visited[index] {
                    visited[index] = true;
                    next.push(index);
                }
            }
        }
        if next.is_empty() {
            break;
        }
        next.sort_by(|&a, &b| modules[a].id.cmp(&modules[b].id));
        waves.push(
            next.iter()
                .map(|&index| modules[index].id.clone())
                .collect(),
        );
        frontier = next;
    }
    let total_affected = waves.iter().skip(1).map(Vec::len).sum::<usize>();
    let direct_affected = waves.get(1).map(Vec::len).unwrap_or(0);
    Ok(ImpactResult {
        removed_module_id: module_id.into(),
        total_affected,
        affected_ratio: ratio(total_affected, modules.len()),
        direct_affected,
        transitive_affected: total_affected - direct_affected,
        max_cascade_depth: waves.len() - 1,
        waves,
    })
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}
