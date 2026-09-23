export type DependencyKind =
  'static' | 'export' | 'dynamic' | 'require' | 'include' | 'module' | 'use';
export interface UnresolvedImport {
  specifier: string;
  kind: DependencyKind;
  reason: string;
}
export interface ModuleAnalysis {
  id: string;
  relativePath: string;
  fileName: string;
  extension: string;
  directory: string;
  linesOfCode: number;
  imports: string[];
  importedBy: string[];
  fanIn: number;
  fanOut: number;
  dependencyDepth: number;
  blastRadius: number;
  blastRatio: number;
  cycleId: number | null;
  isEntryLike: boolean;
  unresolvedImports: UnresolvedImport[];
}
export interface DependencyEdge {
  source: string;
  target: string;
  kind: DependencyKind;
}
export interface StabilityBreakdown {
  averageBlastRatio: number;
  maxBlastRatio: number;
  cycleRatio: number;
  concentration: number;
}
export interface RepositoryAnalysis {
  repositoryName: string;
  rootDisplayName: string;
  totalModules: number;
  totalEdges: number;
  unresolvedCount: number;
  externalCount: number;
  cycleCount: number;
  stabilityScore: number;
  stabilityBreakdown: StabilityBreakdown;
  modules: ModuleAnalysis[];
  edges: DependencyEdge[];
  criticalModules: string[];
  warnings: string[];
}
export interface ImpactResult {
  removedModuleId: string;
  totalAffected: number;
  affectedRatio: number;
  directAffected: number;
  transitiveAffected: number;
  maxCascadeDepth: number;
  waves: string[][];
}
export interface ScanProgress {
  stage: string;
  completed?: number;
  total?: number;
}
export interface DesktopApi {
  minimizeWindow(): Promise<void>;
  toggleMaximizeWindow(): Promise<{ maximized: boolean }>;
  closeWindow(): Promise<void>;
  getWindowState(): Promise<{ maximized: boolean }>;
  onWindowState(callback: (state: { maximized: boolean }) => void): () => void;
  openRepository(): Promise<RepositoryAnalysis | null>;
  analyzePath(path: string): Promise<RepositoryAnalysis>;
  loadDemo(): Promise<RepositoryAnalysis>;
  impact(moduleId: string, excluded: string[]): Promise<ImpactResult>;
  onProgress(callback: (progress: ScanProgress) => void): () => void;
  onOpenPath(callback: (path: string) => void): () => void;
  getDroppedPath(file: File): string;
}
declare global {
  interface Window {
    repoTower?: DesktopApi;
  }
}
