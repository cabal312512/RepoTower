import { config } from './config';

const localRecords: Record<string, unknown[]> = {
  projects: [{ id: 'p1', name: 'Build a little tool', complete: false }],
  activity: [{ id: 'a1', message: 'Workspace created' }],
};

/** An offline transport with the same shape as a small API client. */
export function readLocal<T>(resource: string): { url: string; items: T[] } {
  return { url: `${config.apiBase}/${resource}`, items: (localRecords[resource] ?? []) as T[] };
}
