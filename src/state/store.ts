import { create } from 'zustand';
import type { ImpactResult, RepositoryAnalysis } from '../types/analysis';

interface Snapshot {
  removedIds: string[];
  disconnectedIds: string[];
  reports: ImpactResult[];
  lastImpact: ImpactResult | null;
  selectedId: string | null;
}
interface TowerState extends Snapshot {
  analysis: RepositoryAnalysis | null;
  impact: ImpactResult | null;
  animationStartedAt: number | null;
  history: Snapshot[];
  resetCameraToken: number;
  setAnalysis: (analysis: RepositoryAnalysis) => void;
  select: (id: string | null) => void;
  beginPull: (impact: ImpactResult) => void;
  finishPull: () => void;
  restore: (id: string, reports: ImpactResult[]) => void;
  review: (id: string) => void;
  undo: () => void;
  reset: () => void;
  resetCamera: () => void;
  close: () => void;
}
const clean = {
  removedIds: [] as string[],
  disconnectedIds: [] as string[],
  reports: [] as ImpactResult[],
  lastImpact: null,
  impact: null,
  animationStartedAt: null,
  history: [] as Snapshot[],
  selectedId: null,
};
export const useTowerStore = create<TowerState>((set, get) => ({
  ...clean,
  analysis: null,
  resetCameraToken: 0,
  setAnalysis: (analysis) =>
    set({ ...clean, analysis, resetCameraToken: get().resetCameraToken + 1 }),
  select: (id) => {
    const s = get();
    if (!id || s.analysis?.modules.some((m) => m.id === id)) set({ selectedId: id });
  },
  beginPull: (impact) => {
    const s = get();
    if (
      !s.analysis ||
      s.impact ||
      s.removedIds.includes(impact.removedModuleId) ||
      !s.analysis.modules.some((m) => m.id === impact.removedModuleId)
    )
      return;
    const snapshot: Snapshot = {
      removedIds: [...s.removedIds],
      disconnectedIds: s.disconnectedIds,
      reports: s.reports,
      lastImpact: s.lastImpact,
      selectedId: impact.removedModuleId,
    };
    set({
      impact,
      animationStartedAt: performance.now(),
      selectedId: impact.removedModuleId,
      history: [...s.history, snapshot],
      lastImpact: null,
    });
  },
  finishPull: () => {
    const s = get();
    if (!s.impact) return;
    const next = [...new Set([...s.removedIds, ...s.impact.waves.flat()])];
    set({
      removedIds: next,
      disconnectedIds: [...s.disconnectedIds, s.impact.removedModuleId],
      reports: [...s.reports, s.impact],
      lastImpact: s.impact,
      impact: null,
      animationStartedAt: null,
      selectedId: s.selectedId === s.impact.removedModuleId ? null : s.selectedId,
    });
  },
  restore: (id, reports) => {
    const s = get();
    const remaining = s.disconnectedIds.filter((origin) => origin !== id);
    if (
      s.impact ||
      !s.disconnectedIds.includes(id) ||
      reports.length !== remaining.length ||
      reports.some((report, index) => report.removedModuleId !== remaining[index])
    )
      return;
    const snapshot: Snapshot = {
      removedIds: s.removedIds,
      disconnectedIds: s.disconnectedIds,
      reports: s.reports,
      lastImpact: s.lastImpact,
      selectedId: s.selectedId,
    };
    set({
      removedIds: [...new Set(reports.flatMap((report) => report.waves.flat()))],
      disconnectedIds: remaining,
      reports,
      lastImpact:
        reports.find((report) => report.removedModuleId === s.lastImpact?.removedModuleId) ??
        reports.at(-1) ??
        null,
      history: [...s.history, snapshot],
    });
  },
  review: (id) => {
    const s = get();
    const report = s.reports.find((report) => report.removedModuleId === id);
    if (!s.impact && report) set({ lastImpact: report });
  },
  undo: () => {
    const s = get();
    const snapshot = s.history.at(-1);
    if (!snapshot || s.impact) return;
    set({ ...snapshot, history: s.history.slice(0, -1), impact: null, animationStartedAt: null });
  },
  reset: () => set({ ...clean, resetCameraToken: get().resetCameraToken + 1 }),
  resetCamera: () => set({ resetCameraToken: get().resetCameraToken + 1 }),
  close: () => set({ ...clean, analysis: null }),
}));
