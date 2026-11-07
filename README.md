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
- **Meta**: Summoner's Rift tier lists per lane (win, pick and ban rates) and Hextech ARAM
  champion and augment win rates, by skill bracket.
- **Builds**: runes with stat shards, spells, skill order, starting items, boots and items for
  Summoner's Rift (per lane) and ARAM, imported into the client in one click (or automatically
  when you lock in). In Hextech ARAM the Builds page also ranks the champion's augments.
- **Hextech ARAM augments**: in champ select and in game, the augments to take and to avoid
  for your champion, by rarity. When the augment cards are open, your champion's win rate with
  each card shows above it, the best one marked (read from the screen with Windows' own text
  recognition, English or Chinese; nothing is saved).

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

**Updates:** Mattofessor checks this repository's latest release in the background (no GitHub
account needed) and shows a banner when a new version is out. Click **Update now**, allow the
Windows prompt, and it restarts on the new version; it never updates during a game. Turn the
check off or run it by hand in Settings > Updates.

## Releasing a new version

Bump the version in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`,
commit, then push an annotated tag; its message becomes the release notes:

```bash
git tag -a v0.2.1 -m "What changed, one line per change"
git push origin v0.2.1
```

GitHub Actions (`.github/workflows/release.yml`) runs the tests, builds the installer on
Windows and publishes it as a release, together with `latest.json` and the installer's
signature. Installed copies read `releases/latest/download/latest.json`, so they see the new
version as soon as the release is published.

Updates are signed: the app only installs a download whose signature matches the public key in
`src-tauri/tauri.conf.json`. The private key lives outside the repository
(`%USERPROFILE%\.tauri\mattofessor.key`, password in `mattofessor.key.password`) and in the
repository secrets `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Keep a
backup of both files: without them, existing installs cannot be updated (friends would have to
reinstall by hand). A local `npm run tauri build` needs the same two values as environment
variables.

## Where the data comes from

Everything comes from the signed-in League client's own local API (the "LCU": the lockfile
in the League folder gives its port and password) and the game's Live Client Data API
(`https://127.0.0.1:2999`). Names and icons come from Riot's public Data Dragon CDN. No server of our
own; a Riot API key is optional (see below).

Meta and builds come from two public sources, cached locally so the app keeps working offline:

- **Summoner's Rift** (and classic ARAM builds): OP.GG's public endpoint
  (`https://mcp-api.op.gg/mcp`, [opgginc/opgg-mcp](https://github.com/opgginc/opgg-mcp)): lane
  tier lists and per-champion builds. Tier lists are refreshed every 12 hours, builds daily.
- **Hextech ARAM**: 海克斯大乱斗统计数据 ([czgreat/aramhex-data](https://github.com/czgreat/aramhex-data)),
  licensed [CC BY-NC 4.0](https://creativecommons.org/licenses/by-nc/4.0/): champion win rates
  and the win rate of every champion × augment pair, from millions of games on the CN servers,
  updated about weekly. Downloads are checked against the published SHA-256 sums. The app shows
  this credit wherever these numbers appear, and Mattofessor stays free (non-commercial).
- Augment names and icons: Riot's game data via [CommunityDragon](https://communitydragon.org).

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
| `src-tauri/src/builds.rs` | Builds from OP.GG, rune page and item set import |
| `src-tauri/src/opgg.rs` | OP.GG public endpoint client, its text format parser, tier lists |
| `src-tauri/src/aramhex.rs` | Hextech ARAM statistics (AramHex open data): download, checks, views, augment picks |
| `src-tauri/src/cards.rs` | Augment card reader: finds the offered cards on screen, places the win rates above them |
| `src-tauri/src/ocr.rs` | Windows OCR (Windows.Media.Ocr) wrapper |
| `src-tauri/src/augments.rs` | Augment names, rarity and icons |
| `src-tauri/src/core.rs` | Windows (main, overlay, spell timers), hotkeys |
| `src-tauri/src/store.rs` | SQLite cache in `%APPDATA%\Mattofessor\cache.db` (checked at start; a damaged file is rebuilt from its readable rows) |
| `src/` | Svelte UI; `src/lib/i18n.svelte.ts` holds every string in both languages |

Settings and the log live in `%APPDATA%\Mattofessor`.

## Develop

```bash
npm install
npm run tauri dev
```

Tests: `cargo test` in `src-tauri`, `npx vitest run`, `npx svelte-check`.

Debug builds keep their data in `%APPDATA%\Mattofessor-dev` (and their own WebView data), so a dev run
never opens the installed app's database.

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

`--game <id>` replays a specific game (one of your Hextech ARAM games gives mode `KIWI`),
`--level-every 20` levels up faster and `--deaths` kills you for 15 s of every minute, which
together exercise the augment pick flow. Settings > Augment reader > "Collect augment card
samples" saves the middle of the game window while you are dead, plus the live augment data,
during Hextech ARAM games to `%APPDATA%\Mattofessor\augment-captures` (off by default).
`--screen <image or video>` makes the stand-in game window show it at its own size instead of
a test pattern: a full-size frame with augment cards exercises the card reader, and a video of
saved samples (cards flying in, a reroll) exercises how it follows them. `--at 3440,0` opens the
window there (e.g. on a second monitor). Run the dev app with `MATTOFESSOR_DATA_DIR=<folder>` to
keep a test run's data apart; debug builds log where each card win rate landed (`report:`). To check the reader against saved samples:
`MF_SAMPLES=<capture folder> MF_AUGMENTS=<cherry-augments.json> cargo test --lib scan_samples -- --ignored --nocapture`.
`--summoner-down 60` keeps the signed-in account unanswered for the first 60 s, like a client
that is still starting (e.g. right after switching accounts).

In-game hotkeys use a low-level keyboard hook, because League's game window blocks ordinary
global hotkeys while it has focus. `py -I tools/nohotkeys_window.py` opens a window that blocks
hotkeys the same way and presses Ctrl+Shift+F13 in it (`--check-registerhotkey` shows the
problem with ordinary hotkeys); set that as the overlay hotkey to test during a mock game.
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
