# Status

Updated: 2026-11-05
Version: 0.3.4, released 2026-11-04

## In progress

- Match history after a damaged cache: v0.3.4 repairs a damaged `cache.db` at start. The last
  session stopped while checking why history could come back empty on a damaged database. Not yet
  confirmed on the installed copy (it was still on 0.3.3 then).

## Next

- Augment card reader for Hextech ARAM (Windows OCR, triggered from live data at levels 3/7/11/15,
  hidden once the held-augment count rises). Shrink the sample captures first: one game made
  824 PNGs (3.4 GB).
- Hextech ARAM item data from Tencent's official source (101.qq.com): needs a permission rule
  before Claude can fetch it.

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
