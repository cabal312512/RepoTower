// Source lineage: rt-origin-77f1f64c-6a08-44e8-87a1-19728239c117
import type { ImpactResult, ModuleAnalysis, RepositoryAnalysis } from '../types/analysis';

export const NODE_WIDTH = 158;
export const NODE_HEIGHT = 44;
export const COLUMN_PITCH = 210;
export const ROW_PITCH = 66;
export const MAX_VISIBLE_NODES = 250;
export const WAVE_DURATION = 650;

export interface GraphNode {
  id: string;
  module: ModuleAnalysis;
  x: number;
  y: number;
  column: number;
  wave: number | null;
}
export interface GraphEdge {
  id: string;
  /** Signal direction: the dependency first, then the file that imports it. */
  from: string;
  to: string;
  causal: boolean;
}
export interface GraphLayout {
  nodes: GraphNode[];
  edges: GraphEdge[];
  byId: Map<string, GraphNode>;
  columns: { index: number; x: number; count: number }[];
  width: number;
  height: number;
  hiddenCount: number;
  untouchedCount: number;
  previousRemovedCount: number;
  kind: 'overview' | 'neighbors' | 'impact' | 'impact-overview';
}

const compareIds = (a: string, b: string) => (a < b ? -1 : a > b ? 1 : 0);

// 代码全是AI生成的 连提示词也是ai写的
/** Every edge is derived from the analyzer. Layout never invents dependencies. */
function reverseEdges(
  analysis: RepositoryAnalysis,
  visible: Set<string>,
  waves?: Map<string, number>,
): GraphEdge[] {
  const seen = new Set<string>();
  return analysis.edges
    .flatMap((edge) => {
      if (!visible.has(edge.source) || !visible.has(edge.target)) return [];
      const id = `${edge.target}\u0000${edge.source}`;
      if (seen.has(id)) return [];
      seen.add(id);
      const fromWave = waves?.get(edge.target);
      const toWave = waves?.get(edge.source);
      return [
        {
          id,
          from: edge.target,
          to: edge.source,
          causal: fromWave !== undefined && toWave !== undefined && toWave === fromWave + 1,
        },
      ];
    })
    .sort((a, b) => compareIds(a.id, b.id));
}

function positionNodes(
  modules: ModuleAnalysis[],
  ranks: Map<string, number>,
  edges: GraphEdge[],
  waves?: Map<string, number>,
): Pick<GraphLayout, 'nodes' | 'byId' | 'columns' | 'width' | 'height'> {
  const layers = new Map<number, ModuleAnalysis[]>();
  for (const module of [...modules].sort((a, b) => compareIds(a.id, b.id))) {
    const rank = ranks.get(module.id) ?? 0;
    const layer = layers.get(rank) ?? [];
    layer.push(module);
    layers.set(rank, layer);
  }
  const indices = [...layers.keys()].sort((a, b) => a - b);
  const incoming = new Map<string, string[]>();
  const outgoing = new Map<string, string[]>();
  for (const edge of edges) {
    incoming.set(edge.to, [...(incoming.get(edge.to) ?? []), edge.from]);
    outgoing.set(edge.from, [...(outgoing.get(edge.from) ?? []), edge.to]);
  }
  // A few deterministic barycenter sweeps keep related branches close without physics.
  for (let pass = 0; pass < 4; pass += 1) {
    const positions = new Map<string, number>();
    for (const layer of layers.values())
      layer.forEach((module, row) => positions.set(module.id, row - (layer.length - 1) / 2));
    const order = pass % 2 === 0 ? indices : [...indices].reverse();
    const neighbors = pass % 2 === 0 ? incoming : outgoing;
    for (const index of order) {
      const layer = layers.get(index)!;
      const score = (id: string) => {
        const adjacent = (neighbors.get(id) ?? []).filter((other) => ranks.get(other) !== index);
        return adjacent.length
          ? adjacent.reduce((sum, other) => sum + (positions.get(other) ?? 0), 0) / adjacent.length
          : (positions.get(id) ?? 0);
      };
      layer.sort((a, b) => score(a.id) - score(b.id) || compareIds(a.id, b.id));
      layer.forEach((module, row) => positions.set(module.id, row - (layer.length - 1) / 2));
    }
  }
  const maxRows = Math.max(1, ...[...layers.values()].map((layer) => layer.length));
  const height = Math.max(160, maxRows * ROW_PITCH + 72);
  const nodes: GraphNode[] = [];
  const columns = indices.map((index, position) => {
    const layer = layers.get(index)!;
    const x = 44 + position * COLUMN_PITCH;
    layer.forEach((module, row) =>
      nodes.push({
        id: module.id,
        module,
        x,
        y: height / 2 + (row - (layer.length - 1) / 2) * ROW_PITCH - NODE_HEIGHT / 2 + 16,
        column: index,
        wave: waves?.get(module.id) ?? null,
      }),
    );
    return { index, x, count: layer.length };
  });
  return {
    nodes,
    byId: new Map(nodes.map((node) => [node.id, node])),
    columns,
    width: Math.max(250, 88 + Math.max(0, indices.length - 1) * COLUMN_PITCH + NODE_WIDTH),
    height,
  };
}

