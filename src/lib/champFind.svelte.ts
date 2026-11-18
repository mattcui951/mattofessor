import { api } from './api';
import { buildIndex, searchChamps } from './champSearch';
import { dd, loadOtherNames, otherNames } from './dd.svelte';

/**
 * The one champion search every search box uses: names and titles in the UI language, English and Chinese, pinyin, and
 * Tencent's nicknames, filtered on the page as you type.
 */
export const nicknames = $state({ byId: {} as Record<string, string[]>, loaded: false });

/** Loads what the search needs beyond the UI language: English and Chinese names, nicknames. */
export function loadChampSearch() {
  loadOtherNames();
  if (nicknames.loaded) return;
  nicknames.loaded = true;
  api.championKeywords().then(
    (k) => {
      nicknames.byId = k;
    },
    (e) => {
      console.warn('champion keywords', e);
      nicknames.loaded = false;
    },
  );
}

let built: { a: unknown; b: unknown; c: unknown; index: ReturnType<typeof buildIndex> } | null = null;

/** The search index, rebuilt when a language's names or the nicknames arrive. */
export function champIndex() {
  const a = dd.data?.champions;
  const b = otherNames.sets;
  const c = nicknames.byId;
  if (!built || built.a !== a || built.b !== b || built.c !== c) built = { a, b, c, index: buildIndex([a, ...b], c) };
  return built.index;
}

/** The champions a query finds, best first; none for an empty query. */
export function findChamps(query: string): number[] | null {
  if (!query.trim()) return null;
  return searchChamps(champIndex(), query, Infinity).map((h) => h.id);
}
