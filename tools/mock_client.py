"""Replays a real past game as if it were happening now, for testing Mattofessor end to end.

* A League client stand-in: an HTTPS server with its own lockfile. It answers the game flow
  (Lobby -> ChampSelect -> InProgress -> EndOfGame), champ select and game session with the
  real players of a past game, and forwards every other request to the real, running League
  client. So all player lookups (ranks, histories, mastery) are real.
* A Live Client Data API on https://127.0.0.1:2999 with those players, their spells and items,
  and a game clock (a 10 s loading screen first).
* A stand-in game window, "Mattofessor Mock Game.exe" (a copy of ffplay, borderless). It is not
  named "League of Legends.exe", so other League tools on this PC ignore it.

Run the app against it with tools/dev-mock.cmd (sets MATTOFESSOR_LOCKFILE and
MATTOFESSOR_GAME_EXE). Needs the real League client running and signed in, and ffplay.exe (FFmpeg)
on PATH or in MATTOFESSOR_FFPLAY.

    py -I tools/mock_client.py [--game <gameId>] [--champ-select 30] [--length 900]
"""

import argparse
import base64
import json
import os
import secrets
import shutil
import ssl
import subprocess
import sys
import threading
import time
import urllib.error
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

HERE = Path(__file__).resolve().parent
WORK = HERE / ".mock"
OPENSSL = Path(os.environ.get("OPENSSL", r"C:\Program Files\Git\usr\bin\openssl.exe"))


def real_lockfile() -> Path:
    """The running League client's lockfile: MATTOFESSOR_REAL_LOCKFILE, or next to the client's exe."""
    if os.environ.get("MATTOFESSOR_REAL_LOCKFILE"):
        return Path(os.environ["MATTOFESSOR_REAL_LOCKFILE"])
    out = subprocess.run(
        ["powershell", "-NoProfile", "-Command", "(Get-Process LeagueClientUx -ErrorAction SilentlyContinue | Select-Object -First 1).Path"],
        capture_output=True,
        text=True,
    ).stdout.strip()
    if not out:
        sys.exit("Start the League client first (or set MATTOFESSOR_REAL_LOCKFILE).")
    return Path(out).parent / "lockfile"


def ffplay() -> Path:
    """ffplay.exe for the stand-in game window: MATTOFESSOR_FFPLAY, or on PATH."""
    found = os.environ.get("MATTOFESSOR_FFPLAY") or shutil.which("ffplay")
    if not found:
        sys.exit("ffplay.exe is needed for the stand-in game window: put it on PATH or set MATTOFESSOR_FFPLAY.")
    return Path(found)
GAME_EXE = "Mattofessor Mock Game.exe"
SUPPORT_ITEMS = {3865, 3866, 3867, 3869, 3870, 3871, 3876, 3877}
INSECURE = ssl._create_unverified_context()


class Real:
    """The real League client."""

    def __init__(self):
        parts = real_lockfile().read_text().split(":")
        self.base = f"https://127.0.0.1:{parts[2]}"
        self.auth = "Basic " + base64.b64encode(f"riot:{parts[3]}".encode()).decode()

    def request(self, method, path, body=None):
        req = urllib.request.Request(self.base + path, data=body, method=method, headers={"Authorization": self.auth, "Content-Type": "application/json"})
        try:
            with urllib.request.urlopen(req, context=INSECURE, timeout=30) as r:
                return r.status, r.read()
        except urllib.error.HTTPError as e:
            return e.code, e.read()

    def get(self, path):
        status, body = self.request("GET", path)
        if status != 200:
            raise RuntimeError(f"{path}: {status} {body[:200]!r}")
        return json.loads(body)


def ddragon():
    def get(url):
        with urllib.request.urlopen(url, timeout=20) as r:
            return json.loads(r.read())

    v = get("https://ddragon.leagueoflegends.com/api/versions.json")[0]
    champs = get(f"https://ddragon.leagueoflegends.com/cdn/{v}/data/en_US/champion.json")["data"]
    spells = get(f"https://ddragon.leagueoflegends.com/cdn/{v}/data/en_US/summoner.json")["data"]
    return {int(c["key"]): c["id"] for c in champs.values()}, {int(s["key"]): s["id"] for s in spells.values()}


