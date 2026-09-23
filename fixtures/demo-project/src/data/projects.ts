import { readLocal } from '../core/http';

export interface Project { id: string; name: string; complete: boolean }

export function listProjects(): Project[] {
  return readLocal<Project>('projects').items;
}
