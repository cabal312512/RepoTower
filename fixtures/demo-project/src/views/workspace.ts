import { projectBoard } from '../features/board';
import { activityFeed } from '../features/feed';

export function workspaceView() {
  return { title: 'Workspace', board: projectBoard(), feed: activityFeed() };
}
