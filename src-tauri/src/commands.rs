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
        "version": core.app.package_info().version.to_string(),
        "connected": lcu::current().is_some(),
        "me": *core.me.read(),
        "language": core.language(),
        "riotStatus": crate::riot::status(),
        "update": crate::update::state(),
        "updatedFrom": crate::update::updated_from(),
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
    /// When this data was fetched from the client (ms).
    pub updated_at: i64,
}

fn profile_of(inputs: players::Inputs, puuid: &str, is_me: bool, updated_at: i64) -> Profile {
    let card = inputs.card(&Context { now_ms: players::now_ms(), classes: dd::classes(), ..Default::default() });
    let mut masteries = inputs.masteries.clone();
    masteries.sort_by_key(|m| std::cmp::Reverse(m.points));
    masteries.truncate(10);
    Profile { card, history: inputs.history, rank_history: store::get().rank_history(puuid).unwrap_or_default(), masteries, is_me, updated_at }
}

/// The signed-in player's puuid, or the last one signed in (kept across restarts, so a saved
/// profile shows before the client is up).
fn me_puuid(core: &Core) -> Option<String> {
    core.me.read().as_ref().map(|m| m.puuid.clone()).or_else(|| store::get().kv_get("last_me"))
}

/// A player's profile from the local cache only (instant, works without the client);
/// `None` when this player was never looked up.
#[tauri::command]
pub fn profile_saved(core: State<'_, Arc<Core>>, puuid: Option<String>) -> Option<Profile> {
    saved_profile(&core, puuid)
}

pub fn saved_profile(core: &Core, puuid: Option<String>) -> Option<Profile> {
    let me = me_puuid(core);
    let puuid = puuid.filter(|p| !p.is_empty()).or(me.clone())?;
    let (inputs, updated) = players::inputs_saved(&puuid)?;
    Some(profile_of(inputs, &puuid, me.as_deref() == Some(&puuid), updated))
}

/// A first look at a player not seen before: summoner, rank and mastery, plus any of their games
/// already cached (well under a second). `profile` brings the games; the client takes about
/// 5 s for a new player's history. `updated_at` is 0: this is not a full lookup.
#[tauri::command]
pub async fn profile_quick(core: State<'_, Arc<Core>>, puuid: String) -> Res<Profile> {
    quick_profile(&core, &puuid).await.map_err(err)
}

pub async fn quick_profile(core: &Core, puuid: &str) -> anyhow::Result<Profile> {
    let lcu = lcu::require()?;
    let inputs = players::inputs_quick(&lcu, puuid).await?;
    Ok(profile_of(inputs, puuid, me_puuid(core).as_deref() == Some(puuid), 0))
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
            let _ = core.app.emit("connection", json!({ "connected": true, "me": m, "language": core.language() }));
            m
        }
    };
    store::get().kv_set("last_me", &me.puuid);
    let puuid = puuid.filter(|p| !p.is_empty()).unwrap_or_else(|| me.puuid.clone());
    let inputs = players::inputs(&lcu, &puuid).await?;
    Ok(profile_of(inputs, &puuid, puuid == me.puuid, players::now_ms()))
}

/// Riot ID ("Name#TAG") to puuid.
#[tauri::command]
pub async fn search(riot_id: String) -> Res<String> {
    let lcu = lcu::require().map_err(err)?;
    players::lookup(&lcu, &riot_id).await.map_err(err)
}

#[tauri::command]
pub async fn game(game_id: u64) -> Res<Value> {
    players::game(game_id).await.map_err(err)
}

/// Every player's score in one game (MVP / SVP marked).
#[tauri::command]
pub async fn game_scores(game_id: u64) -> Res<Vec<crate::score::PlayerScore>> {
    let g = players::game(game_id).await.map_err(err)?;
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
    let mut out = Vec::new();
    for id in game_ids {
        let Ok(g) = players::game(id).await else { continue };
        let scores = crate::score::score_game(&g);
        if let Some(s) = scores.iter().find(|s| s.puuid == puuid) {
            out.push(MyScore { game_id: id, score: s.score, badge: s.badge.clone(), place: s.place, players: scores.len() as u32 });
        }
    }
    Ok(out)
}