def position(p):
    s, tl = p["stats"], p.get("timeline", {})
    if 11 in (p["spell1Id"], p["spell2Id"]):
        return "JUNGLE"
    if any(s.get(f"item{i}") in SUPPORT_ITEMS for i in range(7)) or tl.get("role") in ("SUPPORT", "DUO_SUPPORT"):
        return "UTILITY"
    return {"TOP": "TOP", "MIDDLE": "MIDDLE", "BOTTOM": "BOTTOM"}.get(tl.get("lane"), "")


class Script:
    def __init__(self, game, me, champ_select, length, champ_keys, spell_keys):
        self.level_every = 90
        self.deaths = False
        self.summoner_down = 0.0
        self.start = time.time()
        self.cs = champ_select
        self.length = length
        self.champ_keys, self.spell_keys = champ_keys, spell_keys
        ids = {i["participantId"]: i["player"] for i in game["participantIdentities"]}
        self.players = []
        for p in game["participants"]:
            pl = ids[p["participantId"]]
            self.players.append(
                {
                    "puuid": pl["puuid"],
                    "name": pl["gameName"],
                    "tag": pl["tagLine"],
                    "champ": p["championId"],
                    "team": p["teamId"],
                    "spells": [p["spell1Id"], p["spell2Id"]],
                    "items": [p["stats"].get(f"item{i}", 0) for i in range(7)],
                    "position": position(p),
                }
            )
        if not any(p["puuid"] == me for p in self.players):
            me = self.players[0]["puuid"]
        self.me = me
        self.my_team = next(p["team"] for p in self.players if p["puuid"] == me)
        self.queue = {"id": game["queueId"], "gameMode": game["gameMode"], "mapId": game["mapId"]}

    def t(self):
        return time.time() - self.start

    def phase(self):
        t = self.t()
        if t < 4:
            return "Lobby"
        if t < 4 + self.cs:
            return "ChampSelect"
        if t < 4 + self.cs + self.length:
            return "InProgress"
        if t < 4 + self.cs + self.length + 10:
            return "EndOfGame"
        return "None"

    def in_game_time(self):
        """Seconds since the game process started (loading screen included)."""
        return self.t() - 4 - self.cs

    def champ_select(self):
        t = self.t() - 4
        mine = [p for p in self.players if p["team"] == self.my_team]
        theirs = [p for p in self.players if p["team"] != self.my_team]
        my_team = []
        for i, p in enumerate(mine):
            locked = t > 6 + i * 2
            my_team.append(
                {
                    "cellId": i,
                    "puuid": p["puuid"],
                    "gameName": p["name"],
                    "tagLine": p["tag"],
                    "championId": p["champ"] if locked else 0,
                    "championPickIntent": p["champ"] if t > 3 else 0,
                    "assignedPosition": p["position"].lower(),
                    "team": 1,
                    "spell1Id": p["spells"][0],
                    "spell2Id": p["spells"][1],
                }
            )
        their_team = [
            {"cellId": 5 + i, "puuid": "", "championId": p["champ"] if t > 8 + i * 2 else 0, "team": 2} for i, p in enumerate(theirs)
        ]
        local = next(i for i, p in enumerate(mine) if p["puuid"] == self.me)
        return {
            "localPlayerCellId": local,
            "myTeam": my_team,
            "theirTeam": their_team,
            "timer": {"phase": "FINALIZATION" if t > self.cs - 8 else "BAN_PICK"},
        }

    def session(self):
        def entry(p):
            return {"puuid": p["puuid"], "championId": p["champ"], "selectedPosition": p["position"], "gameName": p["name"], "tagLine": p["tag"]}

        return {
            "phase": self.phase(),
            "map": {"id": self.queue["mapId"]},
            "gameData": {
                "queue": self.queue,
                "teamOne": [entry(p) for p in self.players if p["team"] == 100],
                "teamTwo": [entry(p) for p in self.players if p["team"] == 200],
                "playerChampionSelections": [
                    {"puuid": p["puuid"], "championId": p["champ"], "spell1Id": p["spells"][0], "spell2Id": p["spells"][1]} for p in self.players
                ],
            },
        }

    def all_game_data(self):
        gt = self.in_game_time() - 10
        if gt < 0:
            return None
        me = next(p for p in self.players if p["puuid"] == self.me)

        def player(p):
            return {
                "riotId": f"{p['name']}#{p['tag']}",
                "rawChampionName": f"game_character_displayname_{self.champ_keys.get(p['champ'], 'Unknown')}",
                "team": "ORDER" if p["team"] == 100 else "CHAOS",
                "level": min(18, 1 + int(gt // self.level_every)),
                # With --deaths, you are dead for 15 s of every minute (to test augment picks).
                "isDead": self.deaths and p["puuid"] == self.me and gt % 60 < 15,
                "respawnTimer": 0.0,
                "summonerSpells": {
                    k: {"rawDisplayName": f"GeneratedTip_SummonerSpell_{self.spell_keys.get(s, 'SummonerFlash')}_DisplayName"}
                    for k, s in zip(("summonerSpellOne", "summonerSpellTwo"), p["spells"])
                },
                "items": [{"itemID": i} for i in p["items"] if i],
            }

        return {
            "activePlayer": {"riotId": f"{me['name']}#{me['tag']}"},
            "allPlayers": [player(p) for p in self.players],
            "gameData": {"gameTime": gt, "gameMode": self.queue["gameMode"]},
        }


def client_handler(real, script, password):
    expected = "Basic " + base64.b64encode(f"riot:{password}".encode()).decode()

    class H(BaseHTTPRequestHandler):
        def log_message(self, *a):
            pass

        def reply(self, code, obj=None, raw=None):
            body = raw if raw is not None else json.dumps(obj).encode()
            self.send_response(code)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def handle_any(self, method):
            if self.headers.get("Authorization") != expected:
                return self.reply(401, {"message": "bad auth"})
            path = self.path
            phase = script.phase()
            if path == "/lol-summoner/v1/current-summoner" and script.t() < script.summoner_down:
                # A client still starting up: up, but not answering who is signed in yet.
                return self.reply(503, {"message": "summoner not ready"})
            if path == "/lol-gameflow/v1/gameflow-phase":
                return self.reply(200, phase)
            if path == "/lol-gameflow/v1/session":
                return self.reply(200, script.session()) if phase in ("ChampSelect", "InProgress", "EndOfGame") else self.reply(404, {"message": "No gameflow session exists."})
            if path == "/lol-champ-select/v1/session":
                return self.reply(200, script.champ_select()) if phase == "ChampSelect" else self.reply(404, {"message": "No active delegate"})
            length = int(self.headers.get("Content-Length") or 0)
            body = self.rfile.read(length) if length else None
            if method != "GET" and path.startswith(("/lol-perks/", "/lol-item-sets/")):
                # Never change the real account's rune pages or item sets: record the write.
                with open(WORK / "writes.log", "a", encoding="utf-8") as f:
                    f.write(json.dumps({"method": method, "path": path, "body": json.loads(body) if body else None}) + "\n")
                return self.reply(200, {})
            status, data = real.request(method, path, body)
            self.reply(status, raw=data)

        def do_GET(self):
            self.handle_any("GET")

        def do_POST(self):
            self.handle_any("POST")

        def do_PUT(self):
            self.handle_any("PUT")

        def do_DELETE(self):
            self.handle_any("DELETE")

        def do_PATCH(self):
            self.handle_any("PATCH")

    return H


def live_handler(script):
    class H(BaseHTTPRequestHandler):
        def log_message(self, *a):
            pass

        def do_GET(self):
            data = script.all_game_data() if script.phase() == "InProgress" else None
            if self.path.startswith("/liveclientdata/allgamedata") and data:
                body = json.dumps(data).encode()
                self.send_response(200)
            else:
                body = b'{"errorCode":"RPC_ERROR","httpStatus":404}'
                self.send_response(404)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

    return H


def ensure_cert():
    cert, key = WORK / "mock.crt", WORK / "mock.key"
    if not cert.exists():
        subprocess.run(
            [str(OPENSSL), "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "30", "-subj", "/CN=127.0.0.1", "-keyout", str(key), "-out", str(cert)],
            check=True,
            capture_output=True,
        )
    ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    ctx.load_cert_chain(cert, key)
    return ctx


class Server(ThreadingHTTPServer):
    def handle_error(self, request, client_address):
        pass  # the app dropping a connection (restart) is not an error here


def serve(handler, port, ctx):
    httpd = Server(("127.0.0.1", port), handler)
    httpd.socket = ctx.wrap_socket(httpd.socket, server_side=True)
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    return httpd


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--game", type=int, help="game id to replay (default: your latest 5v5 Summoner's Rift game, else your latest game)")
    ap.add_argument("--champ-select", type=float, default=30)
    ap.add_argument("--length", type=float, default=900, help="seconds in game")
    ap.add_argument("--level-every", type=float, default=90, help="seconds per level")
    ap.add_argument("--deaths", action="store_true", help="you die for 15 s of every game minute")
    ap.add_argument("--summoner-down", type=float, default=0, help="seconds the signed-in account is not answered yet (a client still starting)")
    args = ap.parse_args()

    WORK.mkdir(exist_ok=True)
    real = Real()
    me = real.get("/lol-summoner/v1/current-summoner")["puuid"]
    game_id = args.game
    if not game_id:
        hist = real.get(f"/lol-match-history/v1/products/lol/{me}/matches?begIndex=0&endIndex=39")["games"]["games"]
        rift = [g for g in hist if g["mapId"] == 11 and g["gameMode"] == "CLASSIC"]
        if not rift and not hist:
            sys.exit("no recent games on this account: pass --game <gameId>")
        game_id = (rift or hist)[0]["gameId"]
    game = real.get(f"/lol-match-history/v1/games/{game_id}")
    champ_keys, spell_keys = ddragon()
    script = Script(game, me, args.champ_select, args.length, champ_keys, spell_keys)
    script.level_every = args.level_every
    script.deaths = args.deaths
    script.summoner_down = args.summoner_down

    ctx = ensure_cert()
    password = secrets.token_urlsafe(16)
    client = serve(client_handler(real, script, password), 0, ctx)
    port = client.server_address[1]
    lockfile = WORK / "lockfile"
    lockfile.write_text(f"LeagueClient:{__import__('os').getpid()}:{port}:{password}:https")
    serve(live_handler(script), 2999, ctx)
    print(f"mock client on port {port}, lockfile {lockfile}", flush=True)
    print(f"replaying game {game_id} ({script.queue['gameMode']}, queue {script.queue['id']}), {len(script.players)} players", flush=True)

    exe = WORK / GAME_EXE
    shutil.copy(ffplay(), exe)
    game_proc = None
    last = None
    try:
        while script.phase() != "None" or script.t() < 5:
            ph = script.phase()
            if ph != last:
                print(f"{script.t():6.1f}s phase {ph}", flush=True)
                last = ph
            if ph == "InProgress" and game_proc is None:
                game_proc = subprocess.Popen(
                    [str(exe), "-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc2=size=1600x900:rate=30", "-noborder", "-left", "60", "-top", "60", "-window_title", "Mock League game"],
                    creationflags=0x08000000,
                )
            if ph != "InProgress" and game_proc is not None:
                game_proc.terminate()
                game_proc = None
            time.sleep(0.5)
    finally:
        if game_proc is not None:
            game_proc.terminate()
    print("done", flush=True)


if __name__ == "__main__":
    sys.exit(main())
