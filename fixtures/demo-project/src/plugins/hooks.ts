import { hasPlugin } from './registry';

/** Deliberate, safe-at-initialization cycle for the dependency demo. */
export function describeHook(name: string): string {
  return hasPlugin(name) ? `${name}: ready` : `${name}: unavailable`;
}
