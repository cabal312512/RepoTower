import { AlertCircle, CornerDownRight, FileCode2, RotateCcw, Undo2, Unplug, X } from 'lucide-react';
import type { ImpactResult, ModuleAnalysis, RepositoryAnalysis } from '../types/analysis';
import { workbenchCopy } from '../lib/workbench-copy';
import { translateAnalysisMessage } from '../lib/analysis-messages';
import { usePreferences } from '../state/preferences';
import { useEffect, useRef, useState } from 'react';

interface Props {
  analysis: RepositoryAnalysis;
  selected: ModuleAnalysis | undefined;
  preview: ImpactResult | null;
  impact: ImpactResult | null;
  report: ImpactResult | null;
  removedIds: string[];
  disconnectedIds: string[];
  busy: boolean;
  canUndo: boolean;
  onPull: () => void;
  onUndo: () => void;
  onReset: () => void;
  onRestore: (id: string) => void;
  onFinish: () => void;
  onSelect: (id: string) => void;
}
export default function SelectionDock(p: Props) {
  const language = usePreferences((s) => s.language);
  const c = workbenchCopy[language];
  const [relations, setRelations] = useState<'imports' | 'importedBy' | 'unresolved' | null>(null);
  const relationRef = useRef<HTMLDivElement>(null);
  useEffect(() => setRelations(null), [p.selected?.id]);
  useEffect(() => {
    const close = (event: PointerEvent) => {
      if (!relationRef.current?.contains(event.target as Node)) setRelations(null);
    };
    window.addEventListener('pointerdown', close);
    return () => window.removeEventListener('pointerdown', close);
  }, []);
  const active = p.impact ?? p.report;
  const isDisconnected = !!p.selected && p.disconnectedIds.includes(p.selected.id);
  const isUnavailable = !!p.selected && p.removedIds.includes(p.selected.id);
  const file =
    p.selected ??
    (active ? p.analysis.modules.find((m) => m.id === active.removedModuleId) : undefined);
  return (
    <div
      className={`selection-dock ${active && !p.selected ? 'has-result' : ''}`}
      data-testid="selection-dock"
    >
      <div className="dock-file">
        <span
          className={`dock-file-icon ${isDisconnected || p.impact || (active && !p.selected) ? 'cut' : isUnavailable ? 'pink' : ''}`}
        >
          <FileCode2 size={18} />
        </span>
        <div>
          {file ? (
            <>
              <strong>{file.fileName}</strong>
              <code title={file.relativePath}>{file.relativePath}</code>
            </>
          ) : (
            <>
              <strong>{c.select}</strong>
              <small>{c.hint}</small>
            </>
          )}
        </div>
      </div>
      {p.selected && (!p.impact || p.selected.id !== p.impact.removedModuleId) ? (
        <>
          <div className="dock-relations" ref={relationRef}>
            <button onClick={() => setRelations(relations === 'imports' ? null : 'imports')}>
              {p.selected.fanOut}
              <span>{c.imports}</span>
            </button>
            <button onClick={() => setRelations(relations === 'importedBy' ? null : 'importedBy')}>
              {p.selected.fanIn}
              <span>{c.callers}</span>
            </button>
            {p.selected.unresolvedImports.length > 0 && (
              <button
                className="relation-warning"
                data-testid="unresolved-toggle"
                title={c.unresolved}
                aria-label={c.unresolved}
                onClick={() => setRelations(relations === 'unresolved' ? null : 'unresolved')}
              >
                <span className="unresolved-count">
                  <AlertCircle size={13} />
                  {p.selected.unresolvedImports.length}
                </span>
                <span>{c.unresolved}</span>
              </button>
            )}
            {relations && (
              <div className="relation-popover">
                <div className="menu-caption">
                  {relations === 'unresolved'
                    ? c.unresolved
                    : relations === 'imports'
                      ? c.dependencies
                      : c.dependents}
                  <button className="tool-button" onClick={() => setRelations(null)}>
                    <X size={12} />
                  </button>
                </div>
                {relations === 'unresolved' ? (
                  p.selected.unresolvedImports.map((item, index) => (
                    <div className="unresolved-item" key={`${item.specifier}-${index}`}>
                      <code>{item.specifier}</code>
                      <p>{translateAnalysisMessage(item.reason, language)}</p>
                    </div>
                  ))
                ) : p.selected[relations].length ? (
                  p.selected[relations].slice(0, 50).map((id) => (
                    <button
                      key={id}
                      title={id}
                      onClick={() => {
                        p.onSelect(id);
                        setRelations(null);
                      }}
                    >
                      <CornerDownRight size={12} />
                      {p.analysis.modules.find((m) => m.id === id)?.fileName ?? id}
                    </button>
                  ))
                ) : (
                  <p>—</p>
                )}
              </div>
            )}
          </div>
          {isUnavailable ? (
            <div className="dock-impact">
              <small>{isDisconnected ? c.target : c.affected}</small>
            </div>
          ) : (
            <div className="dock-impact">
              <small>{c.potential}</small>
              <strong data-testid="potential-count">{p.preview?.totalAffected ?? '—'}</strong>
            </div>
          )}
          {isDisconnected ? (
            <button
              className="button primary"
              data-testid="restore-file"
              disabled={p.busy}
              onClick={() => p.onRestore(p.selected!.id)}
            >
              <RotateCcw size={15} />
              {c.restoreFile}
            </button>
          ) : isUnavailable ? (
            <span className="dock-unavailable">{c.restoreOrigin}</span>
          ) : (
            <button
              className="button primary disconnect"
              data-testid="pull-button"
              disabled={!p.preview || p.busy || !!p.impact}
              onClick={p.onPull}
            >
              <Unplug size={15} />
              {c.disconnect}
            </button>
          )}
        </>
      ) : active ? (
        <>
          <div
            className="impact-summary"
            data-testid={p.impact ? 'impact-progress' : 'impact-report'}
          >
            <span className="impact-count cut">
              <b>1</b>
              {c.target}
            </span>
            <span className="impact-count pink">
              <b>{active.totalAffected}</b>
              {c.affected}
            </span>
            <span className="impact-count mint">
              <b>
                {p.analysis.totalModules -
                  p.removedIds.length -
                  (p.impact ? active.totalAffected + 1 : 0)}
              </b>
              {c.intact}
            </span>
          </div>
          {p.impact ? (
            <>
              <button className="button subtle" data-testid="finish-impact" onClick={p.onFinish}>
                {c.finish}
              </button>
              {p.selected?.id === p.impact.removedModuleId && (
                <button
                  className="tool-button"
                  data-testid="restore-file"
                  title={c.restoreFile}
                  aria-label={c.restoreFile}
                  disabled={p.busy}
                  onClick={() => p.onRestore(p.impact!.removedModuleId)}
                >
                  <RotateCcw size={15} />
                </button>
              )}
            </>
          ) : (
            <button
              className="button primary"
              data-testid="undo-report"
              onClick={p.onUndo}
              disabled={!p.canUndo || p.busy}
            >
              <Undo2 size={15} />
              {c.undo}
            </button>
          )}
        </>
      ) : (
        <div className="dock-empty-space" />
      )}
      <div className="dock-history">
        <button
          className="tool-button"
          data-testid="undo-toolbar"
          title={c.undo}
          aria-label={c.undo}
          onClick={p.onUndo}
          disabled={!p.canUndo || !!p.impact || p.busy}
        >
          <Undo2 size={15} />
        </button>
        <button
          className="tool-button"
          data-testid="reset-toolbar"
          title={c.reset}
          aria-label={c.reset}
          onClick={p.onReset}
          disabled={!p.removedIds.length || !!p.impact || p.busy}
        >
          <RotateCcw size={14} />
        </button>
      </div>
    </div>
  );
}
