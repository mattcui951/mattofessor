/**
 * UI strings: one dictionary per language in `locales/<id>.ts` (English is the source and the
 * fallback), the languages themselves in `locales.ts`. `{0}`, `{1}` are replaced by arguments.
 */
import type { LangId } from './locales';
import { en, type Dict, type Key } from './locales/en';
import { es } from './locales/es';
import { fr } from './locales/fr';
import { ja } from './locales/ja';
import { ko } from './locales/ko';
import { ru } from './locales/ru';
import { zh } from './locales/zh';
import { zhTW } from './locales/zh-TW';

export type { Key } from './locales/en';
export type Lang = LangId;

/** Every language's dictionary; a language in `locales.ts` without one is a type error. */
const dicts: Record<LangId, Dict> = { en, zh, 'zh-TW': zhTW, ja, ko, es, fr, ru };

export const i18n = $state({ lang: 'en' as Lang });

export function t(key: Key | string, ...args: (string | number)[]): string {
  const d = (dicts[i18n.lang] ?? en) as Record<string, string>;
  const s = d[key] ?? (en as Record<string, string>)[key] ?? key;
  return args.length ? s.replace(/\{(\d)\}/g, (_, n) => String(args[Number(n)] ?? '')) : s;
}

export function hasKey(key: string): boolean {
  return key in en;
}

/** For tests: every language's dictionary. */
export const _dicts = dicts;
