import { listProjects } from '../data/projects';

export function projectBoard() {
  return listProjects().map((project) => ({ title: project.name, checked: project.complete }));
}
