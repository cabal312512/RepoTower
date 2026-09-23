import { beforeEach, describe, expect, it, vi } from 'vitest';

beforeEach(() => {
  localStorage.clear();
  vi.resetModules();
});

describe('portable preferences', () => {
  it('starts a new profile in English and light mode', async () => {
    const { usePreferences } = await import('./preferences');
    expect(usePreferences.getState()).toMatchObject({ language: 'en', theme: 'light' });
  });

  it('preserves a previous choice and persists subsequent changes', async () => {
    localStorage.setItem(
      'repotower-preferences',
      JSON.stringify({ theme: 'dark', language: 'ja' }),
    );
    const { usePreferences } = await import('./preferences');
    expect(usePreferences.getState()).toMatchObject({ language: 'ja', theme: 'dark' });
    usePreferences.getState().setLanguage('zh');
    usePreferences.getState().setTheme('light');
    expect(JSON.parse(localStorage.getItem('repotower-preferences')!)).toEqual({
      language: 'zh',
      theme: 'light',
    });
  });

  it.each(['{broken', 'null', '{"theme":"unknown","language":"unknown"}'])(
    'recovers from invalid saved data: %s',
    async (saved) => {
      localStorage.setItem('repotower-preferences', saved);
      const { usePreferences } = await import('./preferences');
      expect(usePreferences.getState()).toMatchObject({ language: 'en', theme: 'light' });
    },
  );
});
