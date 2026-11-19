/** Meta data helpers: augment names, tier letters, sources and credits. */
import { api } from './api';
import { dec } from './format';
import { i18n } from './i18n.svelte';
import { app, updateSettings } from './state.svelte';
import type { AramBracket, Augment, PatchInfo } from './types';

/** Required by the data's license (CC BY-NC 4.0) wherever its numbers show. */
export const ARAM_CREDIT = '海克斯大乱斗统计数据 (github.com/czgreat/aramhex-data), CC BY-NC 4.0';
export const BRACKETS: AramBracket[] = ['all', 'casual', 'mid', 'high'];

export const augs = $state({ lang: '', byId: {} as Record<string, Augment> });

let loading: { lang: string; done: Promise<void> } | null = null;

/** Augment names and icons in the UI language (loaded once per language). */
export function loadAugments(): Promise<void> {
  const lang = i18n.lang;
  if (augs.lang === lang) return Promise.resolve();
  if (loading?.lang === lang) return loading.done;
  const done = api.augmentNames(lang).then(
    (byId) => {
      augs.byId = byId;
      augs.lang = lang;
    },
    (e) => console.warn('augment names', e),
  );
  loading = { lang, done: done.finally(() => (loading = null)) };
  return loading.done;
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

/**
 * The Hextech ARAM pages' shared patch: "" is the current one. Kept while the app runs; the
 * skill bracket is shared the same way (a setting). `patches` is what AramHex has, newest first.
 */
export const hex = $state({ patch: '', patches: [] as PatchInfo[] });

export function setPatch(p: string) {
  hex.patch = p === hex.patches[0]?.patch ? '' : p;
}

/** Keeps the patch list from a page's answer. */
export function notePatches(list: PatchInfo[]) {
  if (list.length && JSON.stringify(list) !== JSON.stringify(hex.patches)) hex.patches = list;
}

/** The patch picked (its info), or the current one. */
export function patchInfo(): PatchInfo | undefined {
  return hex.patches.find((p) => p.patch === hex.patch) ?? hex.patches[0];
}

/** Win rate lines of the tiers, best first: S (夯) 53.5%+, A 51.5%+, B 50%+, C 48%+, D below. */
export const TIER_LINES: { tier: 'S' | 'A' | 'B' | 'C' | 'D'; min: number }[] = [
  { tier: 'S', min: 0.535 },
  { tier: 'A', min: 0.515 },
  { tier: 'B', min: 0.5 },
  { tier: 'C', min: 0.48 },
  { tier: 'D', min: 0 },
];

/** Hextech ARAM tier from a win rate (`TIER_LINES`). */
export function aramTier(winRate: number): 'S' | 'A' | 'B' | 'C' | 'D' {
  return TIER_LINES.find((l) => winRate >= l.min)!.tier;
}

/** OP.GG tier as a letter: 0 is its OP tier (above tier 1), then 1 to 5 as S to D. */
export function riftTier(tier: number): string {
  return ['OP', 'S', 'A', 'B', 'C', 'D'][tier] ?? '';
}

/** The client's game mode is an ARAM variant (Hextech ARAM is "KIWI"). */
export function isAramMode(mode: string): boolean {
  return mode === 'ARAM' || mode === 'KIWI';
}

export function pct1(rate: number): string {
  return dec(rate * 100, 1);
}

/** A share as a percentage with one decimal, "<0.1" for tiny ones. */
export function share(rate: number): string {
  return rate > 0 && rate < 0.0005 ? `<${dec(0.1, 1)}` : pct1(rate);
}

/** "+2.3" / "−1.0" percentage points ("0.0" when it rounds to nothing). */
export function delta(rate: number): string {
  const v = Math.round(rate * 1000) / 10;
  if (v === 0) return dec(0, 1);
  return (v > 0 ? '+' : '−') + dec(Math.abs(v), 1);
}
