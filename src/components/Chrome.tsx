import { useEffect, useRef, useState } from 'react';
import { FolderOpen, Maximize2, Minus, Moon, Search, Settings2, Sun, X, Zap } from 'lucide-react';
import { usePreferences, type Language } from '../state/preferences';
import { workbenchCopy } from '../lib/workbench-copy';
import type { RepositoryAnalysis } from '../types/analysis';

export function CircuitLogo() {
  return (
    <svg viewBox="0 0 26 26" width="23" height="23" fill="none" aria-hidden="true">
      <path d="M6 7H14V19H21" stroke="currentColor" strokeWidth="1.5" />
      <rect x="2" y="3" width="8" height="8" rx="2.2" fill="var(--mint)" />
      <rect x="10" y="15" width="8" height="8" rx="2.2" fill="var(--blue)" />
      <rect x="18" y="3" width="6" height="6" rx="1.8" fill="var(--pink)" />
      <path d="M21 9V19H18" stroke="currentColor" strokeWidth="1.5" />
    </svg>
  );
}

export function Titlebar({ onOpen, disabled }: { onOpen: () => void; disabled: boolean }) {
  const [settings, setSettings] = useState(false);
  const { theme, language, setTheme, setLanguage } = usePreferences();
  const c = workbenchCopy[language];
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const close = (e: PointerEvent) => {
      if (!ref.current?.contains(e.target as Node)) setSettings(false);
    };
    window.addEventListener('pointerdown', close);
    return () => window.removeEventListener('pointerdown', close);
  }, []);
  return (
    <header className="titlebar">
      <div className="app-identity">
        <CircuitLogo />
        <strong>RepoTower</strong>
        <span className="app-version">05</span>
      </div>
      <div className="titlebar-drag" />
      <div className="title-tools">
        <button
          className="tool-button"
          title={c.open}
          aria-label={c.open}
          data-testid="title-open"
          onClick={onOpen}
          disabled={disabled}
        >
          <FolderOpen size={15} />
        </button>
        <div className="settings-anchor" ref={ref}>
          <button
            className={`tool-button ${settings ? 'active' : ''}`}
            title={c.settings}
            aria-label={c.settings}
            data-testid="settings-toggle"
            onClick={() => setSettings(!settings)}
          >
            <Settings2 size={15} />
          </button>
          {settings && (
            <div className="settings-popover">
              <div className="setting-themes">
                <button
                  data-testid="theme-dark"
                  className={theme === 'dark' ? 'active' : ''}
                  onClick={() => setTheme('dark')}
                >
                  <Moon size={14} />
                  {c.dark}
                </button>
                <button
                  data-testid="theme-light"
                  className={theme === 'light' ? 'active' : ''}
                  onClick={() => setTheme('light')}
                >
                  <Sun size={14} />
                  {c.light}
                </button>
              </div>
              <label>
                {c.language}
                <select
                  data-testid="language-select"
                  value={language}
                  onChange={(e) => setLanguage(e.target.value as Language)}
                >
                  <option value="zh">中文</option>
                  <option value="en">English</option>
                  <option value="ja">日本語</option>
                </select>
              </label>
            </div>
          )}
        </div>
      </div>
      <div className="window-controls">
        <button
          title={c.minimize}
          aria-label={c.minimize}
          onClick={() => void window.repoTower?.minimizeWindow?.()}
        >
          <Minus size={14} />
        </button>
        <button
          title={c.maximize}
          aria-label={c.maximize}
          onClick={() => void window.repoTower?.toggleMaximizeWindow?.()}
        >
          <Maximize2 size={12} />
        </button>
        <button
          className="window-close"
          title={c.close}
          aria-label={c.close}
          onClick={() => void window.repoTower?.closeWindow?.()}
        >
          <X size={15} />
        </button>
      </div>
    </header>
  );
}

