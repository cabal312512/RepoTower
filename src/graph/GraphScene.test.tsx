import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { ImpactResult, ModuleAnalysis, RepositoryAnalysis } from '../types/analysis';
import GraphScene, { type GraphSceneProps } from './GraphScene';

const modules: ModuleAnalysis[] = ['core', 'service', 'view', 'theme'].map(
  (id, dependencyDepth) => ({
    id,
    fileName: `${id}.ts`,
    relativePath: `src/${id}.ts`,
    extension: '.ts',
    directory: 'src',
    linesOfCode: 20,
    imports: [],
    importedBy: [],
    fanIn: 0,
    fanOut: 0,
    dependencyDepth,
    blastRadius: 0,
    blastRatio: 0,
    cycleId: null,
    isEntryLike: false,
    unresolvedImports: [],
  }),
);
const analysis: RepositoryAnalysis = {
  repositoryName: 'example',
  rootDisplayName: 'example',
  totalModules: 4,
  totalEdges: 3,
  unresolvedCount: 0,
  externalCount: 0,
  cycleCount: 0,
  stabilityScore: 90,
  stabilityBreakdown: { averageBlastRatio: 0, maxBlastRatio: 0, cycleRatio: 0, concentration: 0 },
  modules,
  edges: [
    { source: 'service', target: 'core', kind: 'static' },
    { source: 'view', target: 'service', kind: 'static' },
    { source: 'view', target: 'theme', kind: 'static' },
  ],
  criticalModules: ['core'],
  warnings: [],
};
const impact: ImpactResult = {
  removedModuleId: 'core',
  totalAffected: 2,
  affectedRatio: 0.5,
  directAffected: 1,
  transitiveAffected: 1,
  maxCascadeDepth: 2,
  waves: [['core'], ['service'], ['view']],
};
function props(overrides: Partial<GraphSceneProps> = {}): GraphSceneProps {
  return {
    analysis,
    selectedId: 'core',
    removedIds: [],
    impact,
    report: null,
    animationStartedAt: 42,
    onSelect: vi.fn(),
    onAnimationComplete: vi.fn(),
    resetToken: 0,
    theme: 'dark',
    language: 'zh',
    reducedMotion: false,
    ...overrides,
  };
}
let now = 0;
let sequence = 0;
let frames: Map<number, FrameRequestCallback>;
beforeEach(() => {
  now = 0;
  frames = new Map();
  vi.spyOn(performance, 'now').mockImplementation(() => now);
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
    frames.set(++sequence, callback);
    return sequence;
  });
  vi.stubGlobal('cancelAnimationFrame', (id: number) => frames.delete(id));
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});
function advance(milliseconds: number) {
  act(() => {
    const end = now + milliseconds;
    while (now < end) {
      now += Math.min(16, end - now);
      const pending = [...frames.values()];
      frames.clear();
      pending.forEach((callback) => callback(now));
    }
  });
}
function state(id: string) {
  return document
    .querySelector(`[data-testid="graph-node"][data-id="${id}"]`)
    ?.getAttribute('data-state');
}

