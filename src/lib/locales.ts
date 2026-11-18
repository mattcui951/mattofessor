/**
 * The languages Mattofessor speaks. To add one: an entry here, the same entry in
 * `src-tauri/src/lang.rs`, and its dictionary `locales/<id>.ts` (a `Dict`: every key of
 * `locales/en.ts`) added to `dicts` in `i18n.svelte.ts`. Game names come from Riot's data in that
 * language by themselves.
 */
export type LangId = 'en' | 'zh' | 'zh-TW' | 'ja' | 'ko' | 'es' | 'fr';

export type Locale = {
  id: LangId;
  /** The language's own name, for the language picker. */
  name: string;
  /** BCP 47 tag: <html lang>, sorting, dates and numbers. */
  tag: string;
  /** Chinese, Japanese and Korean: no spaces between words, full-width punctuation. */
  cjk: boolean;
  /** Riot's site locale, for links to official pages. */
  riot: string;
  /** Windows fonts that draw its Chinese, Japanese or Korean (also Chinese champion names elsewhere). */
  cjkFont: string;
};

export const LOCALES: Locale[] = [
  { id: 'en', name: 'English', tag: 'en-US', cjk: false, riot: 'en-us', cjkFont: "'Microsoft YaHei UI'" },
  { id: 'zh', name: '简体中文', tag: 'zh-CN', cjk: true, riot: 'zh-cn', cjkFont: "'Microsoft YaHei UI'" },
  { id: 'zh-TW', name: '繁體中文', tag: 'zh-TW', cjk: true, riot: 'zh-tw', cjkFont: "'Microsoft JhengHei UI'" },
  { id: 'ja', name: '日本語', tag: 'ja-JP', cjk: true, riot: 'ja-jp', cjkFont: "'Yu Gothic UI', 'Meiryo UI'" },
  { id: 'ko', name: '한국어', tag: 'ko-KR', cjk: true, riot: 'ko-kr', cjkFont: "'Malgun Gothic'" },
  { id: 'es', name: 'Español', tag: 'es-ES', cjk: false, riot: 'es-es', cjkFont: "'Microsoft YaHei UI'" },
  { id: 'fr', name: 'Français', tag: 'fr-FR', cjk: false, riot: 'fr-fr', cjkFont: "'Microsoft YaHei UI'" },
];

export function isLang(id: string | null | undefined): id is LangId {
  return LOCALES.some((l) => l.id === id);
}

/** A language's entry (English when unknown). */
export function locale(id: string): Locale {
  return LOCALES.find((l) => l.id === id) ?? LOCALES[0];
}

/**
 * A text the game data has in several languages (`{ en, zh, ja... }`), in this one: its own, or
 * English when the data has none for it. `strict` gives "" instead of English (no mixed text).
 */
export function pick(text: Partial<Record<LangId, string>> | undefined, lang: LangId, strict = false): string {
  if (!text) return '';
  return text[lang] ?? (strict && lang !== 'en' ? '' : (text.en ?? ''));
}
