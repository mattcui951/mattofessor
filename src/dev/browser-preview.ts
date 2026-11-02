// Dev-only: lets the UI run in a plain browser tab for visual checks, against the running
// debug app's JSON bridge (src-tauri/src/dev.rs). Open http://localhost:1431/ (main window),
// /#overlay or /#spells. Writes are no-ops except the few the bridge allows.
import { emit } from '@tauri-apps/api/event';
import { mockIPC, mockWindows } from '@tauri-apps/api/mocks';

const BRIDGE = 'http://127.0.0.1:17431';

async function call(path: string, args: Record<string, unknown> = {}) {
  const q = new URLSearchParams(Object.entries(args).filter(([, v]) => v !== null && v !== undefined).map(([k, v]) => [k, String(v)]));
  const r = await fetch(`${BRIDGE}/${path}?${q}`);
  const body = await r.json();
  if (!r.ok) throw body.error ?? 'bridge error';
  return body;
}

const label = location.hash === '#overlay' ? 'overlay' : location.hash === '#spells' ? 'spells' : 'main';
mockWindows(label);
mockIPC(
  async (cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    switch (cmd) {
      case 'bootstrap':
        // The app may be restarting after a rebuild: wait for it.
        for (;;) {
          try {
            return await call(cmd);
          } catch {
            await new Promise((r) => setTimeout(r, 1000));
          }
        }
      case 'live_state':
      case 'crawl_status':
      case 'start_crawl':
      case 'toggle_overlay':
      case 'toggle_spells':
        return call(cmd);
      case 'static_data':
        return call(cmd, { lang: a.lang });
      case 'profile':
        return call(cmd, { puuid: a.puuid });
      case 'search':
        return call(cmd, { riotId: a.riotId });
      case 'game':
        return call(cmd, { gameId: a.gameId });
      case 'game_scores':
        return call('scores', { gameId: a.gameId });
      case 'player_scores':
        return call(cmd, { puuid: a.puuid, gameIds: (a.gameIds as number[]).join(',') });
      case 'build':
        return call(cmd, { championId: a.championId, role: a.role });
      case 'import_build':
        return call(cmd, { championId: a.championId, role: a.role, runes: a.runes, items: a.items });
      case 'set_riot_key':
        return call(cmd, { key: a.key });
      case 'fit_overlay':
      case 'fit_spells':
        console.info('[preview] fit_overlay', a.height);
        return null;
      case 'plugin:event|listen':
        return 0;
      default:
        console.info('[preview] ignored', cmd, a);
        return null;
    }
  },
  { shouldMockEvents: true },
);

// Live updates: poll the bridge and replay them as the app's events.
let last = '';
setInterval(async () => {
  try {
    const s = await call('live_state');
    const text = JSON.stringify(s);
    if (text === last) return;
    last = text;
    await emit('live', s.live);
    if (s.ingame) await emit('ingame', s.ingame);
  } catch {
    /* app not running */
  }
}, 1000);
