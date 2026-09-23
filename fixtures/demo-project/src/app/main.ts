import { route } from './router';
import { theme } from '../ui/theme';

export function openWorkspace(path = '/') {
  return { theme, content: route(path) };
}
