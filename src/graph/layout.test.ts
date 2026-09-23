// @vitest-environment node
import { describe, expect, it } from 'vitest';
import type { ImpactResult, ModuleAnalysis, RepositoryAnalysis } from '../types/analysis';
import {
  buildGraphLayout,
  causalParents,
  edgeRoute,
  MAX_VISIBLE_NODES,
  NODE_HEIGHT,
  NODE_WIDTH,
  nodeState,
} from './layout';

function module(id: string, depth = 0): ModuleAnalysis {
  return {
    id,
    relativePath: id,
    fileName: id,
    extension: '.ts',
    directory: '',
    linesOfCode: 10,
    imports: [],
    importedBy: [],
    fanIn: 0,
    fanOut: 0,
    dependencyDepth: depth,
    blastRadius: 0,
    blastRatio: 0,
    cycleId: null,
    isEntryLike: false,
    unresolvedImports: [],
  };
}
function analysis(ids: string[], imports: [string, string][]): RepositoryAnalysis {
  return {
    repositoryName: 'test',
    rootDisplayName: 'test',
    totalModules: ids.length,
    totalEdges: imports.length,
    unresolvedCount: 0,
    externalCount: 0,
    cycleCount: 0,
    stabilityScore: 100,
    stabilityBreakdown: { averageBlastRatio: 0, maxBlastRatio: 0, cycleRatio: 0, concentration: 0 },
    modules: ids.map((id, index) => module(id, index)),
    edges: imports.map(([source, target]) => ({ source, target, kind: 'static' })),
    criticalModules: [],
    warnings: [],
  };
}
function impact(waves: string[][]): ImpactResult {
  return {
    removedModuleId: waves[0][0],
    totalAffected: waves.flat().length - 1,
    affectedRatio: 0.5,
    directAffected: waves[1]?.length ?? 0,
    transitiveAffected: waves.slice(2).flat().length,
    maxCascadeDepth: waves.length - 1,
    waves,
  };
}

