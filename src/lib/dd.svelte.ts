/** Data Dragon static data (names in the UI language) and image URLs. */
import { api } from './api';
import { appearance } from './appearance.svelte';
import { i18n, t } from './i18n.svelte';
import { readableOn } from './themes';
import type { Champ, ItemClass, Rank, StaticData, TierDiv } from './types';

const CDN = 'https://ddragon.leagueoflegends.com/cdn';

export const dd = $state({ data: null as StaticData | null, lang: '' });

export async function loadStatic(lang: string) {
  if (dd.lang === lang && dd.data) return;
  try {
    dd.data = await api.staticData(lang);
    dd.lang = lang;
  } catch (e) {
    console.warn('static data', e);
  }
}

/** Languages every champion search also knows, whatever the UI language (English, Chinese and its pinyin). */
const SEARCH_ALSO = ['en', 'zh'];

/**
 * Champion names in those languages besides the UI's: `champions` is the first of them (shown
 * under a search result), `sets` all of them (the search index).
 */
export const otherNames = $state({ lang: '', champions: null as Record<string, Champ> | null, sets: [] as Record<string, Champ>[] });

export async function loadOtherNames() {
  if (!dd.lang || otherNames.lang === dd.lang) return;
  const want = SEARCH_ALSO.filter((l) => l !== dd.lang);
  const ui = dd.lang;
  otherNames.lang = ui;
  try {
    const sets = await Promise.all(want.map((l) => api.staticData(l).then((d) => d.champions)));
    if (dd.lang !== ui) return;
    otherNames.sets = sets;
    otherNames.champions = sets[0] ?? null;
  } catch (e) {
    console.warn('static data', want, e);
    otherNames.lang = '';
  }
}

export function champ(id: number) {
  return dd.data?.champions[String(id)];
}

export function champName(id: number): string {
  return champ(id)?.name ?? (id > 0 ? `#${id}` : '?');
}

export function champIcon(id: number): string | null {
  const c = champ(id);
  return c && dd.data ? `${CDN}/${dd.data.version}/img/champion/${c.key}.png` : null;
}

export function champIconByKey(key: string): string | null {
  return dd.data && key ? `${CDN}/${dd.data.version}/img/champion/${key}.png` : null;
}

export const ITEM_CLASSES: ItemClass[] = ['Fighter', 'Assassin', 'Mage', 'Tank', 'Support', 'Marksman', 'Other'];
/**
 * Items ranked under Other whatever their shop roles: the Guardian starters, the small anti-heal
 * items, and the items only Hextech ARAM augments give (Void Immolation, Ultra Hydra, Wooglet's
 * Witchcap, Golden Spatula).
 */
export const OTHER_ITEMS = [2051, 3112, 3177, 3184, 3076, 3916, 3123, 223069, 226668, 228002, 994403];

/** An item's shop roles; Other for `OTHER_ITEMS`, components and items without a role. */
export function itemClasses(id: number): ItemClass[] {
  const it = dd.data?.items[String(id)];
  if (!it || OTHER_ITEMS.includes(id) || !(it.aramLegendary || it.boots) || !it.classes?.length) return ['Other'];
  return it.classes as ItemClass[];
}

export function itemIcon(id: number): string | null {
  return dd.data && id > 0 ? `${CDN}/${dd.data.version}/img/item/${id}.png` : null;
}

export function itemName(id: number): string {
  return dd.data?.items[String(id)]?.name ?? '';
}

export function spell(id: number) {
  return dd.data?.spells[String(id)];
}

export function spellByKey(key: string) {
  if (!dd.data) return null;
  for (const [id, s] of Object.entries(dd.data.spells)) if (s.key === key) return { id: Number(id), ...s };
  return null;
}

export function spellIcon(idOrKey: number | string): string | null {
  const key = typeof idOrKey === 'number' ? spell(idOrKey)?.key : idOrKey;
  return dd.data && key ? `${CDN}/${dd.data.version}/img/spell/${key}.png` : null;
}

export function profileIcon(id: number): string | null {
  return dd.data ? `${CDN}/${dd.data.version}/img/profileicon/${id}.png` : null;
}

export function rune(id: number) {
  for (const tr of dd.data?.trees ?? []) {
    if (tr.id === id) return { id, name: tr.name, icon: tr.icon };
    for (const slot of tr.slots) for (const r of slot) if (r.id === id) return r;
  }
  return null;
}

export function runeIcon(id: number): string | null {
  const r = rune(id);
  return r ? `https://ddragon.leagueoflegends.com/cdn/img/${r.icon}` : null;
}

/** Stat shard ids are not in Data Dragon. */
/** A rune stat shard's name in the UI language (Riot's data). */
export function shardName(id: number): string {
  return dd.data?.shards?.[String(id)] ?? String(id);
}

export const TIER_COLORS: Record<string, string> = {
  IRON: '#a39a92',
  BRONZE: '#c98c62',
  SILVER: '#b7c4d1',
  GOLD: '#ffc83d',
  PLATINUM: '#4fd1c5',
  EMERALD: '#3ddc84',
  DIAMOND: '#7fa7ff',
  MASTER: '#c58cff',
  GRANDMASTER: '#ff6b6b',
  CHALLENGER: '#ffd86b',
};

const APEX = ['MASTER', 'GRANDMASTER', 'CHALLENGER'];

export function tierText(td: TierDiv | Rank | null | undefined, short = false): string {
  if (!td || !td.tier) return t('common.unranked');
  const name = t(`tier.${td.tier}`);
  if (APEX.includes(td.tier) || !td.division || td.division === 'NA') return name;
  const div = short ? { I: '1', II: '2', III: '3', IV: '4' }[td.division] ?? td.division : td.division;
  return i18n.lang === 'zh' ? `${name} ${div}` : `${name} ${div}`;
}

export function tierColor(tier: string | undefined): string {
  return (tier && TIER_COLORS[tier] && readableOn(TIER_COLORS[tier], appearance.theme)) || 'var(--text-3)';
}

const TIER_ORDER = ['IRON', 'BRONZE', 'SILVER', 'GOLD', 'PLATINUM', 'EMERALD', 'DIAMOND', 'MASTER', 'GRANDMASTER', 'CHALLENGER'];
export function tierScore(td: TierDiv | null | undefined): number {
  if (!td) return -1;
  const t = TIER_ORDER.indexOf(td.tier);
  if (t < 0) return -1;
  if (t >= 7) return 28 + (t - 7) * 4;
  return t * 4 + ({ I: 3, II: 2, III: 1 }[td.division] ?? 0);
}
