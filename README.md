# Mattofessor

A local Porofessor replacement for League of Legends on Windows, without Overwolf, ads or
accounts. English and Simplified Chinese.

- **Profile**: your (or any Riot ID's) solo and flex rank, past ranks, tags, most played
  champions, roles, mastery and match history with full 10-player game details.
- **Live game**: in champ select it scouts your team; once the game starts, everyone (Arena
  included). Each player: rank and peak rank, possible smurf, games / win rate / KDA / mastery
  on their champion, last 5 games' KDA, and tags (one-trick, main role, off-role, farmer,
  fighter, premade...).
- **In-game panel**: `Ctrl+Shift+M` shows / hides it over the game (chosen so it can run beside Porofessor, which uses `Ctrl+X`); it also shows by itself on the
  loading screen and hides when the match starts. `Ctrl+Shift+N` toggles enemy summoner spell
  timers (click a spell when it is used). Both hotkeys exist only while a game runs.
- **Builds**: runes, spells, boots and items from Challenger and Grandmaster solo queue games,
  imported into the client in one click (or automatically when you lock in).

- **Scores**: every player in every game gets a 0.0-10.0 performance score and a place (1st to
  10th); the best on the winning team is MVP, the best on the losing team SVP.

## Install (for players)

1. Open this repository's **Releases** page (right side of the GitHub page) and download
   `Mattofessor_x.y.z_x64-setup.exe` from the latest release. (Do not use "Download ZIP":
   that is the source code.)
2. Run it. Windows may show "Windows protected your PC" because the installer is not
   code-signed: click **More info**, then **Run anyway**. It asks for administrator rights and
   installs to `C:\Program Files\Mattofessor` with a desktop shortcut.
3. Start League of Legends and sign in; Mattofessor connects by itself and starts with Windows
   (turn that off in Settings).
4. Optional: get your own free Riot API key for exact ranked wins and losses: Settings > Riot
   API key > **Step-by-step guide with pictures**. Each person uses their own key; never share
   yours.

Needs Windows 10 or 11 (WebView2 is installed automatically if missing). League must run in
**Borderless** or **Windowed** mode for the in-game panel to show over the game. Hotkeys:
`Ctrl+Shift+M` panel, `Ctrl+Shift+N` enemy spell timers (changeable in Settings).
Uninstall from Windows Settings > Apps; settings and cache live in `%APPDATA%\Mattofessor`.

## Releasing a new version

Bump the version in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`,
commit, then push a tag:

```bash
git tag v0.1.1
git push origin v0.1.1
```

GitHub Actions (`.github/workflows/release.yml`) runs the tests, builds the installer on
Windows and publishes it as a release.

## Where the data comes from

Everything comes from the signed-in League client's own local API (the "LCU": the lockfile
in the League folder gives its port and password) and the game's Live Client Data API
(`https://127.0.0.1:2999`). Names and icons come from Riot's public Data Dragon CDN. No server of our
own; a Riot API key is optional (see below).

Porofessor itself shows a page from its own servers (`porofessor.gg/overwolf/live...`) built on
a Riot production key; those endpoints are private, so this app does not use them.

Limits of the client API, found by testing against the live client:

- Other players' ranked **losses are hidden** (always 0), so ranked win rates exist only for
  you; win rates for others come from their match history.
- Other players' history returns their **last 40 games** (paging is ignored); the first
  lookup of a player per client session takes about 5 s and lookups run one at a time, so
  a player first shows rank and mastery, then their games.
- **Past ranks**: the client knows only last season's final and peak rank. The app also
  records a rank snapshot (one per player, queue and day) every time it looks someone up,
  which becomes your rank history over time.
- **Lane labels are unreliable**, so roles come from smite, support items and Riot's
  recommended positions per champion (see `src-tauri/src/positions.rs`).
- Game timelines have **no item purchase order**: builds list items by popularity.

## Appearance

Settings > Appearance: 25 built-in themes (Arcade, Rosé Pine / Moon / Dawn, Catppuccin Mocha /
Macchiato / Frappé / Latte, Tokyo Night / Storm / Day, Dracula, Nord, Gruvbox Dark / Light,
One Dark, Kanagawa, Everforest Dark / Light, Solarized Dark / Light, GitHub Dark / Light,
High Contrast Dark / Light), plus your own: copy any theme and edit every color, the two
fonts, corner roundness, in-game window opacity and light/dark; export / import themes as
text. Text size applies to every window, in-game panel included. Palettes follow each theme's
published colors; "Keep text readable" nudges faint text and accents to WCAG contrast
(tested for every built-in theme in `src/lib/__tests__/themes.test.ts`). Tokens and themes
live in `src/lib/themes.ts`.

## Optional Riot API key

Settings > Riot API key fills in what the client hides: exact ranked wins and losses (and so
win rates) for every player, which also sharpens the smurf check. Get a key at
[developer.riotgames.com](https://developer.riotgames.com/) after signing in with a Riot
account: the Development API Key on the dashboard works immediately but stops after 24 hours;
for a lasting one, Register Product > Personal API Key (personal use only; 20 requests per
second, 100 per 2 minutes). Each player costs two requests (Riot ID to this key's PUUID, then
ranked entries), throttled under those limits. The key is stored with Windows DPAPI in
`%APPDATA%\Mattofessor\riot-key.bin` and never reaches the UI (`src-tauri/src/riot.rs`).

## Layout

| Path | What |
|---|---|
| `src-tauri/src/lcu.rs` | League client API: lockfile discovery, requests |
| `src-tauri/src/game.rs` | Game flow watcher, champ select / in-game rosters, scouting |
| `src-tauri/src/analysis.rs` | Player summary and tag rules (unit tested) |
| `src-tauri/src/positions.rs` | Role inference |
| `src-tauri/src/builds.rs` | High-elo crawler, build aggregation, rune page and item set import |
| `src-tauri/src/core.rs` | Windows (main, overlay, spell timers), hotkeys |
| `src-tauri/src/store.rs` | SQLite cache in `%APPDATA%\Mattofessor\cache.db` |
| `src/` | Svelte UI; `src/lib/i18n.svelte.ts` holds every string in both languages |

Settings and the log live in `%APPDATA%\Mattofessor`.

## Develop

```bash
npm install
npm run tauri dev
```

Tests: `cargo test` in `src-tauri`, `npx vitest run`, `npx svelte-check`.

Debug builds serve a JSON bridge on `http://127.0.0.1:17431`; with the dev server running,
`http://localhost:1431/` (`#overlay`, `#spells`) renders the real UI in a browser with live data.

### Test a game without playing

```bash
py -I tools/mock_client.py --champ-select 30 --length 600
```

Then run the app with `tools/dev-mock.cmd`. The mock replays one of your past Summoner's Rift
games as if it were happening now: a stand-in client (real player lookups are forwarded to the
running League client), a Live Client API with a game clock, and a stand-in game window
named `Mattofessor Mock Game.exe` (so other League tools ignore it). Rune page and item set
writes are recorded in `tools/.mock/writes.log` instead of changing your account.
`tools/inspect_windows.py` prints the app's windows (position, topmost, focus).

## Build

```bash
npm run tauri build
```

Produces an NSIS installer that installs for all users under `C:\Program Files\Mattofessor`.

## Legal

Mattofessor isn't endorsed by Riot Games and doesn't reflect the views or opinions of Riot Games
or anyone officially involved in producing or managing Riot Games properties. Riot Games, and
all associated properties are trademarks or registered trademarks of Riot Games, Inc.
