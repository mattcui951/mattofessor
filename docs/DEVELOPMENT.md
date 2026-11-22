# Developing Mattofessor

For contributors: how the app is laid out, where its data comes from in detail, how to run, test, build and release it. What players see is in the [README](../README.md).

## Layout

| Path | What |
|---|---|
| `src-tauri/src/lcu.rs` | League client API: lockfile discovery, requests |
| `src-tauri/src/game.rs` | Game flow watcher, champ select / in-game rosters, scouting |
| `src-tauri/src/analysis.rs` | Player summary and tag rules (unit tested) |
| `src-tauri/src/positions.rs` | Role inference |
| `src-tauri/src/builds.rs` | Builds from OP.GG, rune page and item set import |
| `src-tauri/src/opgg.rs` | OP.GG public endpoint client, its text format parser, tier lists |
| `src-tauri/src/aramhex.rs` | Hextech ARAM statistics (AramHex open data): download, checks, views, augment picks, item win rates |
| `src-tauri/src/content.rs` | The admin's builds and tier lists: following the `content` branch, editing, publishing with `gh` |
| `src-tauri/src/data.rs` | Refreshed Hextech ARAM modifiers, rules and patch notes: following the `data` branch |
| `src-tauri/src/branch.rs` | Reading a branch of this repository at its latest commit |
| `tools/refresh-data/` | Builds the `data` branch (run by `.github/workflows/refresh-data.yml`) |
| `src-tauri/src/cards.rs` | Augment card reader: finds the offered cards on screen, places the win rates above them |
| `src-tauri/src/ocr/` | Text recognition: Windows.Media.Ocr on Windows, Vision on macOS |
| `src-tauri/src/os/` | The system: processes, other apps' windows, the in-game windows over the game, screen capture (Win32 on Windows; libproc, Quartz, AppKit panels and ScreenCaptureKit on macOS) |
| `src-tauri/src/hotkeys/` | In-game hotkeys: a keyboard hook on Windows, system hotkeys on macOS |
| `src-tauri/src/augments.rs` | Augment names, rarity and icons |
| `src-tauri/src/core.rs` | The app's windows (main, overlay, spell timers), hotkeys |
| `src-tauri/src/store.rs` | SQLite cache in `%APPDATA%\Mattofessor\cache.db` (checked at start; a damaged file is rebuilt from its readable rows) |
| `src/` | Svelte UI; `src/lib/i18n.svelte.ts` holds every string in both languages |

Settings and the log live in `%APPDATA%\Mattofessor` (Windows) or
`~/Library/Application Support/Mattofessor` (macOS).

## Develop

```bash
npm install
npm run tauri dev
```

Tests: `cargo test` in `src-tauri`, `npx vitest run`, `npx svelte-check`.

Debug builds keep their data in `%APPDATA%\Mattofessor-dev` (macOS:
`~/Library/Application Support/Mattofessor-dev`; their own WebView data and keychain item too),
so a dev run never opens the installed app's database.

Debug builds serve a JSON bridge on `http://127.0.0.1:17431`; with the dev server running,
`http://localhost:1431/` (`#overlay`, `#spells`) renders the real UI in a browser with live data.

### Test a game without playing

```bash
py -I tools/mock_client.py --champ-select 30 --length 600          # Windows
/usr/bin/python3 -I tools/mock_client.py --champ-select 30 --length 600   # macOS
```

