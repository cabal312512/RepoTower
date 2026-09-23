import { currentProfile } from '../data/profile';
import { currentSession } from '../data/session';

export function accountCard() {
  return { title: currentProfile().displayName, status: currentSession().mode };
}
