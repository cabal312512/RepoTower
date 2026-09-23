import { useCallback, useEffect, useRef, useState } from 'react';
import { AlertCircle, ChevronRight, FolderOpen, LoaderCircle, X } from 'lucide-react';
import { useTowerStore } from './state/store';
import { usePreferences } from './state/preferences';
import { workbenchCopy } from './lib/workbench-copy';
import { translateAnalysisMessage } from './lib/analysis-messages';
import { detectedLanguages, supportedLanguages } from './lib/source-languages';
import { Titlebar, Projectbar } from './components/Chrome';
import SelectionDock from './components/SelectionDock';
import GraphScene from './graph/GraphScene';
import type { ImpactResult, ScanProgress } from './types/analysis';

function Welcome({ onOpen, onDemo }: { onOpen: () => void; onDemo: () => void }) {
  const c = workbenchCopy[usePreferences((s) => s.language)];
  return (
    <div className="welcome" data-testid="welcome">
      <div className="welcome-circuit" aria-hidden="true">
        <svg viewBox="0 0 470 155">
          <defs>
            <linearGradient id="wire" x1="0" x2="1">
              <stop stopColor="#63e7bd" />
              <stop offset="1" stopColor="#8495ff" />
            </linearGradient>
          </defs>
          <path
            d="M117 76H171Q185 76 185 62V41Q185 28 201 28H293M185 76V116Q185 130 201 130H293"
            fill="none"
            stroke="url(#wire)"
            strokeWidth="1.5"
          />
          <g className="welcome-node">
            <rect x="13" y="53" width="105" height="46" rx="10" />
            <circle cx="30" cy="76" r="3" fill="#63e7bd" />
            <text x="43" y="80">
              config.ts
            </text>
          </g>
          <g className="welcome-node">
            <rect x="293" y="5" width="122" height="46" rx="10" />
            <circle cx="310" cy="28" r="3" fill="#819cff" />
            <text x="323" y="32">
              http.ts
            </text>
          </g>
          <g className="welcome-node">
            <rect x="293" y="107" width="122" height="46" rx="10" />
            <circle cx="310" cy="130" r="3" fill="#c494ff" />
            <text x="323" y="134">
              auth.ts
            </text>
          </g>
          <circle cx="185" cy="76" r="4" fill="#91adca" className="circuit-junction" />
        </svg>
      </div>
      <h1>{c.landing}</h1>
      <p>{supportedLanguages}</p>
      <div className="welcome-actions">
        <button className="button primary" data-testid="open-folder" onClick={onOpen}>
          <FolderOpen size={15} />
          {c.choose}
        </button>
        <button className="button quiet" data-testid="open-demo" onClick={onDemo}>
          {c.demo}
          <ChevronRight size={13} />
        </button>
      </div>
      <small>{c.drop}</small>
    </div>
  );
}

