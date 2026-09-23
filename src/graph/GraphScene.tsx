import { useCallback, useEffect, useLayoutEffect, useId, useMemo, useRef, useState } from 'react';
import {
  ArrowLeft,
  ArrowRight,
  Focus,
  Minus,
  Pause,
  Play,
  Plus,
  RotateCcw,
  Scan,
  SkipForward,
  CircleHelp,
  LayoutGrid,
  X,
} from 'lucide-react';
import type { ImpactResult, RepositoryAnalysis } from '../types/analysis';
import {
  buildGraphLayout,
  causalParents,
  edgeRoute,
  NODE_HEIGHT,
  NODE_WIDTH,
  nodeState,
  WAVE_DURATION,
} from './layout';
import './graph.css';

export interface GraphSceneProps {
  analysis: RepositoryAnalysis;
  selectedId: string | null;
  inspectId?: string | null;
  inspectToken?: number;
  removedIds: string[];
  disconnectedIds?: string[];
  impact: ImpactResult | null;
  report: ImpactResult | null;
  reports?: ImpactResult[];
  onReviewImpact?: (id: string) => void;
  animationStartedAt: number | null;
  onSelect: (id: string | null) => void;
  onAnimationComplete: () => void;
  resetToken: number;
  theme: 'dark' | 'light';
  language: 'zh' | 'en' | 'ja';
  reducedMotion: boolean;
  onInspectAffected?: (id: string) => void;
}

const copy = {
  zh: {
    direction: '依赖 → 使用它的文件',
    overview: '全部',
    neighbors: '相邻',
    map: '依赖图',
    impact: '影响路径',
    removed: '已抽出',
    affected: '受影响',
    pending: '尚未波及',
    untouched: '未受影响',
    previous: '此前不可用',
    prev: '上一步',
    next: '下一步',
    play: '播放',
    pause: '暂停',
    replay: '重播',
    skip: '到结果',
    fit: '适应画布',
    zoomIn: '放大',
    zoomOut: '缩小',
    via: '经由',
    wave: '步',
    complete: '传播完成',
    hidden: '未显示',
    outside: '其他文件',
    hint: '点击文件 · 拖动平移 · 滚轮缩放',
    origin: '抽出',
    direct: '直接',
    depth: '层',
    depends: '依赖',
    callers: '使用方',
    self: '当前文件',
    scrub: '影响传播步骤',
    cross: '其他真实依赖',
    active: '最短影响路径',
    scope: '显示',
    empty: '没有可显示的文件',
    select: '选择文件',
    unavailable: '不可用',
    help: '操作指南',
    close: '关闭',
    position: '还原位置',
    resetSelected: '还原选中文件的位置',
    resetAll: '还原所有文件的位置',
    helpItems: [
      '单击选择；双击进入相邻视图。',
      '全部视图中，按住鼠标右键拖动文件；连线会跟随。',
      '拖动空白处平移，滚轮缩放；R 适应画布。',
      '位置按钮可还原选中或全部文件的位置。',
      '重新选中已断开的文件，点击“恢复文件”。Ctrl+Z 撤销。',
      '“影响路径”可随时回看；多次断开可切换记录。',
      '箭头从依赖文件指向使用它的文件。模拟不修改源码。',
    ],
  },
  en: {
    direction: 'Dependency → its consumer',
    overview: 'All',
    neighbors: 'Nearby',
    map: 'Dependencies',
    impact: 'Impact path',
    removed: 'Removed',
    affected: 'Affected',
    pending: 'Pending',
    untouched: 'Unaffected',
    previous: 'Previously unavailable',
    prev: 'Previous step',
    next: 'Next step',
    play: 'Play',
    pause: 'Pause',
    replay: 'Replay',
    skip: 'Show result',
    fit: 'Fit to view',
    zoomIn: 'Zoom in',
    zoomOut: 'Zoom out',
    via: 'via',
    wave: 'Step',
    complete: 'Propagation complete',
    hidden: 'not shown',
    outside: 'other files',
    hint: 'Click a file · Drag to pan · Scroll to zoom',
    origin: 'Removed',
    direct: 'Direct',
    depth: 'Layer',
    depends: 'Dependencies',
    callers: 'Consumers',
    self: 'Selected file',
    scrub: 'Impact propagation step',
    cross: 'Other real dependencies',
    active: 'Shortest impact path',
    scope: 'Showing',
    empty: 'No files to show',
    select: 'Select file',
    unavailable: 'Unavailable',
    help: 'Quick guide',
    close: 'Close',
    position: 'Reset positions',
    resetSelected: 'Reset selected file position',
    resetAll: 'Reset all file positions',
    helpItems: [
      'Click to select; double-click to open Nearby.',
      'In All, drag a file with the right mouse button. Connections follow.',
      'Drag the background to pan, scroll to zoom; R fits the view.',
      'Reset positions for the selected file or for all files.',
      'Select a disconnected file again to restore it. Ctrl+Z undoes an action.',
      'Revisit Impact path at any time; choose a cut to review earlier results.',
      'Arrows lead from dependencies to consumers. Source files stay unchanged.',
    ],
  },
  ja: {
    direction: '依存先 → 利用するファイル',
    overview: '全体',
    neighbors: '隣接',
    map: '依存関係',
    impact: '影響経路',
    removed: '取り外し',
    affected: '影響あり',
    pending: '未到達',
    untouched: '影響なし',
    previous: '以前から利用不可',
    prev: '前のステップ',
    next: '次のステップ',
    play: '再生',
    pause: '一時停止',
    replay: '再生し直す',
    skip: '結果へ',
    fit: '全体を表示',
    zoomIn: '拡大',
    zoomOut: '縮小',
    via: '経由',
    wave: '段階',
    complete: '伝播完了',
    hidden: '非表示',
    outside: '他のファイル',
    hint: 'クリックで選択 · ドラッグで移動 · スクロールで拡大',
    origin: '取り外し',
    direct: '直接',
    depth: '層',
    depends: '依存先',
    callers: '利用側',
    self: '選択中',
    scrub: '影響の伝播段階',
    cross: 'その他の依存関係',
    active: '最短の影響経路',
    scope: '表示',
    empty: '表示するファイルがありません',
    select: 'ファイルを選択',
    unavailable: '利用不可',
    help: '操作ガイド',
    close: '閉じる',
    position: '位置を戻す',
    resetSelected: '選択したファイルの位置を戻す',
    resetAll: 'すべての位置を戻す',
    helpItems: [
      'クリックで選択、ダブルクリックで隣接表示。',
      '全体表示で右ボタンを押しながらファイルを移動。線も追従します。',
      '背景をドラッグして移動、スクロールで拡大。R で全体を表示。',
      '位置ボタンで選択したファイル、またはすべての位置を復元。',
      '切断したファイルを再選択して復元。Ctrl+Z で操作を取り消し。',
      '影響経路はいつでも再表示でき、過去の切断も選べます。',
      '矢印は依存先から利用側へ。ソースは変更しません。',
    ],
  },
};
const palette = ['#71d8cb', '#a6cb70', '#aaa0ef', '#eeac70', '#6cb8e1', '#e68fb9'];
function moduleColor(directory: string): string {
  let hash = 0;
  for (const character of directory) hash = ((hash << 5) - hash + character.charCodeAt(0)) | 0;
  return palette[Math.abs(hash) % palette.length];
}
function shorten(label: string, limit: number): string {
  return label.length <= limit ? label : `${label.slice(0, Math.max(2, limit - 1))}…`;
}
type Camera = { x: number; y: number; scale: number };
type View = 'overview' | 'neighbors' | 'impact';
type Offset = { x: number; y: number };

