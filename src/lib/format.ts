import { hasKey, i18n, t } from './i18n.svelte';
import type { Tag } from './types';
import { champName, tierText } from './dd.svelte';

export function pct(wins: number, games: number): number {
  return games > 0 ? Math.round((wins * 100) / games) : 0;
}

export function kdaRatio(k: number, d: number, a: number): string {
  if (d === 0) return k + a > 0 ? t('common.perfect') : '0.00';
  return ((k + a) / d).toFixed(2);
}

/** 1st, 2nd, 3rd... (第1, 第2... in Chinese). */
export function ordinal(n: number): string {
  if (i18n.lang === 'zh') return `第${n}`;
  const tail = n % 100 >= 11 && n % 100 <= 13 ? 'th' : ({ 1: 'st', 2: 'nd', 3: 'rd' } as Record<number, string>)[n % 10] ?? 'th';
  return `${n}${tail}`;
}

export function gamesText(n: number): string {
  return n === 1 ? t('common.game', n) : t('common.games', n);
}

export function one(n: number): string {
  return n.toFixed(1);
}

export function ago(ms: number, now = Date.now()): string {
  const s = Math.max(0, (now - ms) / 1000);
  if (s < 60) return t('time.now');
  if (s < 3600) return t('time.min', Math.floor(s / 60));
  if (s < 86400) return t('time.hour', Math.floor(s / 3600));
  return t('time.days', Math.floor(s / 86400));
}

export function duration(sec: number): string {
  const m = Math.floor(sec / 60);
  const s = Math.floor(sec % 60);
  return `${m}:${String(s).padStart(2, '0')}`;
}

export function compact(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1000) return `${Math.round(n / 1000)}k`;
  return String(n);
}

const BOT_QUEUES = [830, 840, 850, 870, 880, 890];

export function queueName(id: number): string {
  if (BOT_QUEUES.includes(id)) return t('queue.bot');
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
