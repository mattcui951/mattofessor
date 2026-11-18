/**
 * Champion search across languages: English name and title, Chinese name and title, the Data
 * Dragon key, pinyin (full or initials) of the Chinese names, and the nicknames in Tencent's hero
 * list ("大嘴" for Kog'Maw), so "jax", "grandmaster", "贾克斯", "武器大师", "jiakesi" and "jks"
 * all find Jax. A query of four letters or more also forgives a typo or two ("cogmo").
 */
import pinyin from 'tiny-pinyin';
import type { Champ } from './types';

export type ChampHit = { id: number; score: number };

/** `name` terms (name, key) outrank `title` terms ("the Blind Monk", 武器大师). */
type Entry = { id: number; name: string[]; title: string[] };

/** Lower case, no spaces or punctuation, full-width letters folded ("Kai'Sa" -> "kaisa"). */
export function norm(s: string): string {
  return s
    .normalize('NFKC')
    .toLowerCase()
    .replace(/[\s'’.·&_-]/g, '');
}

function pinyinTerms(text: string): string[] {
  if (!text || !/[一-鿿]/.test(text) || !pinyin.isSupported()) return [];
  const syllables = pinyin.convertToPinyin(text, ' ', true).split(' ').filter(Boolean);
  return [syllables.join(''), syllables.map((s) => s[0]).join('')];
}

/**
 * Search terms for every champion, from the static data of each language that is loaded and the
 * nicknames by champion id (count as names).
 */
export function buildIndex(datasets: Array<Record<string, Champ> | null | undefined>, nicknames: Record<string, string[]> = {}): Entry[] {
  const terms = new Map<number, { name: Set<string>; title: Set<string> }>();
  const add = (set: Set<string>, text: string) => {
    if (!text) return;
    set.add(norm(text));
    for (const p of pinyinTerms(text)) set.add(p);
  };
  for (const champs of datasets) {
    for (const [id, c] of Object.entries(champs ?? {})) {
      const e = terms.get(Number(id)) ?? { name: new Set<string>(), title: new Set<string>() };
      add(e.name, c.name);
      add(e.name, c.key);
      add(e.title, c.alias);
      terms.set(Number(id), e);
    }
  }
  for (const [id, words] of Object.entries(nicknames)) {
    const e = terms.get(Number(id));
    if (e) for (const w of words) add(e.name, w);
  }
  const list = (s: Set<string>) => [...s].filter(Boolean);
  return [...terms].map(([id, e]) => ({ id, name: list(e.name), title: list(e.title) }));
}

function level(terms: string[], q: string): number {
  let best = 0;
  for (const term of terms) {
    const s = term === q ? 3 : term.startsWith(q) ? 2 : term.includes(q) ? 1 : 0;
    if (s > best) best = s;
    if (best === 3) break;
  }
  return best;
}

/** Edit distance between two short strings (insert, delete, replace). */
function distance(a: string, b: string): number {
  let prev = Array.from({ length: b.length + 1 }, (_, i) => i);
  for (let i = 1; i <= a.length; i++) {
    const cur = [i];
    for (let j = 1; j <= b.length; j++) cur[j] = Math.min(prev[j]! + 1, cur[j - 1]! + 1, prev[j - 1]! + (a[i - 1] === b[j - 1] ? 0 : 1));
    prev = cur;
  }
  return prev[b.length]!;
}

/** Whether a Latin-letter query is a near miss of a name's start ("cogmo" for "kogmaw"). */
function near(terms: string[], q: string): boolean {
  if (q.length < 4 || !/^[a-z0-9]+$/.test(q)) return false;
  const most = q.length >= 5 ? 2 : 1;
  return terms.some((t) => /^[a-z0-9]+$/.test(t) && [q.length - 1, q.length, q.length + 1].some((n) => n > 0 && distance(q, t.slice(0, n)) <= most));
}

/**
 * Best matches first: exact, then prefix, then contains, a name before a title at each step,
 * then near misses when nothing else matched.
 */
export function searchChamps(index: Entry[], query: string, limit = 8): ChampHit[] {
  const q = norm(query);
  if (!q) return [];
  const hits: ChampHit[] = [];
  for (const e of index) {
    const score = Math.max(level(e.name, q) * 2, level(e.title, q) * 2 - 1);
    if (score > 0) hits.push({ id: e.id, score });
  }
  if (!hits.length) for (const e of index) if (near(e.name, q)) hits.push({ id: e.id, score: 0.5 });
  hits.sort((a, b) => b.score - a.score);
  return hits.slice(0, limit);
}

/** "Name#TAG" as typed, with a full-width ＃ (Chinese input) accepted. */
export function riotIdOf(query: string): { name: string; tag: string } | null {
  const q = query.replace(/＃/g, '#').trim();
  const at = q.lastIndexOf('#');
  if (at < 0) return null;
  const name = q.slice(0, at).trim();
  const tag = q.slice(at + 1).trim();
  return name && tag ? { name, tag } : null;
}
