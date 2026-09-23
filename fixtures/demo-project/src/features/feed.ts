import { listActivity } from '../data/activity';

export function activityFeed(): string[] {
  return listActivity().map((event) => event.message);
}
