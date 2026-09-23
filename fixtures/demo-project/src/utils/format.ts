export function bulletList(items: string[]): string {
  return items.map((item) => `• ${item}`).join('\n');
}
