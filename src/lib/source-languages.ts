import type { RepositoryAnalysis } from '../types/analysis';

export const supportedLanguages = 'Java · Python · C/C++ · Go · Rust · C# · JS/TS';
const groups = [
  ['Java', ['java']],
  ['Python', ['py', 'pyi']],
  ['C/C++', ['c', 'h', 'cc', 'cpp', 'cxx', 'hpp', 'hh', 'hxx', 'inl', 'ipp']],
  ['Go', ['go']],
  ['Rust', ['rs']],
  ['C#', ['cs']],
  ['JS/TS', ['js', 'jsx', 'mjs', 'cjs', 'ts', 'tsx', 'mts', 'cts']],
] as const;

export function detectedLanguages(analysis: RepositoryAnalysis): string {
  const extensions = new Set(
    analysis.modules.map((module) => module.extension.replace(/^\./, '').toLowerCase()),
  );
  return groups
    .filter(([, values]) => values.some((ext) => extensions.has(ext)))
    .map(([name]) => name)
    .join(' · ');
}
