import { listen } from '@tauri-apps/api/event';
import { api } from './api';
import { applySettings } from './appearance.svelte';
import { loadStatic } from './dd.svelte';
import { hasKey, i18n, t, type Lang } from './i18n.svelte';
import type { InGame, LiveState, RiotStatus, Settings, Summoner, UpdateState } from './types';

export type Route =
  | { name: 'profile'; puuid?: string }
  | { name: 'live' }
  | { name: 'builds'; championId?: number; role?: string }
  | { name: 'settings' }
  | { name: 'appearance' }
  | { name: 'riotGuide' };

type Toast = { id: number; text: string; tone: 'info' | 'error' | 'good' };

const RELEASES = 'https://github.com/mattcui951/mattofessor/releases';

const emptyLive: LiveState = { phase: '', connected: false, queueId: 0, mode: '', mapId: 0, teams: [] };

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
  ingame: null as InGame | null,
  route: { name: 'profile' } as Route,
  history: [] as Route[],
  toasts: [] as Toast[],
  /** Bumped when the signed-in player's profile should refetch (a game ended). */
  profileEpoch: 0,
});

let toastId = 0;
export function toast(text: string, tone: Toast['tone'] = 'info') {
  const id = ++toastId;
  app.toasts.push({ id, text, tone });
  setTimeout(() => (app.toasts = app.toasts.filter((x) => x.id !== id)), tone === 'error' ? 7000 : 4000);
}

/** Backend errors look like "[code:arg] English text"; known codes are shown translated. */
export function errorText(e: unknown): string {
  const s = typeof e === 'string' ? e : e instanceof Error ? e.message : JSON.stringify(e);
  const m = /^\[([a-z-]+)(?::([^\]]*))?\] (.*)$/s.exec(s);
  if (!m) return s;
  const key = `err.${m[1]}`;
  return hasKey(key) ? t(key, m[2] ?? '') : m[3];
}

export function go(route: Route) {
  app.history.push(app.route);
  if (app.history.length > 30) app.history.shift();
  app.route = route;
}

export function back() {
  app.route = app.history.pop() ?? { name: 'profile' };
}

/** "live", "profile/<puuid>", "builds", "settings" (from the backend). */
export function routeFrom(path: string): Route {
  const [name, arg] = path.replace(/^#?\/?/, '').split('/');
  if (name === 'live') return { name: 'live' };
  if (name === 'builds') return { name: 'builds' };
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
  return s === 'en' || s === 'zh' ? s : app.clientLang;
}

async function applyLang() {
  i18n.lang = uiLang();
  document.documentElement.lang = i18n.lang === 'zh' ? 'zh-CN' : 'en';
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
  listen<string>('navigate', (e) => go(routeFrom(e.payload)));
  listen<{ text: string; tone: Toast['tone'] }>('toast', (e) => toast(errorText(e.payload.text), e.payload.tone));
  listen('profile-stale', () => app.profileEpoch++);
  listen<UpdateState>('update', (e) => {
    app.update = e.payload;
  });
  listen<RiotStatus>('riot', (e) => {
    app.riotStatus = e.payload;
  });
}
