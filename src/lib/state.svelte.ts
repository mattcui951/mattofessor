import { listen } from '@tauri-apps/api/event';
import { api } from './api';
import { applySettings } from './appearance.svelte';
import { loadStatic } from './dd.svelte';
import { hasKey, i18n, t, type Lang } from './i18n.svelte';
import { isLang, locale } from './locales';
import type { BuildMode, InGame, LiveState, RiotStatus, Settings, Summoner, UpdateState } from './types';

export type Route =
  | { name: 'profile'; puuid?: string }
  | { name: 'live' }
  | { name: 'builds'; championId?: number; role?: string; mode?: BuildMode }
  | { name: 'meta'; mode?: BuildMode; view?: 'champions' | 'augments' | 'combos' | 'items'; augment?: number }
  | { name: 'tiers' }
  | { name: 'guide' }
  | { name: 'patches' }
  | { name: 'settings' }
  | { name: 'appearance' }
  | { name: 'riotGuide' };

type Toast = { id: number; text: string; tone: 'info' | 'error' | 'good' };

const RELEASES = 'https://github.com/mattcui951/mattofessor/releases';

const emptyLive: LiveState = { phase: '', connected: false, queueId: 0, mode: '', mapId: 0, teams: [], bench: [] };

export const app = $state({
  ready: false,
  version: '',
  settings: null as Settings | null,
  connected: false,
  me: null as Summoner | null,
  clientLang: 'en' as Lang,
  riotStatus: 'none' as RiotStatus,
  live: emptyLive as LiveState,
  update: { phase: 'idle', version: null, progress: null, error: null, checkedAt: null } as UpdateState,
  /** "Later" on the update banner hides it for that version until the app restarts. */
  updateDismissed: null as string | null,
  /** The Full screen notice was closed (until the app restarts). */
  fullscreenNoticeClosed: false,
  /** "Later" on the Riot API key banner hides it for that key status until the app restarts. */
  riotKeyDismissed: null as RiotStatus | null,
  ingame: null as InGame | null,
  route: { name: 'profile' } as Route,
  /** Pages before this one (back), and after it once you went back (forward). */
  history: [] as Route[],
  future: [] as Route[],
  toasts: [] as Toast[],
  /** Bumped when the signed-in player's profile should refetch (a game ended). */
  profileEpoch: 0,
});

let toastId = 0;
export function toast(text: string, tone: Toast['tone'] = 'info') {
  const id = ++toastId;
  app.toasts.push({ id, text, tone });
  setTimeout(
    () => {
      app.toasts = app.toasts.filter((x) => x.id !== id);
    },
    tone === 'error' ? 7000 : 4000,
  );
}

/** Backend errors look like "[code:arg] English text"; known codes are shown translated. */
export function errorText(e: unknown): string {
  const s = typeof e === 'string' ? e : e instanceof Error ? e.message : JSON.stringify(e);
  const m = /^\[([a-z-]+)(?::([^\]]*))?\] (.*)$/s.exec(s);
  if (!m) return s;
  const key = `err.${m[1]}`;
  return hasKey(key) ? t(key, m[2] ?? '') : m[3];
}

const same = (a: Route, b: Route) => JSON.stringify(a) === JSON.stringify(b);

/** Opens a page (every link and the navigation go through here), remembering the one left. */
export function go(route: Route) {
  if (same(route, app.route)) return;
  app.history.push(app.route);
  if (app.history.length > 50) app.history.shift();
  app.future = [];
  app.route = route;
}

/** The page before, as it was left (pages keep their tabs and filters). */
export function back() {
  const prev = app.history.pop();
  if (!prev) return;
  app.future.push(app.route);
  app.route = prev;
}

/** The page went back from. */
export function forward() {
  const next = app.future.pop();
  if (!next) return;
  app.history.push(app.route);
  app.route = next;
}

