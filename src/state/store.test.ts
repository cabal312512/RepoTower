// @vitest-environment node
import { beforeEach, describe, expect, it } from 'vitest';
import { useTowerStore } from './store';
import type { ImpactResult, ModuleAnalysis, RepositoryAnalysis } from '../types/analysis';
const module = (id: string): ModuleAnalysis => ({
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
  dependencyDepth: 0,
  blastRadius: 0,
  blastRatio: 0,
  cycleId: null,
  isEntryLike: true,
  unresolvedImports: [],
});
export const analysis: RepositoryAnalysis = {
  repositoryName: 'test',
  rootDisplayName: 'test',
  totalModules: 8,
  totalEdges: 2,
  unresolvedCount: 0,
  externalCount: 0,
  cycleCount: 0,
  stabilityScore: 90,
  stabilityBreakdown: { averageBlastRatio: 0, maxBlastRatio: 0, cycleRatio: 0, concentration: 0 },
  modules: ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'].map(module),
  edges: [],
  criticalModules: ['a'],
  warnings: [],
};
const impact = (id: string, affected: string[] = []): ImpactResult => ({
  removedModuleId: id,
  totalAffected: affected.length,
  affectedRatio: affected.length / 8,
  directAffected: affected.length,
  transitiveAffected: 0,
  maxCascadeDepth: affected.length ? 1 : 0,
  waves: affected.length ? [[id], affected] : [[id]],
});
const state = () => useTowerStore.getState();
beforeEach(() => state().setAnalysis(analysis));
describe('selection and deterministic pull state', () => {
  it('retains history when another standing file is selected', () => {
    state().beginPull(impact('a', ['b']));
    state().finishPull();
    state().select('c');
    expect(state().removedIds).toEqual(['a', 'b']);
    expect(state().history).toHaveLength(1);
  });
  it('selects existing files again even after disconnection', () => {
    state().select('a');
    expect(state().selectedId).toBe('a');
    state().select('missing');
    expect(state().selectedId).toBe('a');
    state().beginPull(impact('a'));
    state().finishPull();
    state().select('a');
    expect(state().selectedId).toBe('a');
  });
  it('commits removal only after animation and counts target separately', () => {
    state().beginPull(impact('a', ['b', 'c']));
    expect(state().removedIds).toEqual([]);
    expect(state().impact?.waves).toEqual([['a'], ['b', 'c']]);
    state().finishPull();
    expect(state().removedIds).toEqual(['a', 'b', 'c']);
    expect(state().lastImpact?.totalAffected).toBe(2);
  });
  it('rejects concurrent pulls and duplicate animation completion', () => {
    state().beginPull(impact('a'));
    state().beginPull(impact('b'));
    state().finishPull();
    state().finishPull();
    expect(state().history).toHaveLength(1);
    expect(state().removedIds).toEqual(['a']);
  });
  it('restores the exact pre-pull state over multiple undos', () => {
    state().select('a');
    state().beginPull(impact('a', ['b']));
    state().finishPull();
    state().select('c');
    state().beginPull(impact('c', ['d']));
    state().finishPull();
    state().undo();
    expect(state().removedIds).toEqual(['a', 'b']);
    expect(state().selectedId).toBe('c');
    expect(state().history).toHaveLength(1);
    expect(state().lastImpact?.removedModuleId).toBe('a');
    state().undo();
    expect(state().removedIds).toEqual([]);
    expect(state().selectedId).toBe('a');
    expect(state().history).toHaveLength(0);
  });
  it('reset restores all files while retaining the repository', () => {
    state().beginPull(impact('a', ['b']));
    state().finishPull();
    state().reset();
    expect(state().analysis).toBe(analysis);
    expect(state().removedIds).toEqual([]);
    expect(state().history).toEqual([]);
  });
  it('allows inspection while propagating without changing the cut; undo waits for completion', () => {
    state().select('a');
    state().beginPull(impact('a'));
    state().select('b');
    state().undo();
    expect(state().selectedId).toBe('b');
    expect(state().impact?.removedModuleId).toBe('a');
    state().finishPull();
    state().undo();
    expect(state().removedIds).toEqual([]);
  });
  it('allows consecutive simulations with exact undo, without a game limit', () => {
    for (const id of ['a', 'b', 'c', 'd', 'e', 'f']) {
      state().beginPull(impact(id));
      state().finishPull();
    }
    state().beginPull(impact('a'));
    expect(state().impact).toBeNull();
    expect(state().history).toHaveLength(6);
    state().undo();
    expect(state().history).toHaveLength(5);
    state().beginPull(impact('f'));
    expect(state().impact?.removedModuleId).toBe('f');
  });
  it('tracks disconnected origins independently from affected files', () => {
    state().beginPull(impact('a', ['b']));
    state().finishPull();
    state().beginPull(impact('c', ['d']));
    state().finishPull();
    expect(state().disconnectedIds).toEqual(['a', 'c']);
  });
  it('restores an earlier cut while retaining remaining causes and supports exact undo', () => {
    state().beginPull(impact('a', ['b', 'd']));
    state().finishPull();
    state().beginPull(impact('c'));
    state().finishPull();
    state().select('a');
    state().restore('a', [impact('c', ['d'])]);
    expect(state().removedIds).toEqual(['c', 'd']);
    expect(state().disconnectedIds).toEqual(['c']);
    expect(state().selectedId).toBe('a');
    expect(state().reports).toHaveLength(1);
    state().undo();
    expect(state().removedIds).toEqual(['a', 'b', 'd', 'c']);
    expect(state().disconnectedIds).toEqual(['a', 'c']);
    expect(state().reports).toHaveLength(2);
  });
  it('retains reports when selecting other files and can review earlier cuts', () => {
    state().beginPull(impact('a', ['b']));
    state().finishPull();
    state().beginPull(impact('c', ['d']));
    state().finishPull();
    state().select('e');
    state().review('a');
    expect(state().lastImpact?.removedModuleId).toBe('a');
    expect(state().selectedId).toBe('e');
    const history = state().history;
    state().restore('a', []);
    expect(state().history).toBe(history);
    state().restore('c', [impact('a', ['b'])]);
    state().restore('a', []);
    expect(state().removedIds).toEqual([]);
    expect(state().lastImpact).toBeNull();
  });
});
