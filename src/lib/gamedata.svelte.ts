import { listen } from '@tauri-apps/api/event';
import { api } from './api';
import { i18n } from './i18n.svelte';

/** Text in both languages. */
export type Bi = { en: string; zh: string };

/** A champion's Hextech ARAM modifier keys (`tools/refresh-data`). */
export type ModKey = 'dealt' | 'received' | 'healing' | 'shielding' | 'tenacity' | 'abilityHaste' | 'energyRegen' | 'attackSpeed';
export const MOD_KEYS: ModKey[] = ['dealt', 'received', 'healing', 'shielding', 'tenacity', 'abilityHaste', 'energyRegen', 'attackSpeed'];

/** `text`: the wiki's English; `zh`: its Chinese, when translated. */
export type AbilityMods = { key: 'P' | 'Q' | 'W' | 'E' | 'R' | '' | null; name: Bi; lines: { text: string; zh?: string; mode: 'aram' | 'mayhem' }[] };
/** `stats`: the modifiers in Hextech ARAM (fractions; ability haste in points). `aram`: what regular ARAM has where Hextech ARAM changed it. */
export type ChampMods = { name: string; stats: Partial<Record<ModKey, number>>; aram: Partial<Record<ModKey, number>>; abilities: AbilityMods[] };
export type RuleItem = { label: Bi; text: Bi; note?: Bi; highlight?: boolean };
export type Mayhem = {
  patch: string;
  source: { name: string; url: string; license: string };
  sections: { id: string; title: Bi; items: RuleItem[] }[];
  headliners: { id: string; name: Bi; official: boolean; effect: Bi }[];
  attackTypes: Record<string, 'melee' | 'ranged'>;
  /** Champion id to ability key (P, Q, W, E, R) to its name. */
  abilityNames: Record<string, Record<string, Bi>>;
  champions: Record<string, ChampMods>;
  /** Augment id to its description with numbers (the game's tooltip markup). */
  augments?: Record<string, Bi>;
};

export type NoteLine = { text: string; attr?: string; before?: string; after?: string };
export type NoteEntry = {
  group: string;
  name: string;
  isNew: boolean;
  summary: string;
  parts: { title: string; lines: NoteLine[] }[];
  championId?: number;
  itemId?: number;
  augmentId?: number;
};
export type NoteSection = { title: string; intro: string; lines: NoteLine[]; entries: NoteEntry[] };
export type Patch = { patch: string; url: string; date: string; rift: NoteSection[]; mayhem: NoteSection[] };
/** `patches`: Riot's notes (English); `zh.patches`: Tencent's official Chinese notes. */
export type PatchNotes = { source: { name: string; url: string }; patches: Patch[]; zh?: { source: { name: string; url: string }; patches: Patch[] } };
export type GameData = { checkedAt: string; mayhem: Mayhem; patchNotes: PatchNotes };

/**
 * The refreshed game data (Hextech ARAM modifiers and rules, patch notes): loaded once and again
 * whenever the app takes a newer copy ("data" event).
 */
export const gameData = $state({ data: null as GameData | null, loaded: false });

export function loadGameData() {
  if (gameData.loaded) return;
  gameData.loaded = true;
  const get = () =>
    api.gameData().then(
      (d) => (gameData.data = d),
      (e) => {
        console.warn('game data', e);
        gameData.loaded = false;
      },
    );
  listen('data', get);
  get();
}

/** The UI language's side of a text. */
export const bi = (b: Bi | undefined) => (b ? (i18n.lang === 'zh' ? b.zh : b.en) : '');

/** Modifiers that are a share of normal (damage, healing, shielding): shown as "90%". */
const SHARE: ModKey[] = ['dealt', 'received', 'healing', 'shielding'];

/** A modifier as shown: "90%" for shares, "+10%" for bonuses, "-10" for ability haste. */
export function modText(key: ModKey, v: number): string {
  if (SHARE.includes(key)) return `${Math.round((1 + v) * 1000) / 10}%`;
  const n = key === 'abilityHaste' ? v : Math.round(v * 1000) / 10;
  const s = `${n > 0 ? '+' : ''}${n}`;
  return key === 'abilityHaste' ? s : `${s}%`;
}

/** Whether a modifier helps the champion (taking less damage helps; the rest help when higher). */
export const modGood = (key: ModKey, v: number) => (key === 'received' ? v < 0 : v > 0);

/** Patch note attributes for the same modifiers ("Damage Taken: 110% ⇒ 105%"). */
const NOTE_ATTRS: Record<string, ModKey> = {
  'damage dealt': 'dealt',
  'damage taken': 'received',
  'damage received': 'received',
  'healing modifier': 'healing',
  'heal modifier': 'healing',
  'healing done': 'healing',
  'shield modifier': 'shielding',
  'shielding done': 'shielding',
  tenacity: 'tenacity',
  'ability haste': 'abilityHaste',
  'energy regen': 'energyRegen',
  'energy regeneration': 'energyRegen',
  'total attack speed increase': 'attackSpeed',
  'total attack speed': 'attackSpeed',
  // Tencent's notes
  造成伤害: 'dealt',
  承受伤害: 'received',
  治疗效果: 'healing',
  护盾效果: 'shielding',
  韧性: 'tenacity',
  技能急速: 'abilityHaste',
  能量回复: 'energyRegen',
  总攻击速度提升: 'attackSpeed',
  总攻击速度: 'attackSpeed',
};
export const noteMod = (attr: string | undefined): ModKey | undefined => (attr ? NOTE_ATTRS[attr.toLowerCase()] : undefined);

/** Comparable text: lowercase letters and digits only (accents dropped). */
export const plain = (s: string) => s.normalize('NFKD').toLowerCase().replace(/[^a-z0-9]/g, '');

const noteNum = (s: string | undefined) => (s ? parseFloat(s.replace(/[^\d.-]/g, '')) : NaN);

/** For a Hextech ARAM modifier line: whether the change helps the champion. */
export function noteBetter(l: NoteLine): boolean | undefined {
  const k = noteMod(l.attr);
  const a = noteNum(l.before);
  const b = noteNum(l.after);
  if (!k || Number.isNaN(a) || Number.isNaN(b) || a === b) return undefined;
  return modGood(k, b - a);
}

/** A patch note entry's direction when every modifier line agrees. */
export function noteVerdict(e: NoteEntry): 'buff' | 'nerf' | '' {
  const v = e.parts.flatMap((p) => p.lines.map(noteBetter)).filter((x) => x !== undefined);
  if (!v.length) return '';
  return v.every(Boolean) ? 'buff' : v.every((x) => !x) ? 'nerf' : '';
}
