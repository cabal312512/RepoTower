import { signedInUser } from '../core/auth';

export function currentProfile() {
  const user = signedInUser();
  return { id: user.id, displayName: 'Ada', workspace: user.workspace };
}
