import { invoke } from '@tauri-apps/api/core';
import type { Build, CrawlStatus, FullGame, ImportResult, InGame, LiveState, MyScore, PlayerScore, Profile, RiotStatus, Settings, StaticData, Summoner, UpdateState } from './types';

export type Boot = {
  settings: Settings;
  version: string;
  connected: boolean;
  me: Summoner | null;
  language: 'en' | 'zh';
  live: LiveState;
  ingame: InGame | null;
  riotStatus: RiotStatus;
  update: UpdateState;
  updatedFrom: string | null;
};

export const api = {
  bootstrap: () => invoke<Boot>('bootstrap'),
  staticData: (lang: string) => invoke<StaticData>('static_data', { lang }),
  profile: (puuid?: string) => invoke<Profile>('profile', { puuid: puuid ?? null }),
  search: (riotId: string) => invoke<string>('search', { riotId }),
  game: (gameId: number) => invoke<FullGame>('game', { gameId }),
  gameScores: (gameId: number) => invoke<PlayerScore[]>('game_scores', { gameId }),
  playerScores: (puuid: string, gameIds: number[]) => invoke<MyScore[]>('player_scores', { puuid, gameIds }),
  liveState: () => invoke<{ live: LiveState; ingame: InGame | null }>('live_state'),
  build: (championId: number, role: string) => invoke<Build | null>('build', { championId, role }),
  importBuild: (championId: number, role: string, runes: boolean, items: boolean) =>
    invoke<ImportResult>('import_build', { championId, role, runes, items }),
  crawlStatus: () => invoke<CrawlStatus>('crawl_status'),
  startCrawl: () => invoke<void>('start_crawl'),
  saveSettings: (settings: Settings) => invoke<void>('save_settings', { settings }),
  toggleOverlay: () => invoke<void>('toggle_overlay'),
  hideOverlay: () => invoke<void>('hide_overlay'),
  fitOverlay: (height: number) => invoke<void>('fit_overlay', { height }),
  toggleSpells: () => invoke<void>('toggle_spells'),
  hideSpells: () => invoke<void>('hide_spells'),
  fitSpells: (height: number) => invoke<void>('fit_spells', { height }),
  openProfile: (puuid: string) => invoke<void>('open_profile', { puuid }),
  setRiotKey: (key: string) => invoke<RiotStatus>('set_riot_key', { key }),
  checkUpdate: () => invoke<UpdateState>('check_update'),
  installUpdate: () => invoke<void>('install_update'),
  quitApp: () => invoke<void>('quit_app'),
};
