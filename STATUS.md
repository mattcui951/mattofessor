# Status

Updated: 2026-11-07
Version: 1.0.1

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
  - Not verified: a real game with 1.0.1, the Chinese client, other HUD scales and window sizes.

## Next

- Try the card reader (1.0.1) in a real Hextech ARAM game. The augment sample capture can go
  once the reader is confirmed (one game made 824 PNGs, 3.4 GB).
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

- Without a Riot API key, other players' ranked losses show as 0 (an LCU limit).
- Match history from the LCU is limited to 40 games and the first lookup takes about 5 s.

## Recently done

- Card win rates: update on rerolls and later picks, ignore Mattofessor's own panel, wait for the
  cards to settle (1.0.1)
- Hextech ARAM augment recommendations and win rates on the augment cards (1.0.0)
- Hextech ARAM augments in match history; repair of a damaged cache (0.3.4)
- Profile no longer stuck loading after switching accounts (0.3.3)
- Augment card samples collected by the app (Settings > Augment reader, off by default)
- Champ select: Hextech ARAM win rates for your team and the bench
- Meta page and public builds: OP.GG for the Rift, AramHex for Hextech ARAM (0.3.0)
- Profiles show instantly from a per-account cache
