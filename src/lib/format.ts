import { hasKey, i18n, t } from './i18n.svelte';
import type { Tag } from './types';
import { champName, tierText } from './dd.svelte';
import { locale } from './locales';

export function pct(wins: number, games: number): number {
  return games > 0 ? Math.round((wins * 100) / games) : 0;
}

export function kdaRatio(k: number, d: number, a: number): string {
  if (d === 0) return k + a > 0 ? t('common.perfect') : dec(0, 2);
  return dec((k + a) / d, 2);
}

/** 1st, 2nd, 3rd... in English; each other language's own form (`fmt.ordinal`). */
export function ordinal(n: number): string {
  if (i18n.lang !== 'en') return t('fmt.ordinal', n);
  const tail = n % 100 >= 11 && n % 100 <= 13 ? 'th' : ({ 1: 'st', 2: 'nd', 3: 'rd' } as Record<number, string>)[n % 10] ?? 'th';
  return `${n}${tail}`;
}

/** A count with the UI language's digit grouping (57,823 · 57 823 · 57.823), not the system's. */
export function num(n: number): string {
  return n.toLocaleString(locale(i18n.lang).tag);
}

/** A number with a fixed count of decimals in the UI language's way (54.7 · 54,7). */
export function dec(n: number, digits = 1): string {
  return n.toLocaleString(locale(i18n.lang).tag, { minimumFractionDigits: digits, maximumFractionDigits: digits, useGrouping: false });
}

/** A date as the UI language writes it (11/17/2026 · 2026/11/17 · 17/11/2026), not the system. */
export function day(d: string | number | Date): string {
  return new Date(d).toLocaleDateString(locale(i18n.lang).tag);
}

export function gamesText(n: number): string {
  return n === 1 ? t('common.game', n) : t('common.games', n);
}

export function one(n: number): string {
  return dec(n, 1);
}

export function ago(ms: number, now = Date.now()): string {
  const s = Math.max(0, (now - ms) / 1000);
  if (s < 60) return t('time.now');
  if (s < 3600) return t('time.min', Math.floor(s / 60));
  if (s < 86400) return t('time.hour', Math.floor(s / 3600));
  return new Intl.RelativeTimeFormat(locale(i18n.lang).tag, { numeric: 'always' })
    .format(-Math.floor(s / 86400), 'day');
}

export function duration(sec: number): string {
  const m = Math.floor(sec / 60);
  const s = Math.floor(sec % 60);
  return `${m}:${String(s).padStart(2, '0')}`;
}

/**
 * A number in short, as the UI language writes it: "23K", "57.8M", "2.3万", "5782만", "57,8 M".
 * No decimal once the leading number has three digits ("5782万", not "5782.3万").
 */
export function compact(n: number): string {
  const tag = locale(i18n.lang).tag;
  const short = new Intl.NumberFormat(tag, { notation: 'compact', maximumFractionDigits: 1 }).format(n);
  return /\d{3}/.test(short) ? new Intl.NumberFormat(tag, { notation: 'compact', maximumFractionDigits: 0 }).format(n) : short;
}

const BOT_QUEUES = [830, 840, 850, 870, 880, 890];

export function queueName(id: number): string {
  if (BOT_QUEUES.includes(id)) return t('queue.bot');
  // Custom games (3100 and on; 3140 is the Practice Tool).
  if (id >= 3100 && id < 3200 && id !== 3140) return t('queue.0');
  const k = `queue.${id}`;
  return hasKey(k) ? t(k) : t('queue.other');
}

/** "mid", or a group like "mid+top" (champions played in several lanes). */
export function roleName(role: string): string {
  return role
    .split('+')
    .filter(Boolean)
    .map((r) => t(`role.${r}`))
    .join(' / ');
}

/** Client position names (TOP, UTILITY...) to role keys. */
export function positionRole(p: string): string {
  return { TOP: 'top', JUNGLE: 'jungle', MIDDLE: 'mid', MID: 'mid', BOTTOM: 'adc', UTILITY: 'support' }[p.toUpperCase()] ?? '';
}

export function tagText(tag: Tag): string {
  const arg = tag.arg;
  switch (tag.key) {
    case 'oneTrick':
      return t('tag.oneTrick', champName(Number(arg)));
    case 'mainRole':
    case 'offRole':
      return t(`tag.${tag.key}`, roleName(String(arg)));
    case 'wasHigher':
      return t('tag.wasHigher', tierText({ tier: String(arg), division: '' }));
    case 'strongChamp': {
      const [champ, pct] = arg as [number, number];
      return t('tag.strongChamp', champName(Number(champ)), pct);
    }
    case 'style':
      return t('tag.style', t(`class.${String(arg)}`));
    default:
      return t(`tag.${tag.key}`, Array.isArray(arg) ? arg.join(' ') : (arg ?? ''));
  }
}

export function tagHint(tag: Tag): string {
  return t(`tagHint.${tag.key}`);
}