export function Projectbar({
  analysis,
  selectedId,
  removedIds,
  onSelect,
  onHome,
  onOpen,
  disabled,
  selectionDisabled = false,
}: {
  analysis: RepositoryAnalysis;
  selectedId: string | null;
  removedIds: string[];
  onSelect: (id: string) => void;
  onHome: () => void;
  onOpen: () => void;
  disabled: boolean;
  selectionDisabled?: boolean;
}) {
  const c = workbenchCopy[usePreferences((s) => s.language)];
  const [query, setQuery] = useState('');
  const [menu, setMenu] = useState<'search' | 'critical' | null>(null);
  useEffect(() => {
    setQuery('');
    setMenu(null);
  }, [analysis]);
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const close = (e: PointerEvent) => {
      if (!ref.current?.contains(e.target as Node)) setMenu(null);
    };
    window.addEventListener('pointerdown', close);
    return () => window.removeEventListener('pointerdown', close);
  }, []);
  const byId = new Map(analysis.modules.map((m) => [m.id, m]));
  const files =
    menu === 'critical'
      ? analysis.criticalModules
          .map((id) => byId.get(id)!)
          .filter(Boolean)
          .slice(0, 8)
      : analysis.modules
          .filter((m) => m.relativePath.toLowerCase().includes(query.toLowerCase()))
          .slice(0, 80);
  return (
    <div className="projectbar">
      <button
        className="repo-button"
        data-testid="open-another"
        title={analysis.rootDisplayName}
        onClick={onOpen}
        disabled={disabled}
      >
        <FolderOpen size={14} />
        <strong>{analysis.repositoryName}</strong>
      </button>
      <span className="repo-count">
        {analysis.totalModules} {c.files}
        <i /> {analysis.totalEdges} {c.links}
      </span>
      <div className="project-actions" ref={ref}>
        <button
          className={`tool-button ${menu === 'critical' ? 'active' : ''}`}
          data-testid="critical-toggle"
          title={c.critical}
          aria-label={c.critical}
          disabled={selectionDisabled}
          onClick={() => setMenu(menu === 'critical' ? null : 'critical')}
        >
          <Zap size={14} />
        </button>
        <label className="file-search">
          <Search size={13} />
          <input
            data-testid="module-search"
            value={query}
            disabled={selectionDisabled}
            placeholder={c.find}
            aria-label={c.find}
            onFocus={() => setMenu('search')}
            onChange={(e) => {
              setQuery(e.target.value);
              setMenu('search');
            }}
            onKeyDown={(e) => {
              if (e.key === 'Escape') setMenu(null);
              if (e.key === 'Enter' && files[0]) {
                onSelect(files[0].id);
                setMenu(null);
              }
            }}
          />
        </label>
        {menu && (
          <div className="file-menu">
            <div className="menu-caption">
              {menu === 'critical' ? c.critical : c.all}
              <span>{files.length}</span>
            </div>
            {files.length ? (
              files.map((m) => (
                <button
                  key={m.id}
                  data-testid="module-list-item"
                  data-id={m.id}
                  className={selectedId === m.id ? 'selected' : ''}
                  onClick={() => {
                    onSelect(m.id);
                    setMenu(null);
                  }}
                  disabled={selectionDisabled}
                >
                  <span className={`file-dot ${removedIds.includes(m.id) ? 'unavailable' : ''}`} />
                  <span className="file-menu-name">
                    <strong>{m.fileName}</strong>
                    <small>{m.directory || '/'}</small>
                  </span>
                  <span className="file-impact">
                    {removedIds.includes(m.id) ? '−' : m.blastRadius}
                  </span>
                </button>
              ))
            ) : (
              <p>{c.emptySearch}</p>
            )}
          </div>
        )}
      </div>
      <button
        className="tool-button project-close"
        data-testid="home"
        title={c.home}
        aria-label={c.home}
        onClick={onHome}
        disabled={disabled}
      >
        <X size={13} />
      </button>
    </div>
  );
}
