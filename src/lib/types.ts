export type Settings = {
  language: 'auto' | 'en' | 'zh';
  hotkeyOverlay: string;
  hotkeySpells: string;
  openInChampSelect: boolean;
  showDuringLoading: boolean;
  spellsOnStart: boolean;
  autoImportRunes: boolean;
  autoImportItems: boolean;
  startWithWindows: boolean;
  checkUpdates: boolean;
  spellsPos: [number, number] | null;
  theme: string;
  customThemes: unknown[];
  uiScale: number;
  aramBracket: AramBracket;
};

export type Summoner = { puuid: string; gameName: string; tagLine: string; level: number; icon: number; summonerId: number };

export type Rank = { queue: 'solo' | 'flex'; tier: string; division: string; lp: number; wins: number; losses: number };
export type TierDiv = { tier: string; division: string };
export type Ranked = { solo: Rank | null; flex: Rank | null; seasonPeak: TierDiv | null; prevPeak: TierDiv | null; prevEnd: TierDiv | null; exact: boolean };

export type UpdateState = {
  phase: 'idle' | 'checking' | 'available' | 'downloading' | 'installing' | 'error';
  version: string | null;
  progress: number | null;
  error: string | null;
  checkedAt: number | null;
};

export type RiotStatus = 'none' | 'unchecked' | 'ok' | 'invalid' | 'unreachable';

export type Tag = { key: string; tone: '' | 'pos' | 'neg' | 'info' | 'duo'; arg?: number | string };

export type ChampStats = {
  championId: number;
  games: number;
  wins: number;
  kills: number;
  deaths: number;
  assists: number;
  masteryLevel: number;
  masteryPoints: number;
};

export type RecentGame = {
  gameId: number;
  championId: number;
  kills: number;
  deaths: number;
  assists: number;
  win: boolean;
  remake: boolean;
  queueId: number;
  created: number;
  placement: number;
};

export type PlayerCard = {
  summoner: Summoner;
  ranked: Ranked;
  games: number;
  wins: number;
  kills: number;
  deaths: number;
  assists: number;
  recent: RecentGame[];
  champ: ChampStats | null;
  topChamps: ChampStats[];
  roles: { role: string; games: number }[];
  tags: Tag[];
  lastGame: number | null;
  smurf: boolean;
  fresh: boolean;
};

export type GameRow = {
  gameId: number;
  created: number;
  duration: number;
  queueId: number;
  mapId: number;
  mode: string;
  championId: number;
  teamId: number;
  win: boolean;
  remake: boolean;
  kills: number;
  deaths: number;
  assists: number;
  cs: number;
  gold: number;
  damage: number;
  taken: number;
  vision: number;
  champLevel: number;
  items: number[];
  spells: [number, number];
  perks: number[];
  perkPrimary: number;
  perkSub: number;
  lane: string;
  placement: number;
  augments: number[];
  largestMultikill: number;
};

export type RankPoint = { day: string; queue: string; tier: string; division: string; lp: number };
export type Mastery = { championId: number; level: number; points: number; lastPlay: number };

export type Profile = { card: PlayerCard; history: GameRow[]; rankHistory: RankPoint[]; masteries: Mastery[]; isMe: boolean; updatedAt: number };

export type Slot = {
  puuid: string;
  championId: number;
  position: string;
  spells: [number, number];
  me: boolean;
  name: string;
  status: 'loading' | 'ready' | 'error' | 'hidden';
  error: string | null;
  card: PlayerCard | null;
};

export type LiveTeam = { id: number; ally: boolean; players: Slot[] };
export type LiveState = { phase: string; connected: boolean; queueId: number; mode: string; mapId: number; teams: LiveTeam[]; bench: number[] };

export type LivePlayer = {
  riotId: string;
  champion: string;
  team: string;
  level: number;
  dead: boolean;
  respawn: number;
  spells: string[];
  items: number[];
  me: boolean;
};
export type InGame = { gameTime: number; mode: string; myTeam: string; players: LivePlayer[] };

export type Champ = { key: string; name: string; alias: string; tags: string[] };
export type Item = { name: string; gold: number; boots: boolean; legendary: boolean };
export type Spell = { key: string; name: string; cooldown: number };
export type Rune = { id: number; name: string; icon: string };
export type Tree = { id: number; name: string; icon: string; slots: Rune[][] };
export type StaticData = {
  version: string;
  locale: string;
  champions: Record<string, Champ>;
  items: Record<string, Item>;
  spells: Record<string, Spell>;
  trees: Tree[];
};

export type Pick<T> = { value: T; games: number; wins: number };
export type BuildMode = 'rift' | 'aram';
export type Build = {
  championId: number;
  mode: BuildMode;
  /** Lane on the Rift; "" in ARAM. */
  role: string;
  games: number;
  wins: number;
  roles: Pick<string>[];
  runes: Pick<{ primary: number; sub: number; perks: number[]; shards: number[] }>[];
  spells: Pick<[number, number]>[];
  starters: Pick<number[]>[];
  boots: Pick<number>[];
  /** Core items first (`core` of them), then the most picked later items. */
  items: Pick<number>[];
  core: number;
  skillMax: string[];
  skillOrder: string[];
};
export type TierRow = {
  championId: number;
  position: string;
  tier: number;
  rank: number;
  games: number;
  winRate: number;
  pickRate: number;
  banRate: number;
  roleRate: number;
};
export type RiftTiers = { rows: TierRow[]; fetchedAt: number };
export type AramBracket = 'all' | 'casual' | 'mid' | 'high';
export type AramSource = { patch: string; snapshotAt: string; matches: number; bracket: string };
export type AramChampRow = { championId: number; games: number; winRate: number; pickRate: number; rank: number };
export type AugRow = { id: number; games: number; winRate: number; smoothed: number; delta: number; lowSample: boolean };
export type AramOverview = { source: AramSource; champions: AramChampRow[]; augments: AugRow[] };
export type AramChampion = { source: AramSource; champion: AramChampRow | null; augments: AugRow[] };
export type Augment = { name: string; rarity: 'silver' | 'gold' | 'prismatic' | ''; icon: string };
export type PlayerScore = { participantId: number; puuid: string; score: number; badge: '' | 'mvp' | 'svp'; place: number };
export type MyScore = { gameId: number; score: number; badge: '' | 'mvp' | 'svp'; place: number; players: number };
export type ImportResult = { runePage: string | null; itemSet: string | null };

/** Full game from the client (`/lol-match-history/v1/games/{id}`), the parts we read. */
export type FullGame = {
  gameId: number;
  gameDuration: number;
  queueId: number;
  gameMode: string;
  participantIdentities: { participantId: number; player: { puuid: string; gameName: string; tagLine: string } }[];
  participants: {
    participantId: number;
    championId: number;
    teamId: number;
    spell1Id: number;
    spell2Id: number;
    stats: Record<string, number | boolean>;
  }[];
  teams: { teamId: number; win: string | boolean }[];
};
