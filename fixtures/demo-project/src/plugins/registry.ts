import { describeHook } from './hooks';

const plugins = ['clock', 'notes'];

export function hasPlugin(name: string): boolean {
  return plugins.includes(name);
}

export function pluginSummary(): string[] {
  return plugins.map(describeHook);
}
