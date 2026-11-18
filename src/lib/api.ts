import { invoke } from '@tauri-apps/api/core';
import type { GameData } from './gamedata.svelte';
import type { AramBracket, AramChampion, AramOverview, AramPicks, Augment, Build, BuildMode, Content, ContentEdit, ContentStatus, FullGame, ImportResult, InGame, ItemOverview, LiveState, MyScore, PlayerScore, Profile, RiftTiers, RiotStatus, Settings, StaticData, Summoner, UpdateState } from './types';

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
  /** From the local cache only (instant, no client needed); null when never looked up. */
  profileSaved: (puuid: string | null) => invoke<Profile | null>('profile_saved', { puuid }),
  /** Rank and mastery now, games later (`profile`): for a player seen for the first time. */
  profileQuick: (puuid: string) => invoke<Profile>('profile_quick', { puuid }),
  search: (riotId: string) => invoke<string>('search', { riotId }),
  game: (gameId: number) => invoke<FullGame>('game', { gameId }),
  gameScores: (gameId: number) => invoke<PlayerScore[]>('game_scores', { gameId }),
  playerScores: (puuid: string, gameIds: number[]) => invoke<MyScore[]>('player_scores', { puuid, gameIds }),
  build: (championId: number, role: string, mode: BuildMode) => invoke<Build | null>('build', { championId, role, mode }),
  importBuild: (championId: number, role: string, mode: BuildMode, runes: boolean, items: boolean) =>
    invoke<ImportResult>('import_build', { championId, role, mode, runes, items }),
  riftTiers: () => invoke<RiftTiers>('rift_tiers'),
  aramOverview: (bracket: AramBracket) => invoke<AramOverview>('aram_overview', { bracket }),
  aramPicks: (championId: number, bracket: AramBracket) => invoke<AramPicks>('aram_picks', { championId, bracket }),
  aramChampion: (championId: number, bracket: AramBracket) => invoke<AramChampion>('aram_champion', { championId, bracket }),
  augmentNames: (lang: string) => invoke<Record<string, Augment>>('augment_names', { lang }),
  saveSettings: (settings: Settings) => invoke<void>('save_settings', { settings }),
  toggleOverlay: () => invoke<void>('toggle_overlay'),
  hideOverlay: () => invoke<void>('hide_overlay'),
  fitOverlay: (height: number) => invoke<void>('fit_overlay', { height }),
  toggleSpells: () => invoke<void>('toggle_spells'),
  hideSpells: () => invoke<void>('hide_spells'),
  fitSpells: (height: number) => invoke<void>('fit_spells', { height }),
  openProfile: (puuid: string) => invoke<void>('open_profile', { puuid }),
  takeMainRoute: () => invoke<string | null>('take_main_route'),
  logError: (message: string) => invoke<void>('log_error', { message }),
  leagueWindowMode: () => invoke<'fullscreen' | 'windowed' | 'borderless' | 'unknown' | null>('league_window_mode'),
  setBorderless: () => invoke<void>('set_borderless'),
  fullscreenTried: () => invoke<boolean>('fullscreen_tried'),
  dismissFullscreenTried: () => invoke<void>('dismiss_fullscreen_tried'),
  openBuild: (championId: number, mode: BuildMode) => invoke<void>('open_build', { championId, mode }),
  content: () => invoke<Content>('content'),
  contentStatus: () => invoke<ContentStatus>('content_status'),
  gameData: () => invoke<GameData>('game_data'),
  editContent: (edit: ContentEdit) => invoke<Content>('edit_content', { edit }),
  publishContent: () => invoke<void>('publish_content'),
  aramItems: (bracket: AramBracket) => invoke<ItemOverview>('aram_items', { bracket }),
  cardReaderStatus: () => invoke<{ ocrEn: boolean; ocrZh: boolean; gameLang: 'en' | 'zh' | null; missing: boolean }>('card_reader_status'),
  setRiotKey: (key: string) => invoke<RiotStatus>('set_riot_key', { key }),
  checkUpdate: () => invoke<UpdateState>('check_update'),
  installUpdate: () => invoke<void>('install_update'),
  captureInfo: () => invoke<{ count: number; dir: string }>('capture_info'),
  openCaptures: () => invoke<void>('open_captures'),
  deleteCaptures: () => invoke<void>('delete_captures'),
  quitApp: () => invoke<void>('quit_app'),
};