describe('dependency circuit layout', () => {
  it('reverses imports into real dependency-to-consumer signal edges', () => {
    const source = analysis(
      ['core', 'service', 'app', 'theme'],
      [
        ['service', 'core'],
        ['app', 'service'],
        ['app', 'theme'],
      ],
    );
    const layout = buildGraphLayout(source, { impact: impact([['core'], ['service'], ['app']]) });
    expect(layout.edges.map(({ from, to, causal }) => ({ from, to, causal }))).toEqual([
      { from: 'core', to: 'service', causal: true },
      { from: 'service', to: 'app', causal: true },
    ]);
    expect(layout.byId.has('theme')).toBe(false);
    expect(layout.untouchedCount).toBe(1);
    expect(layout.byId.get('core')!.x).toBeLessThan(layout.byId.get('service')!.x);
    expect(layout.byId.get('service')!.x).toBeLessThan(layout.byId.get('app')!.x);
  });

  it('shows both actual causes in a diamond and never invents same-wave connections', () => {
    const layout = buildGraphLayout(
      analysis(
        ['root', 'a', 'b', 'view'],
        [
          ['a', 'root'],
          ['b', 'root'],
          ['view', 'a'],
          ['view', 'b'],
        ],
      ),
      {
        impact: impact([['root'], ['a', 'b'], ['view']]),
      },
    );
    expect(causalParents(layout, 'view').map((node) => node.id)).toEqual(['a', 'b']);
    expect(layout.edges).toHaveLength(4);
    expect(layout.edges.every((edge) => edge.causal)).toBe(true);
  });

  it('keeps cycle back-links and longer alternative paths contextual, using Rust shortest waves', () => {
    const layout = buildGraphLayout(
      analysis(
        ['root', 'a', 'b'],
        [
          ['a', 'root'],
          ['b', 'root'],
          ['b', 'a'],
          ['root', 'b'],
        ],
      ),
      {
        impact: impact([['root'], ['a', 'b']]),
      },
    );
    expect(
      layout.edges.filter((edge) => edge.causal).map((edge) => `${edge.from}>${edge.to}`),
    ).toEqual(['root>a', 'root>b']);
    expect(
      layout.edges.filter((edge) => !edge.causal).map((edge) => `${edge.from}>${edge.to}`),
    ).toEqual(['a>b', 'b>root']);
    expect(causalParents(layout, 'root')).toEqual([]);
    expect(causalParents(layout, 'b').map((node) => node.id)).toEqual(['root']);
  });

  it('changes a file state only when its shortest-path pulse arrives, including manual rewind', () => {
    const layout = buildGraphLayout(
      analysis(
        ['root', 'a', 'b'],
        [
          ['a', 'root'],
          ['b', 'a'],
        ],
      ),
      { impact: impact([['root'], ['a'], ['b']]) },
    );
    expect(nodeState(layout.byId.get('root')!, 0, new Set())).toBe('removed');
    expect(nodeState(layout.byId.get('a')!, 0.999, new Set())).toBe('pending');
    expect(nodeState(layout.byId.get('a')!, 1, new Set())).toBe('affected');
    expect(nodeState(layout.byId.get('b')!, 1, new Set())).toBe('pending');
    expect(nodeState(layout.byId.get('b')!, 2, new Set())).toBe('affected');
    expect(nodeState(layout.byId.get('b')!, 1, new Set(['root', 'a', 'b']))).toBe('pending');
  });

  it('distinguishes earlier removed files from files affected by them in the overview', () => {
    const layout = buildGraphLayout(analysis(['root', 'a'], [['a', 'root']]));
    expect(nodeState(layout.byId.get('root')!, 0, new Set(['root', 'a']), new Set(['root']))).toBe(
      'removed',
    );
    expect(nodeState(layout.byId.get('a')!, 0, new Set(['root', 'a']), new Set(['root']))).toBe(
      'affected',
    );
  });

  it('does not count previously unavailable files as newly untouched', () => {
    const layout = buildGraphLayout(analysis(['root', 'a', 'old', 'safe'], [['a', 'root']]), {
      impact: impact([['root'], ['a']]),
      removedIds: ['old', 'root', 'a'],
    });
    expect(layout.untouchedCount).toBe(1);
    expect(layout.previousRemovedCount).toBe(1);
  });

  it('is deterministic, separates foundations from consumers, and places no nodes on top of one another', () => {
    const source = analysis(
      Array.from({ length: 80 }, (_, i) => `file-${i}`),
      [],
    );
    source.modules.forEach((item, index) => {
      item.dependencyDepth = index % 6;
    });
    const layout = buildGraphLayout(source);
    const reversed = buildGraphLayout({
      ...source,
      modules: [...source.modules].reverse(),
      edges: [...source.edges].reverse(),
    });
    expect(layout.nodes).toEqual(reversed.nodes);
    for (const a of layout.nodes)
      for (const b of layout.nodes) {
        if (a.id === b.id) continue;
        expect(Math.abs(a.x - b.x) >= NODE_WIDTH || Math.abs(a.y - b.y) >= NODE_HEIGHT).toBe(true);
      }
  });

  it('limits very large views explicitly and preserves earliest waves and the removed origin', () => {
    const ids = Array.from(
      { length: MAX_VISIBLE_NODES + 70 },
      (_, i) => `n${String(i).padStart(3, '0')}`,
    );
    const source = analysis(
      ids,
      ids.slice(1).map((id) => [id, ids[0]]),
    );
    const layout = buildGraphLayout(source, { impact: impact([[ids[0]], ids.slice(1)]) });
    expect(layout.nodes).toHaveLength(MAX_VISIBLE_NODES);
    expect(layout.hiddenCount).toBe(70);
    expect(layout.untouchedCount).toBe(0);
    expect(layout.byId.has(ids[0])).toBe(true);
    expect(layout.edges).toHaveLength(MAX_VISIBLE_NODES - 1);
  });

  it('focuses on actual direct neighbors and exposes how many files are outside the view', () => {
    const source = analysis(
      ['core', 'a', 'app', 'other'],
      [
        ['a', 'core'],
        ['app', 'a'],
      ],
    );
    const layout = buildGraphLayout(source, { selectedId: 'a', neighborhood: true });
    expect(layout.kind).toBe('neighbors');
    expect(layout.nodes.map((node) => node.id)).toEqual(['core', 'a', 'app']);
    expect(layout.untouchedCount).toBe(1);
  });

  it('keeps every overview position when showing impact in the whole repository', () => {
    const source = analysis(
      ['core', 'service', 'app', 'theme'],
      [
        ['service', 'core'],
        ['app', 'service'],
        ['app', 'theme'],
      ],
    );
    const overview = buildGraphLayout(source);
    const result = impact([['core'], ['service'], ['app']]);
    const full = buildGraphLayout(source, { impact: result, impactOverview: true });
    expect(full.kind).toBe('impact-overview');
    expect(full.nodes.map(({ id, x, y }) => ({ id, x, y }))).toEqual(
      overview.nodes.map(({ id, x, y }) => ({ id, x, y })),
    );
    expect(nodeState(full.byId.get('theme')!, 2, new Set(['core', 'service', 'app']))).toBe(
      'standing',
    );
    expect(nodeState(full.byId.get('service')!, 1, new Set())).toBe('affected');
    expect(nodeState(full.byId.get('app')!, 1, new Set())).toBe('pending');
    expect(full.edges.find((edge) => edge.from === 'theme' && edge.to === 'app')?.causal).toBe(
      false,
    );
    expect(full.edges.filter((edge) => edge.causal)).toHaveLength(2);
    expect(full.untouchedCount).toBe(1);
  });

  it('routes signal particles from the visible source port to the visible destination arrow', () => {
    const layout = buildGraphLayout(analysis(['core', 'app'], [['app', 'core']]));
    const from = layout.byId.get('core')!;
    const to = layout.byId.get('app')!;
    const route = edgeRoute(from, to);
    expect(route.point(0)).toEqual({ x: from.x + NODE_WIDTH, y: from.y + NODE_HEIGHT / 2 });
    expect(route.point(1)).toEqual({ x: to.x - 5, y: to.y + NODE_HEIGHT / 2 });
  });
});