export function buildGraphLayout(
  analysis: RepositoryAnalysis,
  options: {
    impact?: ImpactResult | null;
    selectedId?: string | null;
    inspectId?: string | null;
    neighborhood?: boolean;
    removedIds?: string[];
    impactOverview?: boolean;
  } = {},
): GraphLayout {
  const {
    impact,
    selectedId,
    inspectId,
    neighborhood = false,
    removedIds = [],
    impactOverview = false,
  } = options;
  const byId = new Map(analysis.modules.map((module) => [module.id, module]));
  const removed = new Set(removedIds);
  const waves = impact
    ? new Map(impact.waves.flatMap((ids, wave) => ids.map((id) => [id, wave] as const)))
    : undefined;
  const ranks = new Map<string, number>();
  let candidates: string[];
  let kind: GraphLayout['kind'] = 'overview';
  if (impact && waves && !impactOverview && !neighborhood) {
    kind = 'impact';
    candidates = [...waves.keys()].sort(
      (a, b) => waves.get(a)! - waves.get(b)! || compareIds(a, b),
    );
    for (const [id, wave] of waves) ranks.set(id, wave);
  } else if (selectedId && byId.has(selectedId) && neighborhood) {
    kind = 'neighbors';
    ranks.set(selectedId, 1);
    for (const edge of analysis.edges) {
      if (edge.source === selectedId && edge.target !== selectedId) ranks.set(edge.target, 0);
    }
    for (const edge of analysis.edges) {
      if (edge.target === selectedId && edge.source !== selectedId) ranks.set(edge.source, 2);
    }
    candidates = [...ranks.keys()].sort(
      (a, b) => Number(b === selectedId) - Number(a === selectedId) || compareIds(a, b),
    );
  } else {
    if (impact) kind = 'impact-overview';
    candidates = [...byId.keys()].sort((a, b) => compareIds(a, b));
    for (const module of analysis.modules) ranks.set(module.id, module.dependencyDepth);
    // Preserve the current selection even when a large map is summarized.
    const anchors = [
      ...new Set(
        [impact?.removedModuleId, inspectId, selectedId].filter((id): id is string =>
          Boolean(id && byId.has(id)),
        ),
      ),
    ];
    candidates = [...anchors, ...candidates.filter((id) => !anchors.includes(id))];
  }
  candidates = candidates.filter((id) => byId.has(id));
  const visibleIds = candidates.slice(0, MAX_VISIBLE_NODES);
  const visible = new Set(visibleIds);
  const edges = reverseEdges(analysis, visible, waves);
  const modules = visibleIds.map((id) => byId.get(id)!);
  return {
    ...positionNodes(modules, ranks, edges, waves),
    edges,
    kind,
    hiddenCount: candidates.length - visibleIds.length,
    untouchedCount: impact
      ? analysis.modules.filter((module) => !waves!.has(module.id) && !removed.has(module.id))
          .length
      : analysis.modules.length - candidates.length,
    previousRemovedCount: impact
      ? [...removed].filter((id) => byId.has(id) && !waves!.has(id)).length
      : 0,
  };
}

export function causalParents(layout: GraphLayout, id: string): GraphNode[] {
  return layout.edges
    .filter((edge) => edge.to === id && edge.causal)
    .flatMap((edge) => {
      const node = layout.byId.get(edge.from);
      return node ? [node] : [];
    });
}

export type GraphNodeState = 'standing' | 'removed' | 'affected' | 'pending';
export function nodeState(
  node: GraphNode,
  playhead: number,
  removed: Set<string>,
  disconnected?: Set<string>,
): GraphNodeState {
  if (disconnected?.has(node.id)) return 'removed';
  if (node.wave !== null) {
    if (node.wave === 0) return 'removed';
    return playhead >= node.wave ? 'affected' : 'pending';
  }
  return removed.has(node.id)
    ? disconnected && !disconnected.has(node.id)
      ? 'affected'
      : 'removed'
    : 'standing';
}

export interface CubicRoute {
  path: string;
  point: (t: number) => { x: number; y: number };
}
export function edgeRoute(from: GraphNode, to: GraphNode): CubicRoute {
  const forward = to.x > from.x;
  const sameColumn = to.x === from.x;
  const start = { x: from.x + NODE_WIDTH, y: from.y + NODE_HEIGHT / 2 };
  const end = { x: forward ? to.x - 5 : to.x + NODE_WIDTH + 5, y: to.y + NODE_HEIGHT / 2 };
  const reach = forward
    ? Math.max(36, (end.x - start.x) * 0.48)
    : sameColumn
      ? 38 + Math.abs(to.y - from.y) * 0.1
      : 72;
  const c1 = { x: start.x + reach, y: start.y };
  const c2 = { x: forward ? end.x - reach : end.x + reach, y: end.y };
  return {
    path: `M ${start.x} ${start.y} C ${c1.x} ${c1.y}, ${c2.x} ${c2.y}, ${end.x} ${end.y}`,
    point: (t) => {
      const u = 1 - t;
      return {
        x: u ** 3 * start.x + 3 * u ** 2 * t * c1.x + 3 * u * t ** 2 * c2.x + t ** 3 * end.x,
        y: u ** 3 * start.y + 3 * u ** 2 * t * c1.y + 3 * u * t ** 2 * c2.y + t ** 3 * end.y,
      };
    },
  };
}
