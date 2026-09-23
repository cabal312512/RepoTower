import { workspaceView } from '../views/workspace';
import { settingsView } from '../views/settings';

export function route(path: string) {
  return path === '/settings' ? settingsView() : workspaceView();
}
