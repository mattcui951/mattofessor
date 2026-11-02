//! Follows the League client's game flow once a second and keeps the live scouting state:
//! champ select (your team), then in game (everyone), with each player's card and tags.
//!
//! Phases (from `/lol-gameflow/v1/gameflow-phase`): None, Lobby, Matchmaking, ReadyCheck,
//! ChampSelect, GameStart, InProgress, Reconnect, WaitingForStats, PreEndOfGame, EndOfGame.

use crate::analysis::{self, Context, PlayerCard, Tag};
use crate::core::Core;
use crate::lcu;
use crate::players::{self, Inputs};
use crate::{builds, live, win};
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::Emitter;

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveState {
    /// Client game flow phase; "" when the client is not running.
    pub phase: String,
    pub connected: bool,
    pub queue_id: i32,
    pub mode: String,
    pub map_id: i32,
    /// Teams of the current (or last) game; your team first.
    pub teams: Vec<LiveTeam>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveTeam {
    pub id: i32,
    pub ally: bool,
    pub players: Vec<Slot>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Slot {
    pub puuid: String,
    pub champion_id: i32,
    pub position: String,
    pub spells: [i32; 2],
    pub me: bool,
    /// Riot ID when known.
    pub name: String,
    /// "loading" | "ready" | "error" | "hidden" (name hidden by the client)
    pub status: String,
    pub error: Option<String>,
    pub card: Option<PlayerCard>,
}

#[derive(Clone, Debug, Default)]
struct Entry {
    puuid: String,
    champion_id: i32,
    position: String,
    spells: [i32; 2],
    team_id: i32,
    me: bool,
    name: String,
}

#[derive(Clone, Debug, Default)]
struct Roster {
    queue_id: i32,
    mode: String,
    map_id: i32,
    entries: Vec<Entry>,
}

enum Fetch {
    Loading,
    Ready(Arc<Inputs>, Instant),
    Failed(String, Instant),
}

/// Player data for the current game, by puuid. Kept 10 minutes so champ select lookups carry
/// into the game.
static FETCHES: Mutex<Option<HashMap<String, Fetch>>> = Mutex::new(None);

fn with_fetches<R>(f: impl FnOnce(&mut HashMap<String, Fetch>) -> R) -> R {
    f(FETCHES.lock().get_or_insert_with(HashMap::new))
}

fn i(v: &Value, k: &str) -> i32 {
    v.get(k).and_then(Value::as_i64).unwrap_or(0) as i32
}

fn s(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or("").to_string()
}

fn riot_id(v: &Value) -> String {
    let (n, t) = (s(v, "gameName"), s(v, "tagLine"));
    if n.is_empty() {
        String::new()
    } else if t.is_empty() {
        n
    } else {
        format!("{n}#{t}")
    }
}

fn queue_of(session: &Value) -> (i32, String, i32) {
    let q = session.pointer("/gameData/queue").cloned().unwrap_or(Value::Null);
    let map = session.pointer("/map/id").and_then(Value::as_i64).map(|m| m as i32).unwrap_or_else(|| i(&q, "mapId"));
    (i(&q, "id"), s(&q, "gameMode"), map)
}

/// Your team (and the other team's champions) from `/lol-champ-select/v1/session`.
fn roster_from_champ_select(cs: &Value, me: &str) -> Vec<Entry> {
    let local = cs.get("localPlayerCellId").and_then(Value::as_i64).unwrap_or(-1);
    let mut out = Vec::new();
    for (key, mine) in [("myTeam", true), ("theirTeam", false)] {
        for p in cs.get(key).and_then(Value::as_array).cloned().unwrap_or_default() {
            let champ = match i(&p, "championId") {
                0 => i(&p, "championPickIntent"),
                c => c,
            };
            let team = match i(&p, "team") {
                2 => 200,
                1 => 100,
                _ if mine => 100,
                _ => 200,
            };
            let puuid = s(&p, "puuid");
            out.push(Entry {
                me: (mine && p.get("cellId").and_then(Value::as_i64) == Some(local)) || (!me.is_empty() && puuid == me),
                puuid,
                champion_id: champ,
                position: s(&p, "assignedPosition"),
                spells: [i(&p, "spell1Id"), i(&p, "spell2Id")],
                team_id: team,
                name: riot_id(&p),
            });
        }
    }
    out
}

/// Everyone in the game from `/lol-gameflow/v1/session`.
fn roster_from_session(session: &Value, me: &str) -> Vec<Entry> {
    let gd = session.get("gameData").cloned().unwrap_or(Value::Null);
    let selections: Vec<Value> = gd.get("playerChampionSelections").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut out = Vec::new();
    for (key, team) in [("teamOne", 100), ("teamTwo", 200)] {
        for p in gd.get(key).and_then(Value::as_array).cloned().unwrap_or_default() {
            let puuid = s(&p, "puuid");
            let champ = i(&p, "championId");
            let sel = selections.iter().find(|x| (!puuid.is_empty() && s(x, "puuid") == puuid) || (champ > 0 && i(x, "championId") == champ));
            out.push(Entry {
                me: !me.is_empty() && puuid == me,
                puuid,
                champion_id: champ,
                position: s(&p, "selectedPosition"),
                spells: sel.map(|x| [i(x, "spell1Id"), i(x, "spell2Id")]).unwrap_or([0, 0]),
                team_id: team,
                name: riot_id(&p),
            });
        }
    }
    out
}

fn start_fetches(entries: &[Entry]) {
    let lcu = match lcu::current() {
        Some(l) => l,
        None => return,
    };
    for e in entries.iter().filter(|e| !e.puuid.is_empty()) {
        let stale = with_fetches(|m| match m.get(&e.puuid) {
            None => true,
            Some(Fetch::Loading) => false,
            Some(Fetch::Ready(_, at)) => at.elapsed() > Duration::from_secs(600),
            Some(Fetch::Failed(_, at)) => at.elapsed() > Duration::from_secs(20),
        });
        if !stale {
            continue;
        }
        with_fetches(|m| m.insert(e.puuid.clone(), Fetch::Loading));
        let (lcu, puuid) = (lcu.clone(), e.puuid.clone());
        tauri::async_runtime::spawn(async move {
            let started = Instant::now();
            let quick = match players::inputs_quick(&lcu, &puuid).await {
                Ok(q) => q,
                Err(e) => {
                    log::warn!("scouting {puuid}: {e:#}");
                    with_fetches(|m| m.insert(puuid, Fetch::Failed(format!("{e:#}"), Instant::now())));
                    return;
                }
            };
            with_fetches(|m| m.insert(puuid.clone(), Fetch::Ready(Arc::new(quick.clone()), Instant::now())));
            let history = players::history(&lcu, &puuid).await;
            log::info!("scouted {puuid} in {:.1}s{}", started.elapsed().as_secs_f32(), if history.is_err() { " (history failed)" } else { "" });
            let full = Inputs { history: history.unwrap_or(quick.history.clone()), fresh: true, ..quick };
            with_fetches(|m| m.insert(puuid, Fetch::Ready(Arc::new(full), Instant::now())));
        });
    }
}

fn build_state(phase: &str, connected: bool, roster: &Roster) -> LiveState {
    let now = players::now_ms();
    let my_team = roster.entries.iter().find(|e| e.me).map(|e| e.team_id).unwrap_or(100);
    let mut team_ids: Vec<i32> = roster.entries.iter().map(|e| e.team_id).collect();
    team_ids.sort();
    team_ids.dedup();
    team_ids.sort_by_key(|t| (*t != my_team, *t));
    let matchmade = roster.queue_id > 0;
    let mut teams = Vec::new();
    for tid in team_ids {
        let entries: Vec<&Entry> = roster.entries.iter().filter(|e| e.team_id == tid).collect();
        let mut ready: Vec<(String, i32, Vec<crate::model::GameRow>)> = Vec::new();
        let mut slots: Vec<Slot> = entries
            .iter()
            .map(|e| {
                let mut slot = Slot {
                    puuid: e.puuid.clone(),
                    champion_id: e.champion_id,
                    position: e.position.clone(),
                    spells: e.spells,
                    me: e.me,
                    name: e.name.clone(),
                    status: "loading".into(),
                    ..Default::default()
                };
                if e.puuid.is_empty() {
                    slot.status = "hidden".into();
                    return slot;
                }
                with_fetches(|m| match m.get(&e.puuid) {
                    Some(Fetch::Ready(inp, _)) => {
                        let ctx = Context {
                            champion_id: Some(e.champion_id).filter(|c| *c > 0),
                            position: Some(e.position.clone()).filter(|p| !p.is_empty()),
                            mode: Some(roster.mode.clone()).filter(|m| !m.is_empty()),
                            now_ms: now,
                        };
                        let card = inp.card(&ctx);
                        if slot.name.is_empty() {
                            slot.name = format!("{}#{}", card.summoner.game_name, card.summoner.tag_line);
                        }
                        slot.card = Some(card);
                        slot.status = "ready".into();
                        ready.push((e.puuid.clone(), e.team_id, inp.history.clone()));
                    }
                    Some(Fetch::Failed(err, _)) => {
                        slot.status = "error".into();
                        slot.error = Some(err.clone());
                    }
                    _ => {}
                });
                slot
            })
            .collect();
        if matchmade {
            let groups = analysis::premade_groups(&ready);
            for slot in &mut slots {
                if let (Some(g), Some(card)) = (groups.get(&slot.puuid), slot.card.as_mut()) {
                    card.tags.insert(0, Tag { key: "premade".into(), tone: "duo".into(), arg: Some(json!(g)) });
                }
            }
        }
        teams.push(LiveTeam { id: tid, ally: tid == my_team, players: slots });
    }
    LiveState { phase: phase.into(), connected, queue_id: roster.queue_id, mode: roster.mode.clone(), map_id: roster.map_id, teams }
}

#[derive(Default)]
struct Watch {
    phase: String,
    connected: bool,
    roster: Roster,
    last_emit: String,
    last_session_fetch: Option<Instant>,
    loading_handled: bool,
    spells_started: bool,
    imported_for: Option<i32>,
    /// Riot ID -> puuid, for games where the client's session lists no puuids.
    resolved: HashMap<String, String>,
}

fn in_game_phase(p: &str) -> bool {
    matches!(p, "GameStart" | "InProgress" | "Reconnect")
}

async fn on_connected(core: &Core) {
    let Some(lcu) = lcu::current() else { return };
    crate::positions::load(&lcu).await;
    match players::me(&lcu).await {
        Ok(me) => {
            log::info!("signed in as {}#{}", me.game_name, me.tag_line);
            *core.me.write() = Some(me);
        }
        Err(e) => log::warn!("current summoner: {e:#}"),
    }
    if let Ok(v) = lcu.get_value("/riotclient/region-locale").await {
        *core.client_locale.write() = v.get("locale").and_then(Value::as_str).map(str::to_string);
        *core.client_region.write() = v.get("region").and_then(Value::as_str).map(str::to_string);
        crate::riot::check(core.region().as_deref()).await;
    }
    let _ = core.app.emit("connection", json!({ "connected": true, "me": *core.me.read(), "language": core.language() }));
}

fn on_phase_change(core: &Core, w: &mut Watch, old: &str, new: &str) {
    log::info!("phase {old} -> {new}");
    builds::PAUSED.store(new == "ChampSelect" || in_game_phase(new), Ordering::Relaxed);
    if new == "ChampSelect" {
        w.imported_for = None;
        w.roster = Roster::default();
        if core.settings().open_in_champ_select {
            core.show_main(Some("live"));
        }
    }
    if in_game_phase(new) && !in_game_phase(old) {
        w.loading_handled = false;
        w.spells_started = false;
        w.last_session_fetch = None;
        core.register_hotkeys(true);
    }
    if !in_game_phase(new) && in_game_phase(old) {
        core.register_hotkeys(false);
        core.hide_overlay();
        core.hide_spells();
        *core.ingame.lock() = None;
    }
    if new == "EndOfGame" {
        let _ = core.app.emit("profile-stale", ());
    }
}

async fn auto_import(core: &Core, w: &mut Watch, cs: &Value) {
    let s = core.settings();
    if !(s.auto_import_runes || s.auto_import_items) {
        return;
    }
    if cs.pointer("/timer/phase").and_then(Value::as_str) != Some("FINALIZATION") {
        return;
    }
    let Some(me) = w.roster.entries.iter().find(|e| e.me) else { return };
    if me.champion_id <= 0 || w.imported_for == Some(me.champion_id) {
        return;
    }
    w.imported_for = Some(me.champion_id);
    let role = analysis_role(&me.position);
    match builds::import(me.champion_id, role, s.auto_import_runes, s.auto_import_items).await {
        Ok(r) => {
            let what: Vec<String> = [r.rune_page, r.item_set].into_iter().flatten().collect();
            let what = what.join(" + ");
            core.toast(format!("[imported:{what}] Imported {what}"), "good");
        }
        Err(e) => core.toast(format!("{e:#}"), "error"),
    }
}

/// Client position name to the build role names.
pub fn analysis_role(position: &str) -> &'static str {
    match position.to_uppercase().as_str() {
        "TOP" => "top",
        "JUNGLE" => "jungle",
        "MIDDLE" | "MID" => "mid",
        "BOTTOM" => "adc",
        "UTILITY" => "support",
        _ => "",
    }
}

/// When the client's game session lists no puuids (or the client is closed), match players
/// by the Riot IDs the game reports.
async fn resolve_from_game(w: &mut Watch, game: &live::InGame) {
    let Some(lcu) = lcu::current() else { return };
    if w.roster.entries.iter().any(|e| !e.puuid.is_empty()) {
        return;
    }
    let mut entries = Vec::new();
    for p in &game.players {
        if !w.resolved.contains_key(&p.riot_id) && p.riot_id.contains('#') {
            if let Ok(puuid) = players::lookup(&lcu, &p.riot_id).await {
                w.resolved.insert(p.riot_id.clone(), puuid);
            }
        }
        entries.push(Entry {
            puuid: w.resolved.get(&p.riot_id).cloned().unwrap_or_default(),
            champion_id: 0,
            position: String::new(),
            spells: [0, 0],
            team_id: if p.team == "ORDER" { 100 } else { 200 },
            me: p.me,
            name: p.riot_id.clone(),
        });
    }
    if !entries.is_empty() {
        w.roster.entries = entries;
    }
}

async fn tick(core: &Arc<Core>, w: &mut Watch) {
    let connected = lcu::refresh();
    if connected && !w.connected {
        on_connected(core).await;
    } else if !connected && w.connected {
        *core.me.write() = None;
        let _ = core.app.emit("connection", json!({ "connected": false, "me": null, "language": core.language() }));
    }
    w.connected = connected;

    let lcu = lcu::current();
    let mut phase = match &lcu {
        Some(l) => l.get_value("/lol-gameflow/v1/gameflow-phase").await.ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default(),
        None => String::new(),
    };
    let game_running = win::game_running();
    if phase.is_empty() && game_running {
        // Client closed during the game: the game itself still answers.
        phase = "InProgress".into();
    }
    if phase != w.phase {
        let old = std::mem::replace(&mut w.phase, phase.clone());
        on_phase_change(core, w, &old, &phase);
    }
    let me = core.me.read().as_ref().map(|m| m.puuid.clone()).unwrap_or_default();

    if phase == "ChampSelect" {
        if let Some(l) = &lcu {
            if let Ok(Some(cs)) = l.get_opt("/lol-champ-select/v1/session").await {
                let (queue_id, mode, map_id) = match l.get_opt("/lol-gameflow/v1/session").await {
                    Ok(Some(gs)) => queue_of(&gs),
                    _ => (0, String::new(), 0),
                };
                w.roster = Roster { queue_id, mode, map_id, entries: roster_from_champ_select(&cs, &me) };
                auto_import(core, w, &cs).await;
            }
        }
    } else if in_game_phase(&phase) {
        let due = w.last_session_fetch.is_none_or(|t| t.elapsed() > Duration::from_secs(5));
        if due {
            w.last_session_fetch = Some(Instant::now());
            if let Some(l) = &lcu {
                if let Ok(Some(gs)) = l.get_opt("/lol-gameflow/v1/session").await {
                    let entries = roster_from_session(&gs, &me);
                    let (queue_id, mode, map_id) = queue_of(&gs);
                    if !entries.is_empty() {
                        w.roster = Roster { queue_id, mode, map_id, entries };
                    }
                }
            }
        }
        let game = live::fetch().await;
        if let Some(g) = &game {
            resolve_from_game(w, g).await;
            if w.roster.mode.is_empty() {
                w.roster.mode = g.mode.clone();
            }
        }
        // Loading screen: show the panel by itself until the match starts.
        let game_time = game.as_ref().map(|g| g.game_time).unwrap_or(0.0);
        let s = core.settings();
        if !w.loading_handled && win::game_window().is_some() {
            w.loading_handled = true;
            if s.show_during_loading && game_time < 2.0 {
                core.overlay_auto.store(true, Ordering::Relaxed);
                core.show_overlay();
            }
        }
        if game_time >= 2.0 {
            if core.overlay_auto.load(Ordering::Relaxed) {
                core.hide_overlay();
            }
            if s.spells_on_start && !w.spells_started {
                w.spells_started = true;
                core.show_spells();
            }
        }
        if let Some(g) = &game {
            let _ = core.app.emit("ingame", g);
        }
        *core.ingame.lock() = game;
    }

    start_fetches(&w.roster.entries);
    let state = build_state(&phase, connected, &w.roster);
    let json = serde_json::to_string(&state).unwrap_or_default();
    if json != w.last_emit {
        w.last_emit = json;
        let _ = core.app.emit("live", &state);
        *core.live.lock() = state;
    }
}

pub fn spawn(core: Arc<Core>) {
    tauri::async_runtime::spawn(async move {
        let mut w = Watch::default();
        loop {
            let started = Instant::now();
            tick(&core, &mut w).await;
            let elapsed = started.elapsed();
            tokio::time::sleep(Duration::from_millis(1000).saturating_sub(elapsed).max(Duration::from_millis(200))).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn champ_select_roster() {
        let cs = json!({
            "localPlayerCellId": 2,
            "myTeam": [
                {"cellId": 1, "puuid": "a", "championId": 0, "championPickIntent": 64, "assignedPosition": "jungle", "team": 1, "spell1Id": 4, "spell2Id": 11},
                {"cellId": 2, "puuid": "me", "championId": 103, "assignedPosition": "middle", "team": 1, "gameName": "Me", "tagLine": "NA1"}
            ],
            "theirTeam": [{"cellId": 6, "puuid": "", "championId": 0, "team": 2}]
        });
        let r = roster_from_champ_select(&cs, "me");
        assert_eq!(r.len(), 3);
        assert_eq!(r[0].champion_id, 64, "hover intent counts until locked");
        assert!(r[1].me && !r[0].me);
        assert_eq!(r[1].name, "Me#NA1");
        assert_eq!(r[2].team_id, 200);
        assert!(r[2].puuid.is_empty());
    }

    #[test]
    fn session_roster_and_state() {
        let gs = json!({
            "gameData": {
                "queue": {"id": 420, "gameMode": "CLASSIC", "mapId": 11},
                "teamOne": [{"puuid": "x", "championId": 1, "selectedPosition": "TOP"}],
                "teamTwo": [{"puuid": "me", "championId": 2, "selectedPosition": "MIDDLE"}],
                "playerChampionSelections": [{"puuid": "me", "championId": 2, "spell1Id": 4, "spell2Id": 14}]
            }
        });
        let entries = roster_from_session(&gs, "me");
        assert_eq!(entries[1].spells, [4, 14]);
        let (q, mode, map) = queue_of(&gs);
        assert_eq!((q, mode.as_str(), map), (420, "CLASSIC", 11));
        let st = build_state("InProgress", true, &Roster { queue_id: q, mode, map_id: map, entries });
        assert_eq!(st.teams.len(), 2);
        assert!(st.teams[0].ally, "your team first");
        assert_eq!(st.teams[0].id, 200);
        assert_eq!(st.teams[0].players[0].status, "loading");
    }
}