/** Mouse side buttons and Alt + Left / Right go back and forward, as in a browser. */
export function navKeys(): () => void {
  const mouse = (e: MouseEvent) => {
    if (e.button !== 3 && e.button !== 4) return;
    e.preventDefault();
    if (e.type === 'mouseup') (e.button === 3 ? back : forward)();
  };
  const key = (e: KeyboardEvent) => {
    if (!e.altKey || (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight')) return;
    e.preventDefault();
    (e.key === 'ArrowLeft' ? back : forward)();
  };
  window.addEventListener('mousedown', mouse);
  window.addEventListener('mouseup', mouse);
  window.addEventListener('keydown', key);
  return () => {
    window.removeEventListener('mousedown', mouse);
    window.removeEventListener('mouseup', mouse);
    window.removeEventListener('keydown', key);
  };
}

/** "live", "profile/<puuid>", "builds", "settings" (from the backend). */
export function routeFrom(path: string): Route {
  const [name, arg] = path.replace(/^#?\/?/, '').split('/');
  if (name === 'live') return { name: 'live' };
  if (name === 'builds') {
    // builds/<championId>/<mode>, from the in-game panel.
    const championId = Number(arg) || undefined;
    const mode = path.replace(/^#?\/?/, '').split('/')[2];
    return { name: 'builds', championId, mode: mode === 'aram' ? 'aram' : championId ? 'rift' : undefined };
  }
  if (name === 'meta') return { name: 'meta' };
  if (name === 'tiers') return { name: 'tiers' };
  if (name === 'guide') return { name: 'guide' };
  if (name === 'patches') return { name: 'patches' };
  if (name === 'settings') return { name: 'settings' };
  if (name === 'appearance') return { name: 'appearance' };
  if (name === 'riot-guide') return { name: 'riotGuide' };
  return { name: 'profile', puuid: arg || undefined };
}

export function releaseUrl(version: string): string {
  return `${RELEASES}/tag/v${version}`;
}

/** A match is running (updating would close the in-game panel). */
export function inGame(): boolean {
  return ['GameStart', 'InProgress', 'Reconnect'].includes(app.live.phase);
}

export async function checkUpdate() {
  try {
    app.update = await api.checkUpdate();
  } catch (e) {
    toast(errorText(e), 'error');
  }
}

/** A check older than this is redone when the window is opened or focused. */
const STALE_CHECK_MS = 10 * 60_000;

/**
 * Looks for a new version when the last look is old (the window was just opened from the tray,
 * or came back to the front), so a release shows up without pressing Check for updates.
 */
export async function checkUpdateIfStale() {
  if (!app.settings?.checkUpdates) return;
  const u = app.update;
  if (['checking', 'downloading', 'installing'].includes(u.phase)) return;
  if (u.checkedAt && Date.now() - u.checkedAt < STALE_CHECK_MS) return;
  try {
    app.update = await api.checkUpdate();
  } catch {
    /* shown on the Settings page */
  }
}

/** Download and run the installer; the app closes and comes back on the new version. */
export async function installUpdate() {
  try {
    await api.installUpdate();
  } catch (e) {
    toast(errorText(e), 'error');
  }
}

export function uiLang(): Lang {
  const s = app.settings?.language ?? 'auto';
  return isLang(s) ? s : isLang(app.clientLang) ? app.clientLang : 'en';
}

async function applyLang() {
  i18n.lang = uiLang();
  document.documentElement.lang = locale(i18n.lang).tag;
  document.documentElement.style.setProperty('--font-cjk', locale(i18n.lang).cjkFont);
  await loadStatic(i18n.lang);
}

/** Language, theme and text size from the current settings. */
async function applyPrefs() {
  if (app.settings) applySettings(app.settings);
  await applyLang();
}

let saveTimer: ReturnType<typeof setTimeout> | undefined;

/** Change settings. `debounce` applies right away but writes after a short pause (sliders, color drags). */
export async function updateSettings(patch: Partial<Settings>, opts: { debounce?: boolean } = {}) {
  if (!app.settings) return;
  const next = { ...app.settings, ...patch };
  app.settings = next;
  applySettings(next);
  clearTimeout(saveTimer);
  saveTimer = undefined;
  const save = async () => {
    saveTimer = undefined;
    try {
      await api.saveSettings(app.settings!);
    } catch (e) {
      toast(errorText(e), 'error');
    }
  };
  if (opts.debounce) saveTimer = setTimeout(save, 300);
  else await save();
  await applyLang();
}

export async function init() {
  const b = await api.bootstrap();
  app.version = b.version;
  app.settings = b.settings;
  app.connected = b.connected;
  app.me = b.me;
  app.clientLang = b.language;
  app.riotStatus = b.riotStatus;
  app.update = b.update;
  app.live = b.live;
  app.ingame = b.ingame;
  if (location.hash.startsWith('#/')) app.route = routeFrom(location.hash);
  await applyPrefs();
  app.ready = true;
  // The main window opened from the tray, or brought back to the front, looks for a new version.
  if (!['#overlay', '#spells', '#cards'].includes(location.hash)) {
    setTimeout(checkUpdateIfStale, 3000);
    window.addEventListener('focus', () => checkUpdateIfStale());
    document.addEventListener('visibilitychange', () => {
      if (document.visibilityState === 'visible') checkUpdateIfStale();
    });
  }
  if (b.updatedFrom) toast(t('update.done', b.version), 'good');

  listen<LiveState>('live', (e) => {
    app.live = e.payload;
  });
  listen<InGame>('ingame', (e) => {
    app.ingame = e.payload;
  });
  listen<{ connected: boolean; me: Summoner | null; language: Lang }>('connection', async (e) => {
    app.connected = e.payload.connected;
    app.me = e.payload.me;
    app.clientLang = e.payload.language;
    await applyLang();
  });
  listen<Settings>('settings', async (e) => {
    // A pending local edit is newer than the echo of an earlier save.
    if (saveTimer) return;
    app.settings = e.payload;
    await applyPrefs();
  });
  // "navigate": the main window was asked for a page. The page waits in the app until taken, so
  // one asked for while this page was still loading (the app started during champ select) shows too.
  const showAsked = async () => {
    const route = await api.takeMainRoute();
    if (route) go(routeFrom(route));
  };
  await listen('navigate', showAsked);
  await showAsked();
  listen<{ text: string; tone: Toast['tone'] }>('toast', (e) => toast(errorText(e.payload.text), e.payload.tone));
  listen('profile-stale', () => app.profileEpoch++);
  listen<UpdateState>('update', (e) => {
    app.update = e.payload;
  });
  listen<RiotStatus>('riot', (e) => {
    app.riotStatus = e.payload;
  });
}
