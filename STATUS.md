# Status

Updated: 2026-11-06
Version: 0.3.4, released 2026-11-04

## In progress

- Hextech ARAM augment recommendations and card reader, uncommitted, not yet released:
  - `aramhex::picks`: per rarity the 4 augments to take and 2 to avoid for a champion (small
    samples left out); `AugmentPicks.svelte` on Live in champ select and in game, compact in
    the overlay. Seen working in champ select with the mock.
  - `cards.rs` + `ocr.rs`: every 0.5 s in a KIWI game, Windows OCR reads the title band (30-62%
    of the window), matches augment names (en + zh, fuzzy, same row), and a click-through
    window (`overlay/Cards.svelte`) shows win rate, difference and "Best" above each card.
    Placed from card spacing (cards are sized by the HUD: titles at 41% of 3440x1440, 489 px
    apart). Seen working end to end with `mock_client.py --screen` and a real frame: shows,
    hides when the cards are covered, comes back. 0.2 core-seconds/s in a debug build.
    Settings > Augment reader toggle (on by default).
  - Verified on 824 saved samples: 8 offers found, no false matches after the same-row rule.
  - Not verified: a real game (needs a release build), the Chinese client, HUD scales other
    than the default, a window other than 3440x1440.
- Match history after a damaged cache: v0.3.4 repairs a damaged `cache.db` at start. The last
  session stopped while checking why history could come back empty on a damaged database. Not yet
  confirmed on the installed copy (it was still on 0.3.3 then).

## Next

- Try the card reader in a real Hextech ARAM game, then release. The augment sample capture can
  go once the reader is confirmed (one game made 824 PNGs, 3.4 GB).
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

- Hextech ARAM augments in match history; repair of a damaged cache (0.3.4)
- Profile no longer stuck loading after switching accounts (0.3.3)
- Augment card samples collected by the app (Settings > Augment reader, off by default)
- Champ select: Hextech ARAM win rates for your team and the bench
- Meta page and public builds: OP.GG for the Rift, AramHex for Hextech ARAM (0.3.0)
- Profiles show instantly from a per-account cache
