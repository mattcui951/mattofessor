//! Commands the UI calls.

use crate::analysis::{Context, PlayerCard};
use crate::core::Core;
use crate::model::{GameRow, Mastery};
use crate::settings::Settings;
use crate::store::RankPoint;
use crate::{builds, dd, lcu, players, store};
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tauri::{Emitter, State};

type Res<T> = Result<T, String>;

fn err(e: anyhow::Error) -> String {
    format!("{e:#}")
}

#[tauri::command]
pub fn bootstrap(core: State<'_, Arc<Core>>) -> Value {
    boot_value(&core)
}

pub fn boot_value(core: &Core) -> Value {
    json!({
        "settings": core.settings(),
        "version": env!("CARGO_PKG_VERSION"),
        "connected": lcu::current().is_some(),
        "me": *core.me.read(),
        "language": core.language(),
        "riotStatus": crate::riot::status(),
        "live": *core.live.lock(),
        "ingame": *core.ingame.lock(),
    })
}

#[tauri::command]
pub async fn static_data(lang: String) -> Res<Arc<dd::StaticData>> {
    dd::get(&lang).await.map_err(err)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub card: PlayerCard,
    pub history: Vec<GameRow>,
    pub rank_history: Vec<RankPoint>,
    pub masteries: Vec<Mastery>,
    pub is_me: bool,
}

/// A player's profile; `None` means the signed-in player.
#[tauri::command]
pub async fn profile(core: State<'_, Arc<Core>>, puuid: Option<String>) -> Res<Profile> {
    load_profile(&core, puuid).await.map_err(err)
}

pub async fn load_profile(core: &Core, puuid: Option<String>) -> anyhow::Result<Profile> {
    let lcu = lcu::require()?;
    let cached = core.me.read().clone();
    let me = match cached {
        Some(m) => m,
        None => {
            let m = players::me(&lcu).await?;
            *core.me.write() = Some(m.clone());
            m
        }
    };
    let puuid = puuid.filter(|p| !p.is_empty()).unwrap_or_else(|| me.puuid.clone());
    let inputs = players::inputs(&lcu, &puuid).await?;
    let card = inputs.card(&Context { now_ms: players::now_ms(), ..Default::default() });
    let mut masteries = inputs.masteries.clone();
    masteries.sort_by_key(|m| std::cmp::Reverse(m.points));
    masteries.truncate(10);
    Ok(Profile {
        card,
        history: inputs.history,
        rank_history: store::get().rank_history(&puuid).unwrap_or_default(),
        masteries,
        is_me: puuid == me.puuid,
    })
}

/// Riot ID ("Name#TAG") to puuid.
#[tauri::command]
pub async fn search(riot_id: String) -> Res<String> {
    let lcu = lcu::require().map_err(err)?;
    players::lookup(&lcu, &riot_id).await.map_err(err)
}

#[tauri::command]
pub async fn game(game_id: u64) -> Res<Value> {
    let lcu = lcu::require().map_err(err)?;
    players::game(&lcu, game_id).await.map_err(err)
}

/// Every player's score in one game (MVP / SVP marked).
#[tauri::command]
pub async fn game_scores(game_id: u64) -> Res<Vec<crate::score::PlayerScore>> {
    let lcu = lcu::require().map_err(err)?;
    let g = players::game(&lcu, game_id).await.map_err(err)?;
    Ok(crate::score::score_game(&g))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MyScore {
    pub game_id: u64,
    pub score: f32,
    pub badge: String,
    pub place: u32,
    pub players: u32,
}

/// One player's score in each of these games (games without a score are left out).
#[tauri::command]
pub async fn player_scores(puuid: String, game_ids: Vec<u64>) -> Res<Vec<MyScore>> {
    let lcu = lcu::require().map_err(err)?;
    let mut out = Vec::new();
    for id in game_ids {
        let Ok(g) = players::game(&lcu, id).await else { continue };
        let scores = crate::score::score_game(&g);
        if let Some(s) = scores.iter().find(|s| s.puuid == puuid) {
            out.push(MyScore { game_id: id, score: s.score, badge: s.badge.clone(), place: s.place, players: scores.len() as u32 });
        }
    }
    Ok(out)
}

#[tauri::command]
pub fn live_state(core: State<'_, Arc<Core>>) -> Value {
    live_value(&core)
}

pub fn live_value(core: &Core) -> Value {
    json!({ "live": *core.live.lock(), "ingame": *core.ingame.lock() })
}

#[tauri::command]
pub async fn build(champion_id: i32, role: String) -> Res<Option<builds::Build>> {
    builds::build(champion_id, &role).await.map_err(err)
}

#[tauri::command]
pub async fn import_build(champion_id: i32, role: String, runes: bool, items: bool) -> Res<builds::ImportResult> {
    builds::import(champion_id, &role, runes, items).await.map_err(err)
}

#[tauri::command]
pub fn crawl_status() -> builds::CrawlStatus {
    builds::status()
}

#[tauri::command]
pub fn start_crawl() {
    tauri::async_runtime::spawn(async {
        if let Err(e) = builds::crawl().await {
            log::warn!("build crawl: {e:#}");
        }
    });
}

#[tauri::command]
pub fn save_settings(core: State<'_, Arc<Core>>, settings: Settings) -> Res<()> {
    #[cfg(not(debug_assertions))]
    {
        use tauri_plugin_autostart::ManagerExt;
        let al = core.app.autolaunch();
        let _ = if settings.start_with_windows { al.enable() } else { al.disable() };
    }
    core.set_settings(settings).map_err(err)
}

#[tauri::command]
pub fn toggle_overlay(core: State<'_, Arc<Core>>) {
    core.toggle_overlay();
}

#[tauri::command]
pub fn fit_overlay(core: State<'_, Arc<Core>>, height: f64) {
    core.fit_overlay(height);
}

#[tauri::command]
pub fn hide_overlay(core: State<'_, Arc<Core>>) {
    core.hide_overlay();
}

#[tauri::command]
pub fn toggle_spells(core: State<'_, Arc<Core>>) {
    core.toggle_spells();
}

#[tauri::command]
pub fn fit_spells(core: State<'_, Arc<Core>>, height: f64) {
    core.fit_spells(height);
}

#[tauri::command]
pub fn hide_spells(core: State<'_, Arc<Core>>) {
    core.hide_spells();
}

/// Open a player's profile in the main window (from the overlay).
#[tauri::command]
pub fn open_profile(core: State<'_, Arc<Core>>, puuid: String) {
    core.show_main(Some(&format!("profile/{puuid}")));
}

/// Save the Riot API key (empty removes it); returns how Riot answered.
#[tauri::command]
pub async fn set_riot_key(core: State<'_, Arc<Core>>, key: String) -> Res<crate::riot::KeyStatus> {
    let st = crate::riot::set_key(&key, core.region().as_deref()).await.map_err(err)?;
    let _ = core.app.emit("riot", st);
    Ok(st)
}

#[tauri::command]
pub fn quit_app(core: State<'_, Arc<Core>>) {
    core.quit();
}
