"""Scores a player's recent games through the debug app's bridge and prints the spread, to
calibrate src-tauri/src/score.rs. Needs `npm run tauri dev` running with the League client open.

    py -I tools/score_report.py [puuid] [--games 25] [--show 3]
"""

import argparse
import json
import statistics
import urllib.request

BRIDGE = "http://127.0.0.1:17431"


def get(path):
    with urllib.request.urlopen(BRIDGE + path, timeout=60) as r:
        return json.loads(r.read())


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("puuid", nargs="?", default="")
    ap.add_argument("--games", type=int, default=25)
    ap.add_argument("--show", type=int, default=3)
    ap.add_argument("--mode", default="", help="only games of this mode, e.g. CLASSIC")
    a = ap.parse_args()
    prof = get(f"/profile?puuid={a.puuid}")
    me = prof["card"]["summoner"]["puuid"]
    games = [g for g in prof["history"] if not g["remake"] and (not a.mode or g["mode"] == a.mode)][: a.games]
    all_scores, mvp, svp, mine, by_mode = [], [], [], [], {}
    shown = 0
    for g in games:
        gid = g["gameId"]
        scores = get(f"/scores?gameId={gid}")
        if not scores:
            continue
        full = get(f"/game?gameId={gid}")
        vals = [s["score"] for s in scores]
        all_scores += vals
        by_mode.setdefault(full.get("gameMode"), []).extend(vals)
        mvp += [s["score"] for s in scores if s["badge"] == "mvp"]
        svp += [s["score"] for s in scores if s["badge"] == "svp"]
        mine += [s["score"] for s in scores if s["puuid"] == me]
        if shown < a.show:
            shown += 1
            parts = {p["participantId"]: p for p in full["participants"]}
            print(f"\n== game {gid} {full['gameMode']} {full['gameDuration'] // 60} min")
            for s in sorted(scores, key=lambda s: -s["score"]):
                st = parts[s["participantId"]]["stats"]
                print(
                    f"  {s['score']:4.1f} {s['badge']:3} team={parts[s['participantId']]['teamId']} win={st['win']!s:5} "
                    f"{st['kills']:2}/{st['deaths']:2}/{st['assists']:2} dmg={st['totalDamageDealtToChampions']:6} "
                    f"taken={st['totalDamageTaken']:6} cs={st['totalMinionsKilled'] + st['neutralMinionsKilled']:3} vis={st['visionScore']:3}"
                    + ("  <- me" if s["puuid"] == me else "")
                )

    def summary(name, xs):
        if xs:
            print(f"{name:10} n={len(xs):3} mean={statistics.mean(xs):.2f} sd={statistics.pstdev(xs):.2f} min={min(xs):.1f} max={max(xs):.1f}")

    print()
    summary("all", all_scores)
    for mode, xs in by_mode.items():
        summary(mode, xs)
    summary("MVP", mvp)
    summary("SVP", svp)
    summary("me", mine)


if __name__ == "__main__":
    main()
