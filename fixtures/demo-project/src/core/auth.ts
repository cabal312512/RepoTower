import { config } from './config';

export function signedInUser() {
  return { id: config.currentUserId, workspace: config.workspaceName };
}
