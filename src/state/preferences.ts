import { create } from 'zustand';

export type Language = 'zh' | 'en' | 'ja';
export type Theme = 'dark' | 'light';
interface Preferences {
  theme: Theme;
  language: Language;
  setTheme: (theme: Theme) => void;
  setLanguage: (language: Language) => void;
}
const key = 'repotower-preferences';
function read(): { theme: Theme; language: Language } {
  try {
    const saved = JSON.parse(localStorage.getItem(key) ?? '{}');
    return {
      theme: saved.theme === 'dark' ? 'dark' : 'light',
      language: ['zh', 'en', 'ja'].includes(saved.language) ? saved.language : 'en',
    };
  } catch {
    return { theme: 'light', language: 'en' };
  }
}
function save(value: { theme: Theme; language: Language }) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    /* Preferences are optional when storage is unavailable. */
  }
}
export const usePreferences = create<Preferences>((set, get) => ({
  ...read(),
  setTheme: (theme) => {
    set({ theme });
    save({ theme, language: get().language });
  },
  setLanguage: (language) => {
    set({ language });
    save({ language, theme: get().theme });
  },
}));