/// OP.GG lane tier lists (Summoner's Rift, ranked).
#[tauri::command]
pub async fn rift_tiers() -> Res<Arc<crate::opgg::Tiers>> {
    crate::opgg::tiers().await.map_err(err)
}

/// Hextech ARAM champion and augment win rates for a skill bracket ("all", "casual", "mid", "high").
#[tauri::command]
pub async fn aram_overview(bracket: String) -> Res<crate::aramhex::Overview> {
    let (info, b) = crate::aramhex::bracket(&bracket).await.map_err(err)?;
    Ok(crate::aramhex::overview(&info, &bracket, &b))
}

/// One champion's Hextech ARAM augments, best first.
#[tauri::command]
pub async fn aram_champion(champion_id: i32, bracket: String) -> Res<crate::aramhex::Champion> {
    let (info, b) = crate::aramhex::bracket(&bracket).await.map_err(err)?;
    Ok(crate::aramhex::champion(&info, &bracket, &b, champion_id))
}

/// Augment recommendations for a champion in Hextech ARAM, by rarity.
#[tauri::command]
pub async fn aram_picks(champion_id: i32, bracket: String) -> Res<crate::aramhex::Picks> {
    crate::aramhex::champion_picks(champion_id, &bracket).await.map_err(err)
}

/// The augment cards on screen now, with your champion's numbers (Hextech ARAM).
#[tauri::command]
pub fn card_offer() -> Option<crate::cards::Offer> {
    crate::cards::current()
}

/// Augment names, rarity and icons in a language ("en" or "zh").
#[tauri::command]
pub async fn augment_names(lang: String) -> Res<crate::augments::Augments> {
    crate::augments::all(&lang).await.map_err(err)
}

#[tauri::command]
pub async fn build(champion_id: i32, role: String, mode: String) -> Res<Option<builds::Build>> {
    builds::build(champion_id, &role, &mode).await.map_err(err)
}

#[tauri::command]
pub async fn import_build(champion_id: i32, role: String, mode: String, runes: bool, items: bool) -> Res<builds::ImportResult> {
    builds::import(champion_id, &role, &mode, runes, items).await.map_err(err)
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

/// Runs off the main thread (`async`): this can create the window, and on Windows creating a
/// window inside a plain command (a call from another window's page) freezes the app.
#[tauri::command(async)]
pub fn toggle_overlay(core: State<'_, Arc<Core>>) {
    core.toggle_overlay();
}

#[tauri::command]
pub fn fit_overlay(core: State<'_, Arc<Core>>, height: f64) {
    core.fit_overlay(height);
}

#[tauri::command]
pub fn hide_overlay(core: State<'_, Arc<Core>>) {
    core.close_overlay();
}

/// Runs off the main thread (`async`): this can create the window, and on Windows creating a
/// window inside a plain command (a call from another window's page) freezes the app.
#[tauri::command(async)]
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

/// Whether this PC can read the augment cards in the game's language.
#[tauri::command]
pub fn card_reader_status(core: State<'_, Arc<Core>>) -> crate::cards::ReaderStatus {
    crate::cards::status(&core)
}

/// League's window mode: "fullscreen", "windowed", "borderless" or "unknown" (`display`).
#[tauri::command]
pub async fn league_window_mode() -> Option<&'static str> {
    crate::display::window_mode().await
}

/// Switch League to Borderless so the in-game windows can show over it.
#[tauri::command]
pub async fn set_borderless(core: State<'_, Arc<Core>>) -> Res<()> {
    crate::display::set_borderless().await.map_err(err)?;
    core.clear_fullscreen_tried();
    Ok(())
}

/// The player closed the "panel stayed closed" notice.
#[tauri::command]
pub fn dismiss_fullscreen_tried(core: State<'_, Arc<Core>>) {
    core.clear_fullscreen_tried();
}

/// The in-game panel was asked for during a Full screen game (and kept closed).
#[tauri::command]
pub fn fullscreen_tried(core: State<'_, Arc<Core>>) -> bool {
    core.fullscreen_tried()
}

