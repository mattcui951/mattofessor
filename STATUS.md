# Status

Updated: 2026-11-18
Version: 1.0.13

## In progress

- Card reader (`cards.rs`, `ocr.rs`, `overlay/Cards.svelte`), fixed in 1.0.1 after the first
  real game showed two labels in the wrong places with stale numbers. Three causes, all fixed:
  - The `cards` window was missing from `capabilities/default.json`, so it could not listen for
    updates: it kept the first offer it loaded for the whole pick (rerolls and later picks too).
  - It read Mattofessor's own in-game panel (open by itself during loading; Hextech's first pick
    comes right at the start). The panel lists augments in three columns 1.25x wider apart than
    the cards. The reader now waits while the panel is open.
  - Labels show only once two reads in a row agree (cards fly in when they open).
  - "Best" needs at least two cards with enough games.
  - Verified with `mock_client.py --screen <video> --at 3440,0`: a replay of two real picks from
    the saved samples (fly-in frames, a reroll), old build reproduces stale labels, new build
    follows the cards. Debug builds log where each label landed (`report:` lines).
  - Real games with 1.0.3: labels follow rerolls when dead (20:27 game). At a first pick (7 Nov,
    Hecarim) Windows OCR cut two titles into two lines each: "Clown College" got no label and
    "Recursion" (another augment) showed at the right half of "Infinite Recursion". Fixed
    (1.0.4): titles are now joined from the OCR's words (same row, gaps under 3 word
    heights), junk of 1-2 letters around a title is left out, and a name inside other words does
    not count. The live split could not be made to happen on recorded frames; the tests rebuild
    it from the numbers shown. Rescanning today's six games' samples gives the same cards.
  - Every change of the shown labels is now logged (`card win rates shown: id@x ...`), every
    read too in debug builds, so the next odd pick can be traced from `app.log`.
  - Not verified: the Chinese client, other HUD scales and window sizes.

## Next

