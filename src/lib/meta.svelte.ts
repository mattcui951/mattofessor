/** Meta data helpers: augment names, tier letters, sources and credits. */
import { api } from './api';
import { i18n } from './i18n.svelte';
import { app, updateSettings } from './state.svelte';
import type { AramBracket, Augment } from './types';

/** Required by the data's license (CC BY-NC 4.0) wherever its numbers show. */
export const ARAM_CREDIT = '海克斯大乱斗统计数据 (github.com/czgreat/aramhex-data), CC BY-NC 4.0';
export const BRACKETS: AramBracket[] = ['all', 'casual', 'mid', 'high'];

export const augs = $state({ lang: '', byId: {} as Record<string, Augment> });

/** Augment names and icons in the UI language (loaded once per language). */
export async function loadAugments() {
  const lang = i18n.lang;
  if (augs.lang === lang) return;
  try {
    augs.byId = await api.augmentNames(lang);
    augs.lang = lang;
  } catch (e) {
    console.warn('augment names', e);
  }
}

export function augment(id: number): Augment | undefined {
  return augs.byId[String(id)];
}

export function bracket(): AramBracket {
  const b = app.settings?.aramBracket;
  return b && BRACKETS.includes(b) ? b : 'all';
}

export function setBracket(b: AramBracket) {
  updateSettings({ aramBracket: b });
}

/** Hextech ARAM tier from a win rate: S 54%+, A 52%+, B 50%+, C 48%+, D below. */
export function aramTier(winRate: number): string {
  return winRate >= 0.54 ? 'S' : winRate >= 0.52 ? 'A' : winRate >= 0.5 ? 'B' : winRate >= 0.48 ? 'C' : 'D';
}

/** OP.GG tier 1 (OP) to 5 as a letter. */
export function riftTier(tier: number): string {
  return ['', 'S', 'A', 'B', 'C', 'D'][tier] ?? '';
}

/** The client's game mode is an ARAM variant (Hextech ARAM is "KIWI"). */
export function isAramMode(mode: string): boolean {
  return mode === 'ARAM' || mode === 'KIWI';
}

export function pct1(rate: number): string {
  return (rate * 100).toFixed(1);
}

/** "+2.3" / "−1.0" percentage points. */
export function delta(rate: number): string {
  const v = rate * 100;
  return (v >= 0 ? '+' : '−') + Math.abs(v).toFixed(1);
}