/// A page's error (a crash while drawing it, an uncaught exception) into app.log, which otherwise
/// only sees the app's side.
#[tauri::command]
pub fn log_error(window: tauri::WebviewWindow, message: String) {
    log::error!("page {}: {}", window.label(), message.chars().take(2000).collect::<String>());
}

/// The page the main window was asked to show and has not shown yet; nothing for other windows.
#[tauri::command]
pub fn take_main_route(core: State<'_, Arc<Core>>, window: tauri::WebviewWindow) -> Option<String> {
    if window.label() == "main" {
        core.take_main_route()
    } else {
        None
    }
}

/// Open a player's profile in the main window (from the overlay).
/// Runs off the main thread (`async`): this can create the window, and on Windows creating a
/// window inside a plain command (a call from another window's page) freezes the app.
#[tauri::command(async)]
pub fn open_profile(core: State<'_, Arc<Core>>, puuid: String) {
    core.show_main(Some(&format!("profile/{puuid}")));
}

/// Open a champion's build in the main window (from the overlay). `mode` "rift" or "aram".
#[tauri::command(async)]
pub fn open_build(core: State<'_, Arc<Core>>, champion_id: i32, mode: String) {
    let mode = if mode == "aram" { "aram" } else { "rift" };
    core.show_main(Some(&format!("builds/{champion_id}/{mode}")));
}

/// The admin's builds and tier lists in use.
#[tauri::command]
pub fn content() -> crate::content::Content {
    crate::content::current()
}

/// Whether the admin's last edits are published.
#[tauri::command]
pub fn content_status() -> crate::content::Status {
    crate::content::status()
}

/// Apply an admin edit (a build or a tier list) and publish it shortly. Admin only: admin mode on,
/// in the copy whose GitHub account can push to the repository.
#[tauri::command]
pub fn edit_content(core: State<'_, Arc<Core>>, edit: crate::content::Edit) -> Res<crate::content::Content> {
    if !core.settings().admin {
        return Err("Admin mode is off".into());
    }
    crate::content::edit(edit).map_err(err)
}

/// Try publishing the admin's edits again now.
#[tauri::command]
pub fn publish_content() {
    crate::content::schedule_publish();
}

/// Hextech ARAM item win rates over all champions.
#[tauri::command]
pub async fn aram_items(bracket: String) -> Res<crate::aramhex::ItemOverview> {
    crate::aramhex::item_overview(&bracket).await.map_err(err)
}

/// Save the Riot API key (empty removes it); returns how Riot answered.
#[tauri::command]
pub async fn set_riot_key(core: State<'_, Arc<Core>>, key: String) -> Res<crate::riot::KeyStatus> {
    let st = crate::riot::set_key(&key, core.region().as_deref()).await.map_err(err)?;
    let _ = core.app.emit("riot", st);
    Ok(st)
}

/// Look for a newer version now (Settings button).
#[tauri::command]
pub async fn check_update(core: State<'_, Arc<Core>>) -> Res<crate::update::UpdateState> {
    Ok(crate::update::check(&core).await)
}

/// Download and install the update found by the last check; the app restarts.
#[tauri::command]
pub async fn install_update(core: State<'_, Arc<Core>>) -> Res<()> {
    crate::update::install(&core).await.map_err(err)
}

/// How many augment card samples are saved.
#[tauri::command]
pub fn capture_info() -> Value {
    json!({ "count": crate::capture::count(), "dir": crate::capture::root().to_string_lossy() })
}

/// Show the samples folder in Explorer.
#[tauri::command]
pub fn open_captures(core: State<'_, Arc<Core>>) -> Res<()> {
    use tauri_plugin_opener::OpenerExt;
    let dir = crate::capture::root();
    let _ = std::fs::create_dir_all(&dir);
    core.app.opener().open_path(dir.to_string_lossy(), None::<&str>).map_err(|e| e.to_string())
}

/// Delete every saved augment card sample.
#[tauri::command]
pub fn delete_captures() -> Res<()> {
    let dir = crate::capture::root();
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn quit_app(core: State<'_, Arc<Core>>) {
    core.quit();
}