Then run the app with `tools/dev-mock.cmd` (macOS: `tools/dev-mock.sh`). On macOS use a Python
that trusts the system's certificates (`/usr/bin/python3`; the python.org build needs its
"Install Certificates" step first). The mock replays one of your past Summoner's Rift
games as if it were happening now: a stand-in client (real player lookups are forwarded to the
running League client), a Live Client API with a game clock, and a stand-in game window
named `Mattofessor Mock Game(.exe)` (so other League tools ignore it). Rune page and item set
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
keep a test run's data apart; debug builds log where each card win rate landed (`report:`).
`MATTOFESSOR_DEVTOOLS_PORT=<port>` (debug builds) opens the real WebView2 windows to DevTools on
that port, so a script can send keys and mouse wheel through the browser's own input path and
read the page (e.g. to check zoom). To check the reader against saved samples:
`MF_SAMPLES=<capture folder> MF_AUGMENTS=<cherry-augments.json> cargo test --lib scan_samples -- --ignored --nocapture`
(add `MF_LINES=1` to print every line the OCR read). Frames from a game video work too: cut them
to the same band, e.g. `ffmpeg -i match.mp4 -vf "fps=1,scale=3440:1440,crop=3440:1094:0:172" -pix_fmt rgba frames/t%06d.png`.
`--summoner-down 60` keeps the signed-in account unanswered for the first 60 s, like a client
that is still starting (e.g. right after switching accounts).
`--history-lag 120` acts like the real client after a game: your match history answers 500 for
the first quarter, leaves the game out until 120 s after it ends, and the full game is readable
from half way (the mock stays up that long). Use it with a fresh `MATTOFESSOR_DATA_DIR`, so the
game is not already saved, to see the profile pick up the finished game by itself.

On Windows the in-game hotkeys use a low-level keyboard hook, because League's game window
blocks ordinary global hotkeys while it has focus (macOS uses system hotkeys).
`py -I tools/nohotkeys_window.py` opens a window that blocks hotkeys the same way and presses Ctrl+Shift+F13 in it (`--check-registerhotkey` shows the
problem with ordinary hotkeys); set that as the overlay hotkey to test during a mock game.
`tools/inspect_windows.py` prints the app's windows (position, topmost, focus).

## Build

```bash
npm run tauri build
```

On Windows this produces an NSIS installer that installs for all users under
`C:\Program Files\Mattofessor`. On macOS it produces `Mattofessor.app` and a dmg
(`src-tauri/tauri.macos.conf.json`; add `--target universal-apple-darwin` for Apple Silicon and
Intel in one, as releases do). It is signed with the Developer ID certificate named in that file
(it must be in your keychain) and notarized when `APPLE_ID`, `APPLE_PASSWORD` (an app-specific
password) and `APPLE_TEAM_ID` are set.

## Releasing a new version

Bump the version in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`,
commit, then push an annotated tag; its message becomes the release notes:

```bash
git tag -a v0.2.1 -m "What changed, one line per change"
git push origin v0.2.1
```

GitHub Actions (`.github/workflows/release.yml`) runs the tests and builds on Windows (the
installer) and macOS (a universal app and dmg), attaches both to a draft release, then writes
`latest.json` from both builds' update signatures and publishes it. Installed copies read `releases/latest/download/latest.json`, so they see the new
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

- **Summoner's Rift** (and the skill order and starting items of Hextech ARAM builds, from classic ARAM): OP.GG's public endpoint
  (`https://mcp-api.op.gg/mcp`, [opgginc/opgg-mcp](https://github.com/opgginc/opgg-mcp)): lane
  tier lists and per-champion builds. Tier lists are refreshed every 12 hours, builds daily.
