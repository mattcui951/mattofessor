/**
 * Profiles kept for the whole session and saved on disk by the backend, one per account.
 *
 * Opening a profile shows what is saved right away (finished games never change), then refreshes
 * in the background when the data is older than five minutes: new games are added to the top,
 * games already shown keep their rows, and only new games get scored. The profile on screen and
 * the signed-in account are refreshed every five minutes, and the signed-in account after each game.
 */
import { api } from './api';
import { app, errorText } from './state.svelte';
import type { MyScore, Profile } from './types';

export const STALE_MS = 5 * 60_000;

export type ProfileEntry = {
  profile: Profile | null;
  scores: Record<number, MyScore>;
  scoring: boolean;
  refreshing: boolean;
  /** The last refresh failed (the saved data, if any, is still shown). */
  error: string | null;
  /** The saved copy was looked up (so "nothing saved" is known). */
  checked: boolean;
  /** Only the quick first look is shown (rank, mastery); the games are still loading. */
  partial: boolean;
};

export const profiles = $state({
  byId: {} as Record<string, ProfileEntry>,
  /** The last signed-in account, known from the saved data before the client connects. */
  lastMe: '',
  /** The profile on screen, for the background refresh. */
  viewing: '',
});

/** The signed-in account (or the last one, while the client is closed). */
export function meId(): string {
  return app.me?.puuid ?? profiles.lastMe;
}

function entry(id: string): ProfileEntry {
  profiles.byId[id] ??= { profile: null, scores: {}, scoring: false, refreshing: false, error: null, checked: false, partial: false };
  return profiles.byId[id];
}

function stale(e: ProfileEntry): boolean {
  return !e.profile || Date.now() - e.profile.updatedAt > STALE_MS;
}

/** Show a profile: the saved copy at once, then a background refresh if it is old. */
export async function openProfile(puuid: string | undefined) {
  let id = puuid || meId();
  if (!id) {
    // Nothing known yet about who is signed in: the backend remembers the last account.
    const saved = await api.profileSaved(null).catch(() => null);
    if (!saved) return;
    id = saved.card.summoner.puuid;
    profiles.lastMe = id;
    const e = entry(id);
    e.profile ??= saved;
    e.checked = true;
  }
  profiles.viewing = id;
  const e = entry(id);
  if (!e.checked) {
    e.checked = true;
    const saved = await api.profileSaved(id).catch(() => null);
    if (saved && !e.profile) e.profile = saved;
  }
  scoreMissing(id);
  if (!e.profile) quickLook(id);
  if (stale(e)) refresh(id);
}

/** A player never looked up: their rank and mastery while the games load. */
async function quickLook(id: string) {
  const e = entry(id);
  if (!app.connected) return;
  try {
    const quick = await api.profileQuick(id);
    if (!e.profile) {
      e.profile = quick;
      e.partial = true;
    }
  } catch {
    /* the full lookup reports the error */
  }
}

/** Fetch the latest from the client; does nothing while a refresh of this profile runs. */
export async function refresh(id: string) {
  const e = entry(id);
  if (e.refreshing || !app.connected) return;
  e.refreshing = true;
  try {
    const fresh = await api.profile(id);
    if (fresh.isMe) profiles.lastMe = fresh.card.summoner.puuid;
    merge(e, fresh);
    e.partial = false;
    e.error = null;
  } catch (err) {
    e.error = errorText(err);
  } finally {
    e.refreshing = false;
  }
  scoreMissing(id);
}

/** New data in place: rows of games already shown are kept as they are (they never change). */
function merge(e: ProfileEntry, fresh: Profile) {
  const old = new Map((e.profile?.history ?? []).map((g) => [g.gameId, g]));
  fresh.history = fresh.history.map((g) => old.get(g.gameId) ?? g);
  e.profile = fresh;
}

/** Scores for games that have none yet, a few at a time, newest first. */
async function scoreMissing(id: string) {
  const e = entry(id);
  if (e.scoring || !e.profile) return;
  e.scoring = true;
  try {
    for (;;) {
      const p = e.profile;
      const todo = p.history.filter((g) => !g.remake && !(g.gameId in e.scores)).map((g) => g.gameId).slice(0, 5);
      if (!todo.length) break;
      const got = await api.playerScores(p.card.summoner.puuid, todo);
      for (const s of got) e.scores[s.gameId] = s;
      // Games without a score (e.g. not retrievable) are not asked for again this session.
      for (const g of todo) if (!(g in e.scores)) e.scores[g] = { gameId: g, score: NaN, badge: '', place: 0, players: 0 };
    }
  } catch {
    /* client gone: scores come with the next refresh */
  } finally {
    e.scoring = false;
  }
}

/** A real score (placeholders for games without one are left out). */
export function scoreOf(e: ProfileEntry | undefined, gameId: number): MyScore | undefined {
  const s = e?.scores[gameId];
  return s && !Number.isNaN(s.score) ? s : undefined;
}

let timer: ReturnType<typeof setInterval> | undefined;

/** Background refresh of the profile on screen and the signed-in account (once per app start). */
export function startAutoRefresh() {
  if (timer) return;
  timer = setInterval(() => {
    for (const id of new Set([profiles.viewing, meId()])) {
      const e = id ? profiles.byId[id] : undefined;
      if (e && stale(e)) refresh(id);
    }
  }, 60_000);
}

/** A game just ended: the signed-in account has a new game. */
export function gameEnded() {
  const id = meId();
  if (id) refresh(id);
}
