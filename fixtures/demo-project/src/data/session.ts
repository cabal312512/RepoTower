import { signedInUser } from '../core/auth';

export function currentSession() {
  return { userId: signedInUser().id, mode: 'offline' as const };
}
