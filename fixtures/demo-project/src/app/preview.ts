import { appearanceCard } from '../ui/appearance';

/** The standalone theme preview does not need workspace data or authentication. */
export function openThemePreview() {
  return { title: 'Appearance', content: appearanceCard() };
}