describe('impact circuit playback', () => {
  it('keeps the user camera on selection and remembers it when returning from a double-click neighborhood', () => {
    const initial = props({ impact: null });
    const { rerender } = render(<GraphScene {...initial} />);
    const camera = () =>
      document.querySelector('.graph-board > g[transform]')?.getAttribute('transform');
    fireEvent.click(screen.getByRole('button', { name: '放大' }));
    const zoomed = camera();
    fireEvent.click(screen.getByRole('button', { name: 'src/service.ts' }));
    rerender(<GraphScene {...initial} selectedId="service" />);
    expect(camera()).toBe(zoomed);
    expect(screen.getByTestId('graph-scene')).toHaveAttribute('data-view', 'overview');
    fireEvent.doubleClick(screen.getByRole('button', { name: 'src/service.ts' }));
    expect(screen.getByTestId('graph-scene')).toHaveAttribute('data-view', 'neighbors');
    const positions = screen
      .getAllByTestId('graph-node')
      .map((node) => node.getAttribute('transform'));
    rerender(<GraphScene {...initial} selectedId="core" />);
    expect(
      screen.getAllByTestId('graph-node').map((node) => node.getAttribute('transform')),
    ).toEqual(positions);
    fireEvent.click(screen.getByTestId('graph-overview'));
    expect(camera()).toBe(zoomed);
  });

  it('delays hover previews by 350 ms and cancels a preview when the pointer leaves', () => {
    vi.useFakeTimers();
    render(<GraphScene {...props({ impact: null })} />);
    const theme = screen.getByRole('button', { name: 'src/theme.ts' });
    const core = screen.getByRole('button', { name: 'src/core.ts' });
    fireEvent.pointerEnter(theme);
    act(() => vi.advanceTimersByTime(349));
    expect(core).toHaveAttribute('aria-pressed', 'true');
    fireEvent.pointerLeave(theme);
    act(() => vi.advanceTimersByTime(400));
    expect(theme).toHaveAttribute('aria-pressed', 'false');
    fireEvent.pointerEnter(theme);
    act(() => vi.advanceTimersByTime(350));
    expect(theme).toHaveAttribute('aria-pressed', 'true');
    fireEvent.pointerLeave(theme);
    expect(core).toHaveAttribute('aria-pressed', 'true');
  });

  it('moves nodes and attached wires with the right button, then resets one position separately', () => {
    vi.stubGlobal('PointerEvent', MouseEvent);
    render(<GraphScene {...props({ impact: null })} />);
    const board = document.querySelector('.graph-board')!;
    const core = screen.getByRole('button', { name: 'src/core.ts' });
    const theme = screen.getByRole('button', { name: 'src/theme.ts' });
    const baseline = core.getAttribute('transform');
    const themeBaseline = theme.getAttribute('transform');
    const wire = document.querySelector('[data-from="core"] .graph-wire')!;
    const oldWire = wire.getAttribute('d');
    const camera = document.querySelector('.graph-board > g[transform]')!.getAttribute('transform');
    const move = (node: HTMLElement, x: number, y: number) => {
      fireEvent.pointerDown(node, { button: 2, clientX: 200, clientY: 200 });
      fireEvent.pointerMove(board, { buttons: 2, clientX: x, clientY: y });
      fireEvent.pointerUp(board, { button: 2 });
    };
    move(core, 260, 230);
    expect(core.getAttribute('transform')).not.toBe(baseline);
    expect(wire.getAttribute('d')).not.toBe(oldWire);
    expect(wire).toHaveAttribute('marker-end');
    expect(document.querySelector('.graph-board > g[transform]')).toHaveAttribute(
      'transform',
      camera,
    );
    move(theme, 245, 270);
    fireEvent.click(screen.getByTestId('graph-positions'));
    fireEvent.click(screen.getByTestId('graph-reset-selected'));
    expect(core).toHaveAttribute('transform', baseline);
    expect(wire).toHaveAttribute('d', oldWire);
    expect(theme.getAttribute('transform')).not.toBe(themeBaseline);
    fireEvent.click(screen.getByTestId('graph-positions'));
    fireEvent.click(screen.getByTestId('graph-reset-all'));
    expect(theme).toHaveAttribute('transform', themeBaseline);
  });

  it('keeps instructions behind a dismissible help button', () => {
    render(<GraphScene {...props({ impact: null })} />);
    expect(document.querySelector('.graph-direction, .graph-bottom-note')).toBeNull();
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    fireEvent.click(screen.getByTestId('graph-help'));
    expect(screen.getByRole('dialog', { name: '操作指南' })).toBeInTheDocument();
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('lets users inspect the exact causal wave without prematurely marking later files affected', () => {
    render(<GraphScene {...props()} />);
    expect(state('core')).toBe('removed');
    expect(state('service')).toBe('pending');
    expect(state('view')).toBe('pending');
    expect(state('theme')).toBeUndefined();
    expect(screen.getByTestId('graph-untouched-count')).toHaveTextContent('1');
    fireEvent.click(screen.getByTestId('graph-next'));
    expect(state('service')).toBe('affected');
    expect(state('view')).toBe('pending');
    expect(screen.getByTestId('graph-affected-count')).toHaveTextContent('1');
    fireEvent.click(screen.getByTestId('graph-next'));
    expect(state('view')).toBe('affected');
    fireEvent.click(screen.getByTestId('graph-prev'));
    expect(state('view')).toBe('pending');
  });

  it('changes live state on signal arrival and keeps pause stable', () => {
    render(<GraphScene {...props()} />);
    advance(640);
    expect(state('service')).toBe('pending');
    advance(20);
    expect(state('service')).toBe('affected');
    expect(state('view')).toBe('pending');
    fireEvent.click(screen.getByTestId('graph-play'));
    advance(2000);
    expect(state('view')).toBe('pending');
    fireEvent.click(screen.getByTestId('graph-play'));
    advance(700);
    expect(state('view')).toBe('affected');
  });

  it('finishes each pull once, then supports replay and inspection of affected files', () => {
    const onAnimationComplete = vi.fn();
    const onInspectAffected = vi.fn();
    const firstProps = props({ onAnimationComplete, onInspectAffected });
    const { rerender } = render(<GraphScene {...firstProps} />);
    advance(1900);
    expect(onAnimationComplete).toHaveBeenCalledTimes(1);
    rerender(
      <GraphScene
        {...firstProps}
        impact={null}
        report={impact}
        removedIds={['core', 'service', 'view']}
        animationStartedAt={null}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: 'src/service.ts · 步 1 · 受影响' }));
    expect(onInspectAffected).toHaveBeenCalledWith('service');
    expect(screen.getByText('经由')).toBeInTheDocument();
    fireEvent.click(screen.getByTestId('graph-replay'));
    expect(state('service')).toBe('pending');
    advance(1900);
    expect(state('service')).toBe('affected');
    expect(onAnimationComplete).toHaveBeenCalledTimes(1);
  });

  it('keeps playback position when the language and theme change', () => {
    const initialProps = props();
    const { rerender } = render(<GraphScene {...initialProps} />);
    fireEvent.click(screen.getByTestId('graph-next'));
    rerender(<GraphScene {...initialProps} language="en" theme="light" />);
    expect(screen.getByTestId('graph-scene')).toHaveAttribute('data-wave', '1');
    expect(screen.getByTestId('graph-scene')).toHaveAttribute('data-theme', 'light');
    expect(screen.getByRole('button', { name: 'Previous step' })).toBeInTheDocument();
    expect(state('service')).toBe('affected');
    expect(state('view')).toBe('pending');
  });

  it('keeps all-map and direct-neighborhood scopes explicit', () => {
    render(<GraphScene {...props({ impact: null, selectedId: 'service' })} />);
    expect(screen.getByTestId('graph-scene')).toHaveAttribute('data-view', 'overview');
    fireEvent.click(screen.getByTestId('graph-neighbors'));
    expect(screen.getByTestId('graph-scene')).toHaveAttribute('data-view', 'neighbors');
    expect(screen.getAllByTestId('graph-node')).toHaveLength(3);
    fireEvent.click(screen.getByTestId('graph-overview'));
    expect(screen.getByTestId('graph-scene')).toHaveAttribute('data-view', 'overview');
    expect(state('theme')).toBe('standing');
  });

  it('switches between full repository and impact lens without resetting a paused wave', () => {
    const initialProps = props();
    render(<GraphScene {...initialProps} />);
    fireEvent.click(screen.getByTestId('graph-next'));
    expect(state('service')).toBe('affected');
    expect(state('view')).toBe('pending');
    fireEvent.click(screen.getByTestId('graph-overview'));
    expect(screen.getByTestId('graph-scene')).toHaveAttribute('data-view', 'impact-overview');
    expect(screen.getAllByTestId('graph-node')).toHaveLength(4);
    expect(state('theme')).toBe('standing');
    expect(state('service')).toBe('affected');
    expect(state('view')).toBe('pending');
    expect(screen.getByTestId('graph-scene')).toHaveAttribute('data-wave', '1');
    advance(2000);
    expect(state('view')).toBe('pending');
    fireEvent.click(screen.getByTestId('graph-impact-view'));
    expect(screen.getByTestId('graph-scene')).toHaveAttribute('data-view', 'impact');
    expect(screen.getAllByTestId('graph-node')).toHaveLength(3);
    expect(screen.getByTestId('graph-scene')).toHaveAttribute('data-wave', '1');
    expect(initialProps.onAnimationComplete).not.toHaveBeenCalled();
  });

  it('selects healthy files from the completed whole-repository result while keeping affected files inspectable', () => {
    const onSelect = vi.fn();
    const onInspectAffected = vi.fn();
    render(
      <GraphScene
        {...props({
          impact: null,
          report: impact,
          selectedId: null,
          removedIds: ['core', 'service', 'view'],
          disconnectedIds: ['core'],
          onSelect,
          onInspectAffected,
        })}
      />,
    );
    fireEvent.click(screen.getByTestId('graph-overview'));
    fireEvent.click(screen.getByRole('button', { name: 'src/theme.ts · 未受影响' }));
    expect(onSelect).toHaveBeenCalledWith('theme');
    expect(onInspectAffected).not.toHaveBeenCalled();
    expect(document.querySelector('[data-from="theme"]')).toHaveClass('is-selected');
    fireEvent.click(screen.getByRole('button', { name: 'src/service.ts · 步 1 · 受影响' }));
    expect(onInspectAffected).toHaveBeenCalledWith('service');
    expect(onSelect).toHaveBeenLastCalledWith('service');
  });

  it('inspects healthy files without switching the active simulation during playback', () => {
    const onSelect = vi.fn();
    const onInspectAffected = vi.fn();
    render(<GraphScene {...props({ onSelect, onInspectAffected })} />);
    advance(700);
    fireEvent.click(screen.getByTestId('graph-overview'));
    expect(screen.getByTestId('graph-scene')).toHaveAttribute('data-wave', '1');
    fireEvent.click(screen.getByRole('button', { name: 'src/theme.ts · 未受影响' }));
    expect(onSelect).toHaveBeenCalledWith('theme');
    expect(onInspectAffected).toHaveBeenCalledWith('theme');
    advance(700);
    expect(state('view')).toBe('affected');
  });

  it('shows the final wave when the parent completes a paused pull and still permits later replay', () => {
    const initialProps = props();
    const { rerender } = render(<GraphScene {...initialProps} />);
    fireEvent.click(screen.getByTestId('graph-next'));
    expect(state('view')).toBe('pending');
    rerender(
      <GraphScene
        {...initialProps}
        impact={null}
        report={impact}
        removedIds={['core', 'service', 'view']}
        animationStartedAt={null}
      />,
    );
    expect(screen.getByTestId('graph-scene')).toHaveAttribute('data-wave', '2');
    expect(state('view')).toBe('affected');
    expect(screen.getByTestId('graph-affected-count')).toHaveTextContent('2');
    expect(screen.getByTestId('graph-play')).toHaveAttribute('aria-label', '播放');
    fireEvent.click(screen.getByTestId('graph-replay'));
    expect(screen.getByTestId('graph-scene')).toHaveAttribute('data-wave', '0');
    expect(state('service')).toBe('pending');
    advance(700);
    expect(state('service')).toBe('affected');
    expect(state('view')).toBe('pending');
    expect(initialProps.onAnimationComplete).not.toHaveBeenCalled();
  });

  it('reveals a file inspected from external search without changing the paused propagation step', () => {
    const initialProps = props();
    const { rerender } = render(<GraphScene {...initialProps} />);
    fireEvent.click(screen.getByTestId('graph-next'));
    rerender(<GraphScene {...initialProps} inspectId="theme" />);
    expect(screen.getByTestId('graph-scene')).toHaveAttribute('data-view', 'impact-overview');
    expect(screen.getByTestId('graph-scene')).toHaveAttribute('data-wave', '1');
    expect(state('theme')).toBe('standing');
    expect(document.querySelector('.graph-tip-path')).toHaveTextContent('src/theme.ts');
    expect(document.querySelector('.graph-tip-state')).toHaveTextContent('未受影响');
    expect(screen.getByRole('button', { name: 'src/theme.ts · 未受影响' })).toHaveAttribute(
      'aria-pressed',
      'true',
    );
    advance(2000);
    expect(state('view')).toBe('pending');
    fireEvent.click(screen.getByTestId('graph-impact-view'));
    expect(screen.getByTestId('graph-scene')).toHaveAttribute('data-view', 'impact');
    expect(screen.getByTestId('graph-scene')).toHaveAttribute('data-wave', '1');
  });
});
