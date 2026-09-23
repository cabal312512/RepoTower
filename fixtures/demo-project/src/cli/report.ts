import { bulletList } from '../utils/format';

export function plainTextReport(items: string[]): string {
  return `Report\n${bulletList(items)}`;
}