- 1.0.9: the first augment pick of a game took 10+ s on a friend's PC
  (rerolls fast; OCR is ~50 ms a read in release). At the start of a Hextech game the card window is
  now made (hidden) and the bracket numbers loaded (`cards::prepare`); OCR engines are cached; the
  game's language is read first; reads every 350 ms (at least twice a read's time) and new cards
  are confirmed 150 ms later; the loading-screen panel closes at game time 0.5 s. Measured
  2026-11-15 (debug build, the friend's Chinese frame in a stand-in game window): cards on screen
  to win rates ready 0.72-0.87 s, ~0.3 s after first seen. Each shown offer logs that time.
  Dev: `/demo_game` now holds until a restart (`game::DEMO`), so a stand-in game window (the
  mock's ffplay copy with MATTOFESSOR_GAME_EXE) can be read without the client.
- Not verified yet (2026-11-15, needs the real client or a real game):
  - "Switch to Borderless" (`display::set_borderless`: PATCH `/lol-game-settings/v1/game-settings`
    `{"General":{"WindowMode":2}}`, then POST `/lol-game-settings/v1/save`). Reading the mode from
    `Config/game.cfg` was confirmed on a real Full screen game (log line at game start).
  - Ctrl+X during a real champ select (the panel with the ARAM picks), and that the keyboard hook
    there does not get in the way of the client's chat.
  - The Full screen hold-back with a real key press, and the notice after that game.
  - The card reader's new log lines on a Full screen game ("captures as black") and on a PC without
    Chinese OCR. A friend's PC shows no card win rates at all: get his app.log after updating.
  - A rare lane on the Builds page (e.g. Orianna jungle: OP.GG has 1 to 3 games) and its note.
- Off-meta picks below OP.GG's ~10,000-game lane cutoff (Orianna or Aurelion Sol jungle) can only
  be found by asking OP.GG for every champion and lane (~850 calls); not done. The Builds page
  offers every lane on a build instead.
- Watch the card reader in the next games (the log now shows each change). The augment sample
  capture can go once the reader is confirmed (one game made 824 PNGs, 3.4 GB); Matt Game
  Recorder's videos (F:\Matt Game Recorder) are a better source: whole picks, alive too.
- `store.rs`: `rows()` drops read errors, so a damaged database shows an empty history marked
  "updated just now"; return fetched rows even when the cache fails, and rebuild the cache as
  soon as a query reports corruption (not only at start).
- A sandboxed test run (Claude's shell) sees an overlay of the real %APPDATA%, so it shares
  `Mattofessor-dev` with dev runs started by App HQ; set `MATTOFESSOR_DATA_DIR` for test runs.
- Hide the card win rates sooner after a pick (now about 1 s after the cards close).
- Hextech ARAM items from Tencent (101.qq.com). No permission rule or Referer is needed: plain
  GETs to https://mlol.qt.qq.com/go/battle_info/odp_proxy/<name> work.
  `fuwen_hero_rank?championid=N`: one champion's boots, core, full builds, starters, skills (rates
  only, no game counts) and augment pick rates with an S-C grade (no augment win rate).
  `fuwen_aram_hero_rank_v2?dtstatdate=YYYYMMDD`: champion win/pick rates.
  `fuwen_aram_rune_rank_v2?augmentid_level=255`: augment win/pick rates.
  Daily, same population as AramHex (173 champions, Ahri 52.3% vs AramHex 52.2%).
- Recommendations use AramHex (only source with per-champion augment win rates and game counts);
  Tencent is the daily source for items and overall rates.

## Known issues

- Profile page at a small window with a large text size (history column under about 480 px wide,
  e.g. the smallest window at 140%): the collapsed game rows are cut off on the right. The fixed
  width of the left column is the cause.
- Without a Riot API key, other players' ranked losses are unknown (an LCU limit); the UI says so.
- Match history from the LCU is limited to 40 games and the first lookup takes about 5 s.

## Recently done

- Not released yet (after 1.0.13):
  - Hextech ARAM redesign from the design review (`.lavish/ui-redesign/`, crawled haidoubang.com
    and arammeta.com; data stays AramHex, CN, ~10x their samples). The user picked 17 of 22
    features; skipped the five that need arammeta (TW) data: the companion-item comparison, early
    or late game, augment pools, early data of a new patch, team win estimate in champ select.
    - `hexstats.rs`: champion board, augment board (lift over each champion, augments removed
      since the patch before), champion x augment pairs, one champion's page (augments, items with
      the rate without them, boots incl. none, spells, three-item routes, brackets, patches).
      `aramhex.rs` reads `index.json` for 16.19-16.21 (kept in aramhex/h-<patch>) and caches
      parsed files (40).
    - Hextech ARAM builds (and the item set import) take spells, boots and items from AramHex;
      skill order and starters stay OP.GG classic ARAM (`item_source`).
    - Builds page, Hextech mode: sortable stat card rows (win / Δ / pick), 12 augments per rarity,
      build styles and routes, boots and spells, items plus "common but worse" (4%+ pick, 2.5+
      points under), hover cards (`HoverLayer`), recent changes with the champion's numbers,
      bracket bars, patch line, augment picks by bracket.
    - Meta page, Hextech: 英雄榜 (class x tier board, ▲▼ vs the patch before, by best build,
      list), 海克斯榜 (rarity x lift board, removed list), 组合榜, 装备榜; shared patch and bracket
      filter (`HexFilter`). Champion tiers moved there (user's pick); the tier page keeps items.
    - Search box: augments (open their pairs), Ctrl K. Tier lines 53.5/51.5/50/48 everywhere.
    - Fixed on the way: a data set replaced on GitHub since the app last checked made every file
      not downloaded yet fail its checksum (seen live with `hero_spells.json`); `aramhex::fresh`
      switches to the new set and tries once more.
    - Verified in the browser preview against the debug app (zh and en, 1440 and 1280 wide, 16.22
      and 16.21). Not verified: the Tauri window itself, importing the new item set into a client
      (mock), Ctrl K inside the app window.
  - Lost per-bracket champion tiers: the 1.0.12 app re-published content without the key it did
    not know. Republished (all three brackets, same cut-offs); `Content` now passes unknown
    fields on (`other`, serde flatten), so an older copy can no longer drop newer data.
  - Rail: icon plus a short label per page (rail.* strings), settings labelled at the bottom.
  - Back and forward: title bar buttons, mouse side buttons, Alt + Left / Right
    (`state.svelte.ts` history and future). Picking a champion or mode on the builds page is a
    step. Guide, tier lists, patch notes and Meta keep their tabs and filters across visits
    (module state); the guide also its search, open row and scroll.
  - Champion search everywhere through one index (`champFind.svelte.ts`): both languages,
    pinyin, Tencent's hero list keywords (`keywords.rs`, nicknames like 大嘴, cached a day) and a
    typo fallback for 4+ letters ("cogmo"). Tier pool and guide table now use it too.
  - Guide champion table: class filter, the ability changes split into Hextech ARAM-only and ARAM
    columns that expand to the lines.
- 1.0.13: Hextech ARAM guide, champion adjustments and patch notes, kept
  current by a scheduled job (design reviewed in `.lavish/mayhem-guide-design.html`; the user
  picked the recommended options: two rail pages, ability change lines in English with Chinese
  names, structured patch notes plus a link).
  - Job: `tools/refresh-data/refresh.mjs` (Playwright 1.64.1) + `.github/workflows/refresh-data.yml`
    (every 6 h) write the `data` branch; the app follows it (`data.rs`, `branch.rs`, every 30 min)
    and ships a copy in `src-tauri/data/`. Strict: unknown wiki lines or too few rows fail the run.
  - The Mayhem page's "changed to X from Y" gives the final X; its "from" is often stale (about 20
    differ from today's ARAM list, logged as notes), so the regular ARAM value shown is the ARAM
    list's. A Mayhem ability line replaces the ARAM line about the same thing.
  - Pages: 海克斯大乱斗简介 (rules, bands, champion table), 版本更新 (Rift / Hextech ARAM, 5
    patches, localized champion, item, augment and ability names, buff/nerf tags for modifier
    lines), and a 海克斯大乱斗调整 card on each champion's Hextech ARAM page (own modifiers, the
    ARAM value where Mayhem changed it, melee/ranged bonus from the rules, ability changes).
  - Verified in the dev app with the shipped data (zh). Not verified: the workflow on GitHub (the
    wiki may challenge GitHub's runners harder; fallback is running the same script on this PC),
    and the app reading the `data` branch (it does not exist until the workflow first runs).
  - One language at a time (user's rule: no English in Chinese, official translations first).
    Chinese patch notes are Tencent's (lol.qq.com list API target 24, GBK pages, parsed like
    Riot's; 26.18 to 26.22 now, China lags Riot by a patch). Names and band texts come from the
    game's string tables (Bit Rush 比特狂飙, Debonair 执事乐队 from their voice lines; La Ilusión
    and Maestro have none: 魂蔷薇乐队 and 大师乐队 marked 暂译). Rule terms follow the game's
    wording (连招破坏者, 坚强壁垒, 属性锻造器, 治疗圣物, 外/水晶/枢纽防御塔, 强化符文). The 142
    wiki ability lines are translated by hand in `zh-lines.json`; new ones need the
    ANTHROPIC_API_KEY secret (Claude Opus 5.5, structured output, refusal fallback; untested,
    no key here) or a hand translation. Verified in the dev app: no English words in Chinese
    pages and no Chinese in English pages (DOM scan).
- 1.0.13: champion tiers per skill bracket (娱乐, 中分, 前 5%; AramHex
  bands, no all-players list). Content key `championTiersByBracket` (bracket to class to tiers);
  the old `championTiers` (all players) is passed on untouched for apps before 1.0.13 and can be
  dropped once they are gone. The tier page's champion view has a bracket picker shared with the
  Meta page's (`aramBracket`; "all" shows 中分). Published: every bracket ranked by its own win
  rates with the same cut-offs (53.5% S, 51.5% A, 50% B, 48% C). The Hextech ARAM build page shows
  the champion's win rate in all four brackets (全体, 娱乐, 中分, 前 5%; games on hover; a click
  picks the bracket). Items: AramHex has item stats for all players and 前 5% only, so item tiers
  stay one list.
- 1.0.12: items by class. Every item gets the shop's roles from
  CommunityDragon's game data (`dd.rs` `roles`, cached as `dd/<patch>-roles.json`; Data Dragon
  cache renamed `-5.json`, not written when the roles fail so the next start retries). Tier list
  items have a class bar (all, Fighter, Assassin, Mage, Tank, Support, Marksman, Other) over the
  one ranking; editing in a class keeps the item's place among the hidden ones (verified: drag
  and key 1 in Fighter). Other: `OTHER_ITEMS` (Guardian items, small anti-heal items, the
  augment-only Void Immolation, Ultra Hydra, Wooglet's Witchcap, Golden Spatula), components and
  roleless items. The Meta item win rates have the same filter (no Other: all finished items have
  a role). The page and its rail button say "Hextech ARAM tier lists" (海克斯大乱斗梯队榜).
  Published content: champion tiers for every class ranked by Hextech ARAM win rate (all players,
  16.22): 53.5%+ S, 51.5%+ A, 50%+ B, 48%+ C, below D. The user's "2 Sunfire" item was taken to be
  Void Immolation (223069); unconfirmed.
- 1.0.11: only the admin sees the Admin setting and the editing tools.
  At start the app asks GitHub, through the GitHub CLI, whether its signed-in account can push to
  the repository (`.permissions.push`). No `gh`, signed out (or a token GitHub refuses) or no
  push access: no Admin section in Settings, tier lists and builds are view-only and
  `edit_content` is refused, even with `admin: true` in settings.json. GitHub unreachable: asks
  again every 3 minutes. Verified in the dev app as the admin and with `gh` signed out
  (`GH_CONFIG_DIR` pointing at an empty folder). Not tested: a signed-in account without push
  access (needs a second GitHub account; the same `false` answer path).
- 1.0.11: tier lists in Chinese use the usual Chinese names (夯, 顶级, 人上人, NPC, 拉完了; keys
  1-5 as well as S-D); English keeps S-D. The item pool now matches what Hextech ARAM players
  finish: Data Dragon's `aramLegendary` (sold on map 12 under a base or 12xxxx id, 2200+ gold,
  nothing sold after it) gives exactly AramHex's 108 items, including the ARAM-only ones the old
  Rift-based rule dropped (Rite of Ruin, Sword of Blossoming Dawn, Atma's Reckoning, its Hubris,
  Spectral Cutlass, Whispering Circlet). Also in the pool: the four Guardian items and the small
  anti-heal items (Bramble Vest, Oblivion Orb, Executioner's Calling); a search finds any item
  Hextech ARAM sells. `aram` now excludes other modes' twins (22xxxx, 77xxxx), so the build
  editor needs no id cut-off. The board no longer shrinks its last row when the pool is long.
  Data Dragon cache renamed `-4.json`.
- 1.0.10: the admin's hand-written content, published to every user.
  - Content (`content.rs`): Hextech ARAM builds plus item and champion tier lists in one
    `content.json` on the repo's `content` branch. Apps check every 3 minutes (latest commit via
    the GitHub API, 60 calls an hour per address, else the raw branch file, cached up to 5 min).
    In use: the admin's draft (`content-draft.json`), else the newer of the shipped copy
    (`src-tauri/data/content.json`) and the last fetched one (`%APPDATA%\Mattofessor\content.json`).
    Admin edits publish 4 s after the last one with `gh api` (contents API); a failed publish shows
    "发布失败" with Retry and is retried on the next check. `MATTOFESSOR_CONTENT_BRANCH` points a
    dev build at another branch.
  - Settings > Admin turns on the editors. Builds page (ARAM): starting items, order,
    situational, note; green ring and a count for champions with a build. The build replaces
    OP.GG's items there, shows under the augments in the in-game panel and on Live, and is what
    the item set import writes.
  - Tier lists page (new rail icon): items, and champions by Data Dragon class (战士, 刺客, 法师,
    坦克, 辅助, ADC), tiers S to D. The admin drags icons (pointer events) or clicks one and presses
    S/A/B/C/D; the unranked pool is sorted by win rate. Everyone else sees the ranked rows.
  - Meta page, Hextech ARAM: new Items view (AramHex `hero_items.json` summed over champions: win
    rate, difference from each champion's own win rate, built-by share; "all" and "top 5%" only)
    and a class filter on Champions.
  - Items: Hextech ARAM games record the normal ids; Data Dragon's 77xxxx ARAM twins belong to
    regular ARAM and are left out of the editor. Tier-2 boots count as boots again (they build
    into tier 3 now); Data Dragon cache files are `<patch>-<locale>-3.json`.
  - Tested in a dev build against a `content-dev` branch (deleted after): drags, keys, publish,
    a fresh install receiving it. Not tested: the installed app publishing (needs this release).
- 1.0.9, cleanup: one HTTP client for public data (`net::http`, timeout
  per request; dd, opgg, aramhex, augments and riot each built their own, three per request); one
  cache-with-fallback (`store::cached`, was copied in opgg and augments); window helpers in core
  (`hwnd`, `hide_window`); `Callout` for the notices (RiotKeyBanner, UpdateBanner, FullscreenNotice
  had the same markup and CSS); close buttons use `.icon-btn` / `.icon-btn.sm`; removed the unused
  `live_state` command, `champByKey`, `DEFAULT_THEME`; stale and repeated comments trimmed.
- 1.0.8, from a friend's first install:
  - In-game panel size in Settings: small, medium, regular, large, extra large (fills the game's
    width). The app zooms the panel (`core::overlay_zoom`, layout 1520 CSS px, smaller zoom when
    the game is too narrow); the page reports its unzoomed height.
  - Panel layout: augment rows are as wide as their text (number right after the name, plus the
    win rate) with the augments to avoid beside them; a team alone keeps half the width
    (`auto-fill`); recent game cells fit ARAM scores like 36/21/40 (they overlapped).
  - Full screen: no window can show over a Full screen game; tested 2026-11-15 in a custom game:
    the panel stayed invisible and Ctrl+X minimized the game. Now the panel, spell timers and card
    win rates stay closed while the game runs in Full screen (`display::game_fullscreen`, from
    `Config/game.cfg`, which the game rewrites when its mode changes; the client's setting is
    stale during a game). A key press then is remembered: after the game the main window opens on
    Live with "the panel stayed closed: switch to Borderless or Windowed" and the one-click switch
    (client setting, plus `game.cfg` when no game runs). Overwolf apps (Porofessor) draw inside the
    game by injecting into it; not an option here (Vanguard). Windowed: the panel shows over the
    game (tested).
    Tested 2026-11-15 with a real key press in a Full screen custom game: kept closed, and the
    notice opened on Live after the game. The notice has a close button (until the app restarts;
    it also forgets the kept-closed panel).
  - Panel header: size buttons (A- / size / A+) step through the five sizes at once.
  - Custom games (queue 3100 and on) are labelled Custom (they showed "Other mode").
  - Card reader read no Chinese card ever (a friend's PC, Chinese client): Windows' Chinese OCR
    gives each character as a word, and every word of 2 characters or less around a title was
    dropped as junk, so the whole title went. CJK words are never junk now; CJK names allow one
    wrong character from 4 characters on; a close match needs a clear winner; up to 5 cards
    (a 4-card mode is coming). Checked on two of the friend's real screenshots (3 of 3 each, ~50 ms)
    and live in an English game. `cards::scan::scan_image` reads any screenshot by hand.
  - Card reader: logs its setup once per game (game window, League mode, OCR languages, game
    language) and a black screen capture; Settings warns when Windows cannot read the game's
    language (no OCR pack) with the fix.
  - Ctrl+X works from champ select: the panel shows the Hextech ARAM win rates of your team and the
    bench (click opens the ARAM build in the main window) and opens on the client's monitor. The
    spell timer key still only works in game.
  - Update available: red dot on Settings and a closable card beside it (`UpdateNotice`); the top
    banner now only shows the download.
  - Builds page: lane tabs (All, Top, Jungle, Mid, Bot, Support, Off-meta) from OP.GG's lane lists
    (a lane is regular from 20% of a champion's games); Off-meta groups the rest by lane. A build
    shows every lane: an off-meta lane gets its own game count, a rare one is marked as such.
  - Dev: `/demo_game` on the dev bridge loads a stand-in Hextech game (`?phase=ChampSelect` for
    champ select) from the cache, to check the panel without League.
- 1.0.8: Builds stuck on "Loading..." for many champions, Rift and
  ARAM. OP.GG now lists starter items with repeats (Doran's Ring + 2 Health Potions) and the page
  keyed that list by item id, so Svelte threw `each_key_duplicate` while drawing and the page
  stayed half drawn. The backend was fine (it fetched and cached the build). Build lists are now
  keyed by position. The keystone row was squeezed to 28 px by the skill badges' `.key` style
  (same class name), so the 4th keystone (Conqueror) sat on the secondary tree; renamed to
  `keystone`. Pages now sit in an error boundary (message + Try again, cleared by changing page)
  and page errors go to app.log (`log_error`), which had no trace of this bug.
- Missing or rejected Riot API key: a banner on the Profile and Live pages says losses and win
  rates cannot be loaded and opens the guide ("Later" hides it until the app restarts or the key
  status changes). Wins-only lines show a "?" (tooltip) in player rows and a "Win rate needs a
  Riot API key" link on profile rank cards. Settings and the guide now point to the Personal
  API Key (no expiry) and tell people to skip the 24-hour development key.
- 1.0.7: the main window's close (and minimize) button did nothing.
  The app had two main windows on top of each other: 1.0.6 was installed by the in-app update
  and restarted during champ select, and the setup and the game watcher ("open in champ
  select") each found no main window and opened one. Both had the label "main", so the title
  bar buttons worked the hidden one. Windows are now looked up and created on the main thread
  only, one request after another (`Core::on_main`), for the main window and the in-game ones.
  A page asked for while the main window's page is still loading is kept until the page takes
  it (`take_main_route`), so starting during champ select still opens on the Live page.
  Reproduced with a stand-in client that is in champ select from the start (old build: two
  windows, X dead; fixed build: one window on the Live page, 8 starts); close, minimize,
  reopening on a profile, the panel and the spell tracker buttons checked after the fix.
- 1.0.6: a teammate was missing from the in-game panel and the Live page
  in a ranked game (11 Nov, Seraphine on your team). The game writes Seraphine's name key as
  `Character_Seraphine_Name` (everyone else: `game_character_displayname_X`), it was read as
  "Name", matched no session player (a ranked session has no Riot IDs, so the champion is the
  only link) and the player was dropped as "left over from an earlier game". Fixed in three
  layers: both key shapes are read, champion keys are compared without case, and
  `game::settle` no longer drops anyone on a side where a game player is still unmatched (one
  left over on a side is paired by elimination, which also gives the spell tracker its icon).
  An unknown champion name is logged once per game. Seen on the real game (old build 4 allies,
  fixed build scouted all 10) and on a replay of that game through the mock (panel and Live
  page list 5 + 5). `mock_client.py` now sends the session without Riot IDs and Seraphine's
  real name key, as the real client and game do.
- 1.0.6: "Show in-game panel" / "Enemy summoner spells" on the Live page (and opening a
  profile from the panel while the main window is closed) froze the app when the window had not
  been created yet in that run: a window created inside a plain command deadlocks on Windows.
  Those three commands now run off the main thread.
- Match history, opened game: a second view "Damage and healing" with bars and exact numbers for
  damage dealt, taken, self-mitigated and healing per player (the choice is saved as
  `matchView`); column headings in both views; in narrow windows the rows use a second line
  instead of squeezing the names away. Checked on real games in the dev app (1.0.5)
- Scouting panel and Live page: the two teams share row heights (CSS subgrid), so players line up
  across the teams when tags wrap or details are open (1.0.4)
- Zoom: Ctrl + / Ctrl - / Ctrl + mouse wheel (or pinch) / Ctrl 0 in the main window step the
  saved text size, with a badge showing the size and a reset button (1.0.3)
- Tags: separate rules for ARAM / Hextech ARAM (no Rift role, farming or death tags there; "dies
  a lot" counts Rift games only); new tags: kind of champion played most (40%+), win rate
  (60%+ / 40%- over 15+), strong champion (70%+ over 5+). Checked on 40 saved players (1.0.2)
- Card win rates: update on rerolls and later picks, ignore Mattofessor's own panel, wait for the
  cards to settle (1.0.1)
- Hextech ARAM augment recommendations and win rates on the augment cards (1.0.0)
- Hextech ARAM augments in match history; repair of a damaged cache (0.3.4)
- Profile no longer stuck loading after switching accounts (0.3.3)
- Augment card samples collected by the app (Settings > Augment reader, off by default)
- Champ select: Hextech ARAM win rates for your team and the bench
- Meta page and public builds: OP.GG for the Rift, AramHex for Hextech ARAM (0.3.0)
- Profiles show instantly from a per-account cache
