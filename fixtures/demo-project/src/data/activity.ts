import { readLocal } from '../core/http';

export interface Activity { id: string; message: string }

export function listActivity(): Activity[] {
  return readLocal<Activity>('activity').items;
}