export default function GraphScene({
  analysis,
  selectedId,
  inspectId,
  inspectToken,
  removedIds,
  disconnectedIds,
  impact,
  report,
  reports = [],
  onReviewImpact,
  animationStartedAt,
  onSelect,
  onAnimationComplete,
  resetToken,
  theme,
  language,
  reducedMotion,
  onInspectAffected,
}: GraphSceneProps) {
  const t = copy[language];
  const activeReport = impact ?? report;
  const reportOptions = impact ? [...reports, impact] : reports;
  const [view, setView] = useState<View>(activeReport ? 'impact' : 'overview');
  const [neighborId, setNeighborId] = useState(selectedId);
  const [offsets, setOffsets] = useState<Record<string, Offset>>({});
  const [help, setHelp] = useState(false);
  const [positionMenu, setPositionMenu] = useState<{
    x: number;
    y: number;
    id: string | null;
  } | null>(null);
  const [draggingNode, setDraggingNode] = useState(false);
  const isOverview = view === 'overview';
  const baseLayout = useMemo(
    () =>
      buildGraphLayout(analysis, {
        impact: activeReport,
        selectedId: view === 'neighbors' ? neighborId : selectedId,
        inspectId,
        neighborhood: view === 'neighbors',
        removedIds,
        impactOverview: view !== 'impact',
      }),
    [analysis, activeReport, selectedId, inspectId, view, neighborId, removedIds],
  );
  const layout = useMemo(() => {
    if (!isOverview) return baseLayout;
    const nodes = baseLayout.nodes.map((node) => ({
      ...node,
      x: node.x + (offsets[node.id]?.x ?? 0),
      y: node.y + (offsets[node.id]?.y ?? 0),
    }));
    return { ...baseLayout, nodes, byId: new Map(nodes.map((node) => [node.id, node])) };
  }, [baseLayout, isOverview, offsets]);
  const removed = useMemo(() => new Set(removedIds), [removedIds]);
  const disconnected = useMemo(
    () => (disconnectedIds ? new Set(disconnectedIds) : undefined),
    [disconnectedIds],
  );
  const maxWave = Math.max(0, (activeReport?.waves.length ?? 1) - 1);
  const [playhead, setPlayhead] = useState(impact ? 0 : maxWave);
  const [playing, setPlaying] = useState(Boolean(impact));
  const [hoveredId, setHoveredId] = useState<string | null>(null);
  const [inspectedId, setInspectedId] = useState<string | null>(null);
  const [viewport, setViewport] = useState({ width: 920, height: 470 });
  const [camera, setCamera] = useState<Camera>({ x: 0, y: 0, scale: 1 });
  const containerRef = useRef<HTMLDivElement>(null);
  const cameraRef = useRef(camera);
  const headRef = useRef(playhead);
  const callbackRef = useRef(onAnimationComplete);
  const completed = useRef<number | null | undefined>(undefined);
  const lastReport = useRef<ImpactResult | null>(null);
  const lastLiveImpact = useRef<ImpactResult | null>(null);
  const drag = useRef<{
    x: number;
    y: number;
    camera: Camera;
    moved: boolean;
    pointerId: number;
    nodeId?: string;
    offset?: Offset;
  } | null>(null);
  const rightDragged = useRef(false);
  const hoverTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const savedCameras = useRef(new Map<string, Camera>());
  const cameraScope = useRef<{
    key: string;
    analysis: RepositoryAnalysis;
    reset: number;
    width: number;
    height: number;
  } | null>(null);
  const suppressClick = useRef(false);
  const unique = useId().replace(/:/g, '');
  cameraRef.current = camera;
  headRef.current = playhead;
  callbackRef.current = onAnimationComplete;
  const clearHover = useCallback(() => {
    if (hoverTimer.current) clearTimeout(hoverTimer.current);
    hoverTimer.current = null;
    setHoveredId(null);
  }, []);
  useEffect(() => clearHover(), [selectedId, view, clearHover]);
  useEffect(() => setInspectedId(null), [selectedId]);
  useEffect(
    () => () => {
      if (hoverTimer.current) clearTimeout(hoverTimer.current);
    },
    [],
  );
  useEffect(() => {
    setOffsets({});
    setPositionMenu(null);
    setHelp(false);
    setView(activeReport ? 'impact' : 'overview');
    setNeighborId(null);
    setInspectedId(null);
  }, [analysis]);
  useEffect(() => {
    const close = (event: PointerEvent) => {
      if (!(event.target as Element).closest('.graph-position-menu, .graph-position-toggle'))
        setPositionMenu(null);
      if (!(event.target as Element).closest('.graph-help, .graph-help-toggle')) setHelp(false);
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setHelp(false);
        setPositionMenu(null);
        clearHover();
        setInspectedId(null);
      }
    };
    window.addEventListener('pointerdown', close);
    window.addEventListener('keydown', escape);
    return () => {
      window.removeEventListener('pointerdown', close);
      window.removeEventListener('keydown', escape);
    };
  }, [clearHover]);

  useEffect(() => {
    const element = containerRef.current;
    if (!element) return;
    const measure = () => {
      const rect = element.getBoundingClientRect();
      if (rect.width && rect.height) setViewport({ width: rect.width, height: rect.height });
    };
    measure();
    if (typeof ResizeObserver !== 'undefined') {
      const observer = new ResizeObserver(measure);
      observer.observe(element);
      return () => observer.disconnect();
    }
    window.addEventListener('resize', measure);
    return () => window.removeEventListener('resize', measure);
  }, []);

  const fit = useCallback(() => {
    const minX = Math.min(0, ...layout.nodes.map((node) => node.x - 30));
    const minY = Math.min(0, ...layout.nodes.map((node) => node.y - 30));
    const width =
      Math.max(layout.width, ...layout.nodes.map((node) => node.x + NODE_WIDTH + 30)) - minX;
    const height =
      Math.max(layout.height, ...layout.nodes.map((node) => node.y + NODE_HEIGHT + 30)) - minY;
    const availableHeight = Math.max(140, viewport.height - (activeReport ? 116 : 86));
    const scale = Math.min(1.35, (viewport.width - 40) / width, availableHeight / height);
    setCamera({
      x: (viewport.width - width * scale) / 2 - minX * scale,
      y: 48 + (availableHeight - height * scale) / 2 - minY * scale,
      scale,
    });
  }, [layout, viewport, Boolean(activeReport)]);
  const fitRef = useRef(fit);
  fitRef.current = fit;
  const viewKey =
    view === 'neighbors'
      ? `neighbors:${neighborId}`
      : view === 'impact'
        ? `impact:${activeReport?.removedModuleId}`
        : 'overview';
  useLayoutEffect(() => {
    const previous = cameraScope.current;
    const fresh =
      !previous ||
      previous.analysis !== analysis ||
      previous.reset !== resetToken ||
      previous.width !== viewport.width ||
      previous.height !== viewport.height;
    if (fresh) savedCameras.current.clear();
    else if (previous.key !== viewKey) savedCameras.current.set(previous.key, cameraRef.current);
    if (fresh || previous?.key !== viewKey) {
      const saved = savedCameras.current.get(viewKey);
      if (saved) setCamera(saved);
      else fitRef.current();
    }
    cameraScope.current = { key: viewKey, analysis, reset: resetToken, ...viewport };
  }, [analysis, resetToken, viewKey, viewport.width, viewport.height]);

  const zoom = useCallback(
    (factor: number, at?: { x: number; y: number }) => {
      setCamera((current) => {
        const scale = Math.min(3.5, Math.max(0.12, current.scale * factor));
        const center = at ?? { x: viewport.width / 2, y: viewport.height / 2 };
        const ratio = scale / current.scale;
        return {
          scale,
          x: center.x - (center.x - current.x) * ratio,
          y: center.y - (center.y - current.y) * ratio,
        };
      });
    },
    [viewport],
  );

  useEffect(() => {
    const element = containerRef.current;
    if (!element) return;
    const wheel = (event: WheelEvent) => {
      if (
        (event.target as Element).closest(
          'button, input, select, .graph-help, .graph-position-menu',
        ) ||
        drag.current
      )
        return;
      event.preventDefault();
      const rect = element.getBoundingClientRect();
      zoom(Math.exp(-event.deltaY * 0.0015), {
        x: event.clientX - rect.left,
        y: event.clientY - rect.top,
      });
    };
    element.addEventListener('wheel', wheel, { passive: false });
    return () => element.removeEventListener('wheel', wheel);
  }, [zoom]);

  useEffect(() => {
    if (activeReport !== lastReport.current) {
      lastReport.current = activeReport;
      setPlayhead(impact ? 0 : maxWave);
      headRef.current = impact ? 0 : maxWave;
      setPlaying(Boolean(impact));
      if (impact) setView('impact');
      else if (!activeReport) setView((current) => (current === 'impact' ? 'overview' : current));
      setInspectedId(null);
      clearHover();
    } else if (lastLiveImpact.current && !impact && activeReport) {
      // The app can finish from its own result button while this view is paused.
      // Only synchronize that transition; later replay remains independent.
      setPlayhead(maxWave);
      headRef.current = maxWave;
      setPlaying(false);
    }
    lastLiveImpact.current = impact;
  }, [activeReport, impact, maxWave, clearHover]);

  useEffect(() => {
    if (!inspectId || !analysis.modules.some((module) => module.id === inspectId)) return;
    setInspectedId(inspectId);
    setHoveredId(null);
    if (!layout.byId.has(inspectId)) setView('overview');
  }, [inspectId, inspectToken, activeReport, analysis]);

  useEffect(() => {
    if (!inspectId) return;
    const node = layout.byId.get(inspectId);
    if (!node) return;
    const current = cameraRef.current;
    const x = node.x * current.scale + current.x,
      y = node.y * current.scale + current.y;
    if (
      x < 16 ||
      x + NODE_WIDTH * current.scale > viewport.width - 16 ||
      y < 48 ||
      y + NODE_HEIGHT * current.scale > viewport.height - 110
    )
      setCamera({
        ...current,
        x: viewport.width / 2 - (node.x + NODE_WIDTH / 2) * current.scale,
        y: viewport.height / 2 - (node.y + NODE_HEIGHT / 2) * current.scale,
      });
  }, [inspectId, inspectToken, layout.kind]);

  const complete = useCallback(() => {
    if (impact && completed.current !== animationStartedAt) {
      completed.current = animationStartedAt;
      callbackRef.current();
    }
  }, [impact, animationStartedAt]);

  useEffect(() => {
    if (!activeReport || !playing) return;
    let frame = 0;
    let last = performance.now();
    let elapsedAtEnd = 0;
    const tick = (now: number) => {
      const delta = Math.min(80, Math.max(0, now - last));
      last = now;
      const next = Math.min(
        maxWave,
        headRef.current + delta / (reducedMotion ? 180 : WAVE_DURATION),
      );
      headRef.current = next;
      setPlayhead(next);
      if (next >= maxWave) {
        elapsedAtEnd += delta;
        if (elapsedAtEnd >= (reducedMotion ? 80 : 400)) {
          setPlaying(false);
          complete();
          return;
        }
      }
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [activeReport, playing, maxWave, complete, reducedMotion]);

  const seek = (step: number) => {
    setPlaying(false);
    const value = Math.max(0, Math.min(maxWave, step));
    headRef.current = value;
    setPlayhead(value);
  };
  const replay = () => {
    headRef.current = 0;
    setPlayhead(0);
    setPlaying(true);
  };
  const currentWave = Math.floor(playhead + 0.0001);
  const arrived =
    activeReport?.waves.slice(1, currentWave + 1).reduce((sum, wave) => sum + wave.length, 0) ?? 0;
  const highlightedId = hoveredId ?? inspectedId ?? selectedId;
  const showTrace = Boolean(
    activeReport && (view === 'impact' || !highlightedId || playing || impact),
  );
  const highlighted = highlightedId ? layout.byId.get(highlightedId) : undefined;
  const highlightedState = highlighted
    ? nodeState(highlighted, playhead, removed, disconnected)
    : null;
  const parents = highlighted && activeReport ? causalParents(layout, highlighted.id) : [];
  const nodeFont = Math.max(12, Math.min(23, 11.5 / camera.scale));
  const labelLimit = Math.floor((NODE_WIDTH - 30) / (nodeFont * 0.58));
  const edges = useMemo(
    () =>
      layout.edges.map((edge) => ({
        ...edge,
        route: edgeRoute(layout.byId.get(edge.from)!, layout.byId.get(edge.to)!),
      })),
    [layout],
  );
  const causeNames = useMemo(() => {
    const names = new Map<string, string[]>();
    for (const edge of layout.edges)
      if (edge.causal) {
        names.set(edge.to, [
          ...(names.get(edge.to) ?? []),
          layout.byId.get(edge.from)!.module.fileName,
        ]);
      }
    return names;
  }, [layout]);
  const focusedNeighbors = useMemo(() => {
    if (!highlightedId) return null;
    const ids = new Set([highlightedId]);
    for (const edge of layout.edges)
      if (edge.from === highlightedId || edge.to === highlightedId) {
        ids.add(edge.from);
        ids.add(edge.to);
      }
    return ids;
  }, [layout, highlightedId]);

  const inspect = (id: string) => {
    if (suppressClick.current) return;
    const node = layout.byId.get(id)!;
    clearHover();
    setInspectedId(id);
    if (impact || node.wave !== null || removed.has(id)) onInspectAffected?.(id);
    onSelect(id);
  };
  const showNeighbors = (id: string) => {
    clearHover();
    setInspectedId(id);
    onSelect(id);
    setNeighborId(id);
    setView('neighbors');
    setPositionMenu(null);
  };
  const resetPosition = (id?: string | null) => {
    setOffsets((current) => {
      if (!id) return {};
      const next = { ...current };
      delete next[id];
      return next;
    });
    setPositionMenu(null);
  };

  return (
    <div
      className={`graph-scene${draggingNode ? ' is-moving-node' : ''}`}
      data-testid="graph-scene"
      data-theme={theme}
      data-view={layout.kind}
      data-wave={currentWave}
      ref={containerRef}
    >
      <div className="graph-heading">
        <div className="graph-tools">
          <button
            type="button"
            className="graph-help-toggle"
            data-testid="graph-help"
            title={t.help}
            aria-label={t.help}
            aria-expanded={help}
            onClick={() => {
              setHelp(!help);
              setPositionMenu(null);
            }}
          >
            <CircleHelp size={15} />
          </button>
          {isOverview && (
            <button
              type="button"
              className="graph-position-toggle"
              data-testid="graph-positions"
              title={t.position}
              aria-label={t.position}
              aria-expanded={!!positionMenu}
              onClick={() => {
                setPositionMenu(positionMenu ? null : { x: 20, y: 46, id: selectedId });
                setHelp(false);
              }}
            >
              <LayoutGrid size={15} />
            </button>
          )}
        </div>
        <div className="graph-scope">
          {activeReport && reportOptions.length > 1 && (
            <select
              className="graph-report-select"
              data-testid="graph-report-select"
              aria-label={t.impact}
              value={activeReport.removedModuleId}
              disabled={!!impact}
              onChange={(event) => {
                onReviewImpact?.(event.target.value);
                setView('impact');
              }}
            >
              {reportOptions.map((item) => (
                <option key={item.removedModuleId} value={item.removedModuleId}>
                  {item.removedModuleId.split('/').at(-1)} ·{' '}
                  {item.removedModuleId.split('/').slice(0, -1).join('/') || '/'}
                </option>
              ))}
            </select>
          )}
          <div className="graph-view-switch">
            <button
              type="button"
              data-testid="graph-overview"
              className={isOverview ? 'is-active' : ''}
              aria-pressed={isOverview}
              onClick={() => setView('overview')}
            >
              {t.overview}
            </button>
            <button
              type="button"
              data-testid="graph-neighbors"
              className={view === 'neighbors' ? 'is-active' : ''}
              aria-pressed={view === 'neighbors'}
              disabled={!selectedId}
              onClick={() => selectedId && showNeighbors(selectedId)}
            >
              {t.neighbors}
            </button>
            {activeReport && (
              <button
                type="button"
                data-testid="graph-impact-view"
                className={view === 'impact' ? 'is-active' : ''}
                aria-pressed={view === 'impact'}
                onClick={() => setView('impact')}
              >
                {t.impact}
              </button>
            )}
          </div>
          {layout.hiddenCount > 0 && (
            <span className="graph-limit">
              {layout.nodes.length} / {layout.nodes.length + layout.hiddenCount} ·{' '}
              {layout.hiddenCount} {t.hidden}
            </span>
          )}
        </div>
      </div>
      {help && (
        <div className="graph-help" role="dialog" aria-label={t.help}>
          <div>
            <strong>{t.help}</strong>
            <button type="button" aria-label={t.close} onClick={() => setHelp(false)}>
              <X size={14} />
            </button>
          </div>
          <ul>
            {t.helpItems.map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ul>
        </div>
      )}
      {positionMenu && (
        <div
          className="graph-position-menu"
          role="menu"
          style={{ left: positionMenu.x, top: positionMenu.y }}
        >
          <button
            type="button"
            role="menuitem"
            data-testid="graph-reset-selected"
            disabled={!positionMenu.id || !offsets[positionMenu.id]}
            onClick={() => resetPosition(positionMenu.id)}
          >
            {t.resetSelected}
          </button>
          <button
            type="button"
            role="menuitem"
            data-testid="graph-reset-all"
            disabled={!Object.keys(offsets).length}
            onClick={() => resetPosition()}
          >
            {t.resetAll}
          </button>
        </div>
      )}

      <svg
        className="graph-board"
        width="100%"
        height="100%"
        aria-label={activeReport ? t.impact : t.map}
        onPointerDown={(event) => {
          rightDragged.current = false;
          const nodeId = (event.target as Element).closest('[data-id]')?.getAttribute('data-id');
          if (event.button !== 0 && !(event.button === 2 && nodeId && isOverview)) return;
          clearHover();
          drag.current = {
            x: event.clientX,
            y: event.clientY,
            camera: cameraRef.current,
            moved: false,
            pointerId: event.pointerId,
            ...(event.button === 2 && nodeId
              ? { nodeId, offset: offsets[nodeId] ?? { x: 0, y: 0 } }
              : {}),
          };
          suppressClick.current = false;
          if (event.button === 2 && nodeId) {
            event.preventDefault();
            setInspectedId(nodeId);
            onSelect(nodeId);
            event.currentTarget.setPointerCapture?.(event.pointerId);
          }
        }}
        onPointerMove={(event) => {
          const start = drag.current;
          if (!start) return;
          const dx = event.clientX - start.x;
          const dy = event.clientY - start.y;
          if (Math.abs(dx) + Math.abs(dy) > 4) start.moved = true;
          if (start.moved) {
            event.currentTarget.setPointerCapture?.(event.pointerId);
            suppressClick.current = true;
            if (start.nodeId) {
              const id = start.nodeId;
              rightDragged.current = true;
              setDraggingNode(true);
              setPositionMenu(null);
              setOffsets((current) => ({
                ...current,
                [id]: {
                  x: start.offset!.x + dx / start.camera.scale,
                  y: start.offset!.y + dy / start.camera.scale,
                },
              }));
            } else setCamera({ ...start.camera, x: start.camera.x + dx, y: start.camera.y + dy });
            clearHover();
          }
        }}
        onPointerUp={(event) => {
          drag.current = null;
          setDraggingNode(false);
          if (event.currentTarget.hasPointerCapture?.(event.pointerId))
            event.currentTarget.releasePointerCapture(event.pointerId);
        }}
        onPointerCancel={() => {
          drag.current = null;
          setDraggingNode(false);
          clearHover();
        }}
        onLostPointerCapture={() => {
          drag.current = null;
          setDraggingNode(false);
        }}
        onPointerLeave={() => {
          clearHover();
        }}
        onContextMenu={(event) => {
          event.preventDefault();
          if (rightDragged.current || !isOverview) return;
          const rect = event.currentTarget.getBoundingClientRect();
          const id =
            (event.target as Element).closest('[data-id]')?.getAttribute('data-id') ?? selectedId;
          setPositionMenu({
            x: Math.max(12, Math.min(viewport.width - 260, event.clientX - rect.left)),
            y: Math.max(12, Math.min(viewport.height - 86, event.clientY - rect.top)),
            id,
          });
          setHelp(false);
        }}
        onClick={(event) => {
          if (
            !suppressClick.current &&
            !(event.target as Element).closest('[data-testid="graph-node"]')
          ) {
            setInspectedId(null);
            clearHover();
            onSelect(null);
          }
        }}
      >
        <defs>
          <pattern id={`${unique}-dots`} width="22" height="22" patternUnits="userSpaceOnUse">
            <circle cx="1" cy="1" r="0.75" className="graph-grid-dot" />
          </pattern>
          {['normal', 'causal', 'quiet', 'selected'].map((kind) => (
            <marker
              key={kind}
              id={`${unique}-${kind}`}
              viewBox="0 0 8 8"
              refX="7"
              refY="4"
              markerWidth="5"
              markerHeight="5"
              orient="auto-start-reverse"
            >
              <path d="M1 1 L7 4 L1 7" className={`graph-arrow graph-arrow-${kind}`} />
            </marker>
          ))}
          <filter id={`${unique}-glow`} x="-100%" y="-100%" width="300%" height="300%">
            <feGaussianBlur stdDeviation="3" />
          </filter>
        </defs>
        <rect width="100%" height="100%" fill={`url(#${unique}-dots)`} pointerEvents="none" />
        <g transform={`translate(${camera.x}, ${camera.y}) scale(${camera.scale})`}>
          {layout.columns.map((column) => {
            const label =
              layout.kind === 'impact'
                ? column.index === 0
                  ? t.origin
                  : column.index === 1
                    ? t.direct
                    : `${t.wave} ${column.index}`
                : layout.kind === 'neighbors'
                  ? [t.depends, t.self, t.callers][column.index]
                  : `${t.depth} ${column.index}`;
            return (
              <g key={column.index} className="graph-column">
                <text
                  x={column.x + NODE_WIDTH / 2}
                  y="23"
                  textAnchor="middle"
                  fontSize={Math.min(20, 10 / camera.scale)}
                >
                  {label}
                </text>
                <path d={`M${column.x + NODE_WIDTH / 2} 37 V${layout.height - 8}`} />
              </g>
            );
          })}
          {edges.map((edge) => {
            const target = layout.byId.get(edge.to)!;
            const related = highlightedId === edge.from || highlightedId === edge.to;
            const reached = activeReport && edge.causal && (target.wave ?? 0) <= playhead;
            const transmitting =
              showTrace &&
              edge.causal &&
              target.wave !== null &&
              playhead > target.wave - 1 &&
              playhead < target.wave;
            const routeClass = showTrace
              ? edge.causal
                ? reached
                  ? 'is-reached'
                  : 'is-waiting'
                : 'is-context'
              : related
                ? 'is-selected'
                : '';
            const progress = target.wave === null ? 0 : playhead - (target.wave - 1);
            const point = edge.route.point(Math.max(0, Math.min(1, progress)));
            const marker = showTrace
              ? edge.causal && reached
                ? 'causal'
                : 'quiet'
              : related
                ? 'selected'
                : 'normal';
            return (
              <g
                key={edge.id}
                className={`graph-edge ${routeClass}${highlightedId && !related && !showTrace ? ' is-dim' : ''}`}
                data-testid="graph-edge"
                data-from={edge.from}
                data-to={edge.to}
                data-causal={edge.causal}
              >
                <title>
                  {layout.byId.get(edge.from)!.module.fileName} → {target.module.fileName}
                  {activeReport ? ` · ${edge.causal ? t.active : t.cross}` : ''}
                </title>
                <path
                  className="graph-wire"
                  d={edge.route.path}
                  markerEnd={`url(#${unique}-${marker})`}
                />
                {transmitting && !reducedMotion && (
                  <g className="graph-pulse">
                    <circle
                      cx={point.x}
                      cy={point.y}
                      r="8"
                      filter={`url(#${unique}-glow)`}
                      opacity="0.65"
                    />
                    <circle cx={point.x} cy={point.y} r="3.8" />
                    <circle cx={point.x} cy={point.y} r="1.6" fill="#fff" />
                  </g>
                )}
              </g>
            );
          })}
          {layout.nodes.map((node) => {
            const state = nodeState(node, playhead, removed, disconnected);
            const selected = highlightedId === node.id;
            const dim = Boolean(focusedNeighbors && !focusedNeighbors.has(node.id) && !showTrace);
            const color = moduleColor(node.module.directory);
            const parentNames = causeNames.get(node.id) ?? [];
            const status =
              state === 'standing'
                ? activeReport
                  ? t.untouched
                  : ''
                : state === 'removed'
                  ? node.wave === 0 || disconnected?.has(node.id)
                    ? t.removed
                    : t.unavailable
                  : state === 'affected' && node.wave === null
                    ? t.previous
                    : t[state];
            const statusLabel =
              node.wave !== null && node.wave > 0 ? `${t.wave} ${node.wave} · ${status}` : status;
            return (
              <g
                key={node.id}
                transform={`translate(${node.x}, ${node.y})`}
                className={`graph-node is-${state}${selected ? ' is-selected' : ''}${dim ? ' is-dim' : ''}`}
                style={{ '--node-color': color } as React.CSSProperties}
                role="button"
                tabIndex={0}
                aria-label={`${node.module.relativePath}${statusLabel ? ` · ${statusLabel}` : ''}`}
                aria-pressed={selected}
                data-testid="graph-node"
                data-id={node.id}
                data-wave={node.wave ?? ''}
                data-state={state}
                onPointerEnter={() => {
                  if (drag.current) return;
                  clearHover();
                  if ((selectedId || inspectedId) && node.id !== (inspectedId ?? selectedId))
                    hoverTimer.current = setTimeout(() => {
                      hoverTimer.current = null;
                      setHoveredId(node.id);
                    }, 350);
                  else setHoveredId(node.id);
                }}
                onPointerLeave={clearHover}
                onFocus={() => {
                  if (!drag.current) setHoveredId(node.id);
                }}
                onBlur={clearHover}
                onClick={(event) => {
                  event.stopPropagation();
                  inspect(node.id);
                }}
                onDoubleClick={(event) => {
                  event.stopPropagation();
                  if (!suppressClick.current) showNeighbors(node.id);
                }}
                onKeyDown={(event) => {
                  if (event.key === 'Enter' || event.key === ' ') {
                    event.preventDefault();
                    suppressClick.current = false;
                    if (event.key === 'Enter' && event.altKey) showNeighbors(node.id);
                    else inspect(node.id);
                  }
                }}
              >
                <title>
                  {node.module.relativePath}
                  {statusLabel ? `\n${statusLabel}` : ''}
                  {parentNames.length ? `\n${t.via}: ${parentNames.join(', ')}` : ''}
                </title>
                <rect
                  className="graph-node-halo"
                  x="-3"
                  y="-3"
                  width={NODE_WIDTH + 6}
                  height={NODE_HEIGHT + 6}
                  rx="10"
                />
                <rect className="graph-node-body" width={NODE_WIDTH} height={NODE_HEIGHT} rx="7" />
                {node.wave === 0 ? (
                  <>
                    <path className="graph-cut" d="M-8 15 L-2 21 L-8 27 M166 15 L160 21 L166 27" />
                    <path
                      className="graph-cut-line"
                      d={`M4 ${NODE_HEIGHT - 8} H${NODE_WIDTH - 4}`}
                    />
                  </>
                ) : (
                  <circle className="graph-node-dot" cx="12" cy={NODE_HEIGHT / 2} r="2.5" />
                )}
                <text
                  className="graph-node-label"
                  x={node.wave === 0 ? NODE_WIDTH / 2 : 22}
                  y={NODE_HEIGHT / 2 + nodeFont * 0.35}
                  fontSize={nodeFont}
                  textAnchor={node.wave === 0 ? 'middle' : undefined}
                >
                  {shorten(node.module.fileName, labelLimit)}
                </text>
                <circle className="graph-port graph-port-in" cx="0" cy={NODE_HEIGHT / 2} r="2" />
                <circle
                  className="graph-port graph-port-out"
                  cx={NODE_WIDTH}
                  cy={NODE_HEIGHT / 2}
                  r="2"
                />
              </g>
            );
          })}
        </g>
      </svg>

      {!layout.nodes.length && <div className="graph-empty">{t.empty}</div>}
      <div className="graph-zoom">
        <button
          type="button"
          title={t.zoomOut}
          aria-label={t.zoomOut}
          onClick={() => zoom(1 / 1.25)}
        >
          <Minus size={14} />
        </button>
        <button
          type="button"
          title={t.fit}
          aria-label={t.fit}
          data-testid="graph-fit"
          onClick={fit}
        >
          <Scan size={14} />
        </button>
        <button type="button" title={t.zoomIn} aria-label={t.zoomIn} onClick={() => zoom(1.25)}>
          <Plus size={14} />
        </button>
      </div>

      {highlighted && (
        <div className={`graph-file-tip${activeReport ? ' has-report' : ''}`}>
          <span className="graph-tip-path">{highlighted.module.relativePath}</span>
          {activeReport && highlightedState && (
            <span className={`graph-tip-state is-${highlightedState}`}>
              {highlighted.wave === null && removed.has(highlighted.id)
                ? t.previous
                : highlightedState === 'standing'
                  ? t.untouched
                  : highlightedState === 'removed'
                    ? highlighted.wave === 0 || disconnected?.has(highlighted.id)
                      ? t.removed
                      : t.unavailable
                    : t[highlightedState]}
            </span>
          )}
          {parents.length > 0 && (
            <span className="graph-tip-via">
              {t.via} <b>{parents.map((node) => node.module.fileName).join(' + ')}</b>
            </span>
          )}
        </div>
      )}

      {activeReport ? (
        <>
          <div className="graph-impact-summary" aria-live="polite">
            <span>
              <i className="is-affected" />
              {t.affected} <b data-testid="graph-affected-count">{arrived}</b>
              <em>/{activeReport.totalAffected}</em>
            </span>
            <span>
              <i className="is-untouched" />
              {t.untouched} <b data-testid="graph-untouched-count">{layout.untouchedCount}</b>
            </span>
            {layout.previousRemovedCount > 0 && (
              <span>
                {t.previous} <b>{layout.previousRemovedCount}</b>
              </span>
            )}
          </div>
          <div className="graph-playback">
            <button
              type="button"
              title={t.replay}
              aria-label={t.replay}
              data-testid="graph-replay"
              onClick={replay}
            >
              <RotateCcw size={14} />
            </button>
            <button
              type="button"
              title={t.prev}
              aria-label={t.prev}
              data-testid="graph-prev"
              disabled={playhead <= 0}
              onClick={() => seek(Math.ceil(playhead) - 1)}
            >
              <ArrowLeft size={14} />
            </button>
            <button
              type="button"
              className="graph-play-button"
              title={playing ? t.pause : t.play}
              aria-label={playing ? t.pause : t.play}
              data-testid="graph-play"
              onClick={() => {
                if (!playing && playhead >= maxWave) replay();
                else setPlaying(!playing);
              }}
            >
              {playing ? (
                <Pause size={13} fill="currentColor" />
              ) : (
                <Play size={13} fill="currentColor" />
              )}
            </button>
            <button
              type="button"
              title={t.next}
              aria-label={t.next}
              data-testid="graph-next"
              disabled={playhead >= maxWave}
              onClick={() => seek(Math.floor(playhead + 0.0001) + 1)}
            >
              <ArrowRight size={14} />
            </button>
            <input
              type="range"
              min="0"
              max={maxWave || 1}
              step="1"
              value={currentWave}
              disabled={!maxWave}
              aria-label={t.scrub}
              data-testid="graph-scrub"
              onChange={(event) => seek(Number(event.target.value))}
            />
            <span className="graph-step-count">
              {currentWave}
              <span> / {maxWave}</span>
            </span>
            {impact ? (
              <button
                type="button"
                title={t.skip}
                aria-label={t.skip}
                data-testid="graph-skip"
                onClick={() => {
                  seek(maxWave);
                  complete();
                }}
              >
                <SkipForward size={14} />
              </button>
            ) : (
              <span className="graph-playback-done" title={t.complete}>
                <Focus size={13} />
              </span>
            )}
          </div>
        </>
      ) : null}
    </div>
  );
}
