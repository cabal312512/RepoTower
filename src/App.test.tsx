import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App';
import { useTowerStore } from './state/store';
import { usePreferences } from './state/preferences';
import type { ImpactResult, RepositoryAnalysis } from './types/analysis';

vi.mock('./graph/GraphScene', () => ({ default: () => <div data-testid="scene" /> }));
const analysis: RepositoryAnalysis = {
  repositoryName: 'example',
  rootDisplayName: 'example',
  totalModules: 1,
  totalEdges: 0,
  unresolvedCount: 0,
  externalCount: 0,
  cycleCount: 0,
  stabilityScore: 100,
  stabilityBreakdown: { averageBlastRatio: 0, maxBlastRatio: 0, cycleRatio: 0, concentration: 0 },
  edges: [],
  criticalModules: ['a.ts'],
  warnings: [],
  modules: [
    {
      id: 'a.ts',
      relativePath: 'a.ts',
      fileName: 'a.ts',
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
    },
  ],
};
const result: ImpactResult = {
  removedModuleId: 'a.ts',
  totalAffected: 0,
  affectedRatio: 0,
  directAffected: 0,
  transitiveAffected: 0,
  maxCascadeDepth: 0,
  waves: [['a.ts']],
};
let deliver: (value: ImpactResult) => void;
beforeEach(() => {
  vi.stubGlobal('matchMedia', () => ({
    matches: false,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
  }));
  usePreferences.setState({ theme: 'dark', language: 'zh' });
  useTowerStore.getState().setAnalysis(analysis);
  useTowerStore.getState().select('a.ts');
  const pending = new Promise<ImpactResult>((resolve) => {
    deliver = resolve;
  });
  window.repoTower = {
    openRepository: vi.fn(),
    analyzePath: vi.fn(),
    loadDemo: vi.fn(),
    impact: vi.fn().mockResolvedValueOnce(result).mockReturnValueOnce(pending),
    onProgress: () => () => {},
    onOpenPath: () => () => {},
    getDroppedPath: () => '',
    minimizeWindow: vi.fn(),
    toggleMaximizeWindow: vi.fn(),
    closeWindow: vi.fn(),
    getWindowState: vi.fn(),
    onWindowState: () => () => {},
  };
});
afterEach(() => {
  cleanup();
  delete window.repoTower;
  vi.unstubAllGlobals();
  localStorage.clear();
  useTowerStore.getState().close();
});
async function pendingPull() {
  render(<App />);
  await waitFor(() => expect(screen.getByTestId('pull-button')).toBeEnabled());
  fireEvent.click(screen.getByTestId('pull-button'));
}
describe('pending impact requests', () => {
  function twoCuts() {
    const modules = ['a.ts', 'b.ts', 'shared.ts'].map((id) => ({
      ...analysis.modules[0],
      id,
      relativePath: id,
      fileName: id,
    }));
    useTowerStore.getState().setAnalysis({ ...analysis, modules, totalModules: 3 });
    useTowerStore
      .getState()
      .beginPull({ ...result, totalAffected: 1, waves: [['a.ts'], ['shared.ts']] });
    useTowerStore.getState().finishPull();
    useTowerStore.getState().beginPull({ ...result, removedModuleId: 'b.ts', waves: [['b.ts']] });
    useTowerStore.getState().finishPull();
    useTowerStore.getState().select('a.ts');
  }
  it('recomputes remaining cuts through Rust when restoring an earlier file; shared impact survives', async () => {
    twoCuts();
    vi.mocked(window.repoTower!.impact)
      .mockReset()
      .mockImplementation(async (id) => ({
        ...result,
        removedModuleId: id,
        totalAffected: 1,
        waves: [[id], ['shared.ts']],
      }));
    render(<App />);
    fireEvent.click(screen.getByTestId('restore-file'));
    await waitFor(() => expect(useTowerStore.getState().disconnectedIds).toEqual(['b.ts']));
    expect(window.repoTower!.impact).toHaveBeenCalledWith('b.ts', []);
    expect(useTowerStore.getState().removedIds).toEqual(['b.ts', 'shared.ts']);
    fireEvent.click(screen.getByTestId('undo-toolbar'));
    expect(useTowerStore.getState().disconnectedIds).toEqual(['a.ts', 'b.ts']);
    expect(screen.getByTestId('restore-file')).toBeEnabled();
  });
  it('discards a delayed restore calculation after closing the repository', async () => {
    twoCuts();
    vi.mocked(window.repoTower!.impact)
      .mockReset()
      .mockReturnValue(
        new Promise((resolve) => {
          deliver = resolve;
        }),
      );
    render(<App />);
    fireEvent.click(screen.getByTestId('restore-file'));
    fireEvent.click(screen.getByTestId('home'));
    await act(async () => deliver({ ...result, removedModuleId: 'b.ts', waves: [['b.ts']] }));
    expect(useTowerStore.getState().analysis).toBeNull();
    expect(useTowerStore.getState().history).toHaveLength(0);
  });
  it('can restore an earlier disconnected file while another cut is still playing', async () => {
    twoCuts();
    useTowerStore.getState().undo();
    useTowerStore.getState().beginPull({ ...result, removedModuleId: 'b.ts', waves: [['b.ts']] });
    useTowerStore.getState().select('a.ts');
    vi.mocked(window.repoTower!.impact)
      .mockReset()
      .mockImplementation(async (id) => ({
        ...result,
        removedModuleId: id,
        waves: [[id], ['shared.ts']],
      }));
    render(<App />);
    fireEvent.click(screen.getByTestId('restore-file'));
    await waitFor(() => expect(useTowerStore.getState().disconnectedIds).toEqual(['b.ts']));
    expect(useTowerStore.getState().removedIds).toEqual(['b.ts', 'shared.ts']);
    expect(useTowerStore.getState().impact).toBeNull();
  });
  it('exposes unresolved imports without counting them as confirmed dependencies', async () => {
    useTowerStore.getState().setAnalysis({
      ...analysis,
      unresolvedCount: 1,
      modules: [
        {
          ...analysis.modules[0],
          unresolvedImports: [
            { specifier: './missing-file', kind: 'static', reason: '路径位于所选项目之外。' },
          ],
        },
      ],
    });
    useTowerStore.getState().select('a.ts');
    render(<App />);
    await waitFor(() => expect(screen.getByTestId('pull-button')).toBeEnabled());
    fireEvent.click(screen.getByTestId('unresolved-toggle'));
    expect(screen.getByText('./missing-file')).toBeVisible();
    expect(screen.getByText('路径位于所选项目之外。')).toBeVisible();
    expect(screen.getByTestId('potential-count')).toHaveTextContent('0');
  });
  it('discards a late result after closing the repository', async () => {
    await pendingPull();
    fireEvent.click(screen.getByTestId('home'));
    await act(async () => {
      deliver(result);
    });
    expect(useTowerStore.getState().analysis).toBeNull();
    expect(useTowerStore.getState().impact).toBeNull();
    expect(useTowerStore.getState().history).toHaveLength(0);
  });
  it('keeps a pending pull valid when only theme and language change', async () => {
    await pendingPull();
    fireEvent.click(screen.getByTestId('settings-toggle'));
    fireEvent.click(screen.getByTestId('theme-light'));
    fireEvent.change(screen.getByTestId('language-select'), { target: { value: 'en' } });
    await act(async () => {
      deliver(result);
    });
    expect(usePreferences.getState().theme).toBe('light');
    expect(usePreferences.getState().language).toBe('en');
    expect(useTowerStore.getState().impact?.removedModuleId).toBe('a.ts');
  });
});