export default function App() {
  const s = useTowerStore();
  const { theme, language } = usePreferences();
  const c = workbenchCopy[language];
  const [loading, setLoading] = useState(false);
  const [progress, setProgress] = useState<ScanProgress>({ stage: 'scan' });
  const [error, setError] = useState<string | null>(null);
  const [dragging, setDragging] = useState(false);
  const [notes, setNotes] = useState(false);
  const [preview, setPreview] = useState<ImpactResult | null>(null);
  const [impactBusy, setImpactBusy] = useState(false);
  const [inspectedId, setInspectedId] = useState<string | null>(null);
  const [inspectToken, setInspectToken] = useState(0);
  const [reducedMotion, setReducedMotion] = useState(
    () => window.matchMedia('(prefers-reduced-motion: reduce)').matches,
  );
  const requestLock = useRef(false);
  const impactLock = useRef(false);
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    document.documentElement.lang = language === 'zh' ? 'zh-CN' : language;
  }, [theme, language]);
  useEffect(() => {
    const media = window.matchMedia('(prefers-reduced-motion: reduce)');
    const update = () => setReducedMotion(media.matches);
    media.addEventListener('change', update);
    return () => media.removeEventListener('change', update);
  }, []);
  const scan = useCallback(async (kind: 'open' | 'demo' | 'path', path?: string) => {
    if (requestLock.current || useTowerStore.getState().impact) return;
    if (!window.repoTower) {
      setError('desktop-unavailable');
      return;
    }
    requestLock.current = true;
    setLoading(true);
    setError(null);
    setProgress({ stage: 'scan' });
    try {
      const result =
        kind === 'demo'
          ? await window.repoTower.loadDemo()
          : kind === 'path'
            ? await window.repoTower.analyzePath(path!)
            : await window.repoTower.openRepository();
      if (result) {
        useTowerStore.getState().setAnalysis(result);
        setInspectedId(null);
        setNotes(false);
      }
    } catch (e) {
      setError(String(e).replace(/^Error: /, ''));
    } finally {
      setLoading(false);
      requestLock.current = false;
    }
  }, []);
  useEffect(() => {
    const a = window.repoTower?.onProgress(setProgress);
    const b = window.repoTower?.onOpenPath((path) => {
      void scan('path', path);
    });
    return () => {
      a?.();
      b?.();
    };
  }, [scan]);
  useEffect(() => {
    let active = true;
    setPreview(null);
    if (s.selectedId && !s.removedIds.includes(s.selectedId) && !s.impact && window.repoTower)
      window.repoTower
        .impact(s.selectedId, s.removedIds)
        .then((result) => {
          if (active) setPreview(result);
        })
        .catch((e) => {
          if (active) setError(String(e));
        });
    return () => {
      active = false;
    };
  }, [s.selectedId, s.removedIds, s.impact, s.analysis]);
  const select = (id: string | null) => {
    setInspectedId(id);
    setInspectToken((value) => value + 1);
    s.select(id);
  };
  const pull = async () => {
    const before = useTowerStore.getState();
    if (!before.selectedId || !window.repoTower || impactLock.current) return;
    impactLock.current = true;
    setImpactBusy(true);
    setInspectedId(null);
    try {
      const result = await window.repoTower.impact(before.selectedId, before.removedIds);
      const current = useTowerStore.getState();
      if (
        current.analysis === before.analysis &&
        current.selectedId === before.selectedId &&
        current.removedIds === before.removedIds
      )
        current.beginPull(result);
    } catch (e) {
      setError(String(e));
    } finally {
      impactLock.current = false;
      setImpactBusy(false);
    }
  };
  const undo = () => {
    setInspectedId(null);
    s.undo();
  };
  const restore = async (id: string) => {
    if (!window.repoTower || impactLock.current) return;
    // Completing a live cut makes its origin immediately restorable too.
    useTowerStore.getState().finishPull();
    const before = useTowerStore.getState();
    if (!before.disconnectedIds.includes(id)) return;
    impactLock.current = true;
    setImpactBusy(true);
    try {
      const reports: ImpactResult[] = [];
      let unavailable: string[] = [];
      for (const origin of before.disconnectedIds.filter((value) => value !== id)) {
        const result = await window.repoTower.impact(
          origin,
          unavailable.filter((value) => value !== origin),
        );
        reports.push(result);
        unavailable = [...new Set([...unavailable, ...result.waves.flat()])];
      }
      const current = useTowerStore.getState();
      if (
        current.analysis === before.analysis &&
        current.removedIds === before.removedIds &&
        !current.impact
      )
        current.restore(id, reports);
    } catch (e) {
      setError(String(e));
    } finally {
      impactLock.current = false;
      setImpactBusy(false);
    }
  };
  const reset = () => {
    setInspectedId(null);
    s.reset();
  };
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if ((e.target as HTMLElement)?.matches('input,textarea,select')) return;
      if (e.key === 'Escape') {
        setInspectedId(null);
        setNotes(false);
        useTowerStore.getState().select(null);
      }
      if ((e.ctrlKey || e.metaKey) && e.key === 'z') {
        e.preventDefault();
        setInspectedId(null);
        useTowerStore.getState().undo();
      }
      if (e.key.toLowerCase() === 'r') useTowerStore.getState().resetCamera();
    };
    window.addEventListener('keydown', key);
    return () => window.removeEventListener('keydown', key);
  }, []);
  const selected = s.analysis?.modules.find((m) => m.id === s.selectedId);
  const stage =
    (
      {
        scan: c.scan,
        parse: c.parse,
        graph: c.graph,
        metrics: c.metrics,
        complete: c.complete,
      } as Record<string, string>
    )[progress.stage] ?? c.scan;
  return (
    <div
      className="app instrument"
      data-theme={theme}
      onDragOver={(e) => {
        e.preventDefault();
        if (!loading && !s.impact) setDragging(true);
      }}
      onDragLeave={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget as Node)) setDragging(false);
      }}
      onDrop={(e) => {
        e.preventDefault();
        setDragging(false);
        const file = e.dataTransfer.files[0];
        const path = file && window.repoTower?.getDroppedPath(file);
        if (path) void scan('path', path);
      }}
    >
      <Titlebar onOpen={() => void scan('open')} disabled={loading || !!s.impact} />
      {s.analysis ? (
        <>
          <Projectbar
            analysis={s.analysis}
            selectedId={s.selectedId}
            removedIds={s.removedIds}
            onSelect={select}
            onOpen={() => void scan('open')}
            onHome={() => {
              s.close();
              setInspectedId(null);
            }}
            disabled={loading || !!s.impact}
            selectionDisabled={loading}
          />
          <main className="graph-workspace" data-testid="workspace">
            {s.analysis.totalModules ? (
              <GraphScene
                analysis={s.analysis}
                selectedId={s.selectedId}
                inspectId={inspectedId}
                inspectToken={inspectToken}
                removedIds={s.removedIds}
                disconnectedIds={s.disconnectedIds}
                impact={s.impact}
                report={s.lastImpact}
                reports={s.reports}
                onReviewImpact={s.review}
                animationStartedAt={s.animationStartedAt}
                onSelect={(id) => {
                  setInspectedId(null);
                  s.select(id);
                }}
                onAnimationComplete={s.finishPull}
                resetToken={s.resetCameraToken}
                theme={theme}
                language={language}
                reducedMotion={reducedMotion}
              />
            ) : (
              <div className="empty-project">
                <FolderOpen size={28} />
                <strong>{c.emptyTitle}</strong>
                <small>{supportedLanguages}</small>
                <button className="button quiet" onClick={() => void scan('open')}>
                  {c.choose}
                </button>
              </div>
            )}
          </main>
          <SelectionDock
            analysis={s.analysis}
            selected={selected}
            preview={preview}
            impact={s.impact}
            report={s.lastImpact}
            removedIds={s.removedIds}
            disconnectedIds={s.disconnectedIds}
            busy={impactBusy}
            canUndo={!!s.history.length}
            onPull={pull}
            onUndo={undo}
            onReset={reset}
            onRestore={(id) => void restore(id)}
            onFinish={s.finishPull}
            onSelect={select}
          />
        </>
      ) : (
        <Welcome onOpen={() => void scan('open')} onDemo={() => void scan('demo')} />
      )}
      <footer className="statusline">
        <span className="offline-indicator">
          <i />
          {c.offline}
        </span>
        <span className="read-only-note" title={c.note}>
          {c.readOnly}
        </span>
        {!!s.analysis?.warnings.length && (
          <button className={notes ? 'active' : ''} onClick={() => setNotes(!notes)}>
            <AlertCircle size={11} />
            {s.analysis.warnings.length}
          </button>
        )}
        <span className="status-spacer" />
        <span>{s.analysis ? detectedLanguages(s.analysis) : supportedLanguages}</span>
      </footer>
      {notes && s.analysis && (
        <div className="notes-popover">
          <div className="menu-caption">
            {c.warnings}
            <button className="tool-button" onClick={() => setNotes(false)}>
              <X size={13} />
            </button>
          </div>
          {s.analysis.warnings.map((w, i) => (
            <p key={i}>{translateAnalysisMessage(w, language)}</p>
          ))}
        </div>
      )}
      {error && (
        <div className="error-toast" role="alert">
          <AlertCircle size={16} />
          <p>
            {error === 'desktop-unavailable'
              ? c.desktop
              : translateAnalysisMessage(error, language)}
          </p>
          <button className="tool-button" data-testid="close-error" onClick={() => setError(null)}>
            <X size={14} />
          </button>
        </div>
      )}
      {loading && (
        <div className="loading-overlay">
          <div>
            <LoaderCircle className="spin" size={19} />
            <span>{stage}</span>
            {progress.total ? (
              <small>
                {progress.completed ?? 0} / {progress.total}
              </small>
            ) : null}
          </div>
        </div>
      )}
      {dragging && (
        <div className="drop-overlay">
          <FolderOpen size={30} />
          <strong>{c.dropNow}</strong>
        </div>
      )}
    </div>
  );
}
