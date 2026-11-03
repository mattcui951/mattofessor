"""Collects samples for the augment card reader during real Hextech ARAM games.

Run it before or during a game and leave it running; it stops by itself when the game ends.

    py -I tools/capture_augments.py

While you are dead at level 3 or higher it saves the middle band of the game window (where the
augment cards appear) about once a second, and it logs what the game's live data says about you
(level, death, runes, which is where held augments are listed). Nothing is sent anywhere and
nothing outside the League game window is captured. Output: tools/.captures/<start time>/.
Delete the folder when the reader is done.
"""

import ctypes
import ctypes.wintypes as wt
import json
import ssl
import sys
import time
import urllib.request
from pathlib import Path

from PIL import ImageGrab

LIVE = "https://127.0.0.1:2999/liveclientdata/allgamedata"
GAME_TITLE = "League of Legends (TM) Client"
BAND = (0.12, 0.88)  # vertical part of the window kept: the cards sit in the middle
MAX_PER_DEATH = 40

user32 = ctypes.windll.user32
try:
    ctypes.windll.shcore.SetProcessDpiAwareness(2)  # real pixels on scaled displays
except Exception:
    user32.SetProcessDPIAware()

ctx = ssl.create_default_context()
ctx.check_hostname = False
ctx.verify_mode = ssl.CERT_NONE


def live():
    try:
        with urllib.request.urlopen(LIVE, context=ctx, timeout=1.5) as r:
            return json.loads(r.read())
    except Exception:
        return None


def game_rect():
    hwnd = user32.FindWindowW(None, GAME_TITLE)
    if not hwnd:
        return None
    r = wt.RECT()
    user32.GetClientRect(hwnd, ctypes.byref(r))
    pt = wt.POINT(0, 0)
    user32.ClientToScreen(hwnd, ctypes.byref(pt))
    return pt.x, pt.y, pt.x + r.right, pt.y + r.bottom


def me_of(data):
    ap = data.get("activePlayer", {})
    rid = ap.get("riotId") or ap.get("summonerName")
    for p in data.get("allPlayers", []):
        if rid in (p.get("riotId"), p.get("summonerName")):
            return ap, p
    return ap, None


def main():
    out = Path(__file__).resolve().parent / ".captures" / time.strftime("%Y%m%d-%H%M%S")
    print("Waiting for a game (start this before or during a Hextech ARAM game)...")
    data = None
    while data is None:
        data = live()
        if data is None:
            time.sleep(2)
    out.mkdir(parents=True, exist_ok=True)
    mode = data.get("gameData", {}).get("gameMode")
    print(f"Game found (mode {mode}). Saving to {out}")
    log = (out / "live.jsonl").open("a", encoding="utf-8")
    last_state, shots, missing_since, total = None, 0, None, 0
    while True:
        data = live()
        if data is None:
            missing_since = missing_since or time.time()
            if time.time() - missing_since > 15:
                break
            time.sleep(1)
            continue
        missing_since = None
        ap, me = me_of(data)
        if not me:
            time.sleep(1)
            continue
        game_time = data.get("gameData", {}).get("gameTime", 0)
        level, dead = me.get("level", 0), me.get("isDead", False)
        state = json.dumps([level, dead, me.get("runes"), ap.get("fullRunes")], sort_keys=True)
        if state != last_state:
            last_state = state
            log.write(json.dumps({"gameTime": round(game_time, 1), "level": level, "isDead": dead, "champion": me.get("championName"),
                                  "runes": me.get("runes"), "activeFullRunes": ap.get("fullRunes"), "mode": data.get("gameData", {}).get("gameMode")},
                                 ensure_ascii=False) + "\n")
            log.flush()
            if not dead:
                shots = 0
        if dead and level >= 3 and shots < MAX_PER_DEATH:
            rect = game_rect()
            if rect:
                x0, y0, x1, y1 = rect
                h = y1 - y0
                band = (x0, y0 + int(h * BAND[0]), x1, y0 + int(h * BAND[1]))
                img = ImageGrab.grab(bbox=band, all_screens=True)
                img.save(out / f"t{game_time:06.0f}_lvl{level:02d}_{shots:02d}.png", optimize=True)
                shots += 1
                total += 1
        time.sleep(1)
    log.close()
    print(f"Game over. {total} captures in {out}")


if __name__ == "__main__":
    sys.exit(main())