- **Hextech ARAM**: 海克斯大乱斗统计数据 ([czgreat/aramhex-data](https://github.com/czgreat/aramhex-data)),
  licensed [CC BY-NC 4.0](https://creativecommons.org/licenses/by-nc/4.0/): champion win rates
  and the win rate of every champion × augment pair, from millions of games on the CN servers,
  updated about weekly, plus item statistics from 16.22 on: finished items, boots, summoner
  spells and three-item builds (the Hextech ARAM builds, the champion page, the item win rates).
  Earlier patches come from its `index.json` (patch picker, win rate by patch).
  Downloads are checked against the published SHA-256 sums. The app shows this credit wherever
  these numbers appear, and Mattofessor stays free (non-commercial).
- **Hand-written content**: the admin's Hextech ARAM builds and the item and champion tier lists,
  in `content.json` on this repository's `content` branch. Every app checks it every 3 minutes
  (the branch's latest commit through GitHub's API, then the file at that commit) and starts from
  the copy it shipped with (`src-tauri/data/content.json`). The admin edits it in the app
  (Settings > Admin), which publishes through the GitHub CLI (`gh`). Only a copy whose `gh`
  account can push to this repository shows that section or accepts edits; at start it asks
  GitHub (`repos/<repo>` `.permissions.push`).
- **Refreshed data** (`src-tauri/src/data.rs`), on this repository's `data` branch, written every 6
  hours by `.github/workflows/refresh-data.yml` (`tools/refresh-data`, Node + headless Chromium):
  - `mayhem.json`: each champion's Hextech ARAM modifiers and ability changes from the
    [League of Legends Wiki](https://wiki.leagueoflegends.com/en-us/ARAM:_Mayhem) (CC BY-SA 3.0):
    the ARAM page's mode overrides with the ARAM: Mayhem page's layer on top. The wiki answers
    scripts with a challenge page, hence the browser. Plus the mode's rules and headliner bands,
    written by hand in both languages (`tools/refresh-data/rules.json`); when the wiki's text of
    those sections changes, the run opens an issue labelled `data-rules`. Melee or ranged comes
    from CommunityDragon's per-champion `attackType`. And every augment's description with its
    numbers, in both languages (`tools/refresh-data/augments.mjs`): the text from the game's string
    tables, the values and formulas from the mode's spell data (CommunityDragon's `kiwi.bin.json`).
  - `patch-notes.json`: the latest 5 patches, Summoner's Rift sections (champions, items, ...)
    and ARAM: Mayhem apart, with champion, item and augment ids: Riot's notes for English and
    Tencent's official Chinese notes (lol.qq.com) for Chinese.
  - Chinese never mixes in English. Names come from the game's own Chinese data (Data Dragon,
    CommunityDragon's `cherry-augments.json` and `lol.stringtable.json`, which also give the
    headliners' official names and texts). The wiki's ability lines have no official Chinese:
    `tools/refresh-data/zh-lines.json` holds reviewed translations; new lines are translated by
    Claude (`translate.mjs`) when the `ANTHROPIC_API_KEY` repository secret is set, else the run
    opens a `data-translations` issue and the app leaves those lines out in Chinese.
  Apps check the branch every 30 minutes (`src-tauri/src/branch.rs`, shared with `content`) and
  start from the copy they shipped with (`src-tauri/data/`), whichever was built later. To try
  it: `cd tools/refresh-data && npm ci && npx playwright install chromium && node refresh.mjs`.
- Augment names and icons: Riot's game data via [CommunityDragon](https://communitydragon.org).
- Item stats and effects (hover cards): Data Dragon's item descriptions, in the UI language.
- Item roles (the shop's Fighter, Marksman, ... filter, `mItemAttributes`): CommunityDragon's
  `items.cdtb.bin.json` for the patch (about 16 MB, read once per patch; only the roles are kept).

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

## Adding a language

1. `src/lib/locales.ts` and `src-tauri/src/lang.rs`: one entry each (a test checks they match):
   Data Dragon locale, CommunityDragon folder, OCR tag, League client locales, font.
2. `src/lib/locales/<id>.ts`: the dictionary, every key of `en.ts` (a missing key is a type error,
   a test checks the `{0}` placeholders), added to `dicts` in `src/lib/i18n.svelte.ts`.
3. `tools/refresh-data/`: the language in `LANGS` and `RIOT_LOCALES` (refresh.mjs), its wording in
   `WORDS` and `STATS` (augments.mjs), and its text in every entry of `rules.json`.
4. `README.<id>.md` (`README.zh-CN.md` for Simplified Chinese): the players' part of the README
   in that language, and the language in the bar at the top of every README (imported by name in
   `src/lib/__tests__/readme.test.ts`, which checks there is one per language and each bar lists
   every language in the registry's order).

## Appearance

Zoom lives in `src/lib/zoom.svelte.ts`. Contrast is tested for every built-in theme in `src/lib/__tests__/themes.test.ts`; tokens and themes live in `src/lib/themes.ts`.

## Riot API key storage

The key is stored with Windows DPAPI in
`%APPDATA%\Mattofessor\riot-key.bin`, or in the login keychain on macOS, and never reaches the UI
(`src-tauri/src/riot.rs`, `src-tauri/src/secret.rs`).
