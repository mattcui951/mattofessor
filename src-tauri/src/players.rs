//! Player lookups through the League client, backed by the local cache.

use crate::analysis::{self, Context, PlayerCard};
use crate::lcu::Lcu;
use crate::model::{self, GameRow, Mastery, Ranked, Summoner};
use crate::store;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

/// Recent games kept per player for statistics (the client returns up to 40 at a time;
/// older cached games from earlier lookups widen the window).
const HISTORY: usize = 60;

pub async fn me(lcu: &Lcu) -> Result<Summoner> {
    Ok(model::summoner_from(&lcu.get_value("/lol-summoner/v1/current-summoner").await?))
}

pub async fn summoner(lcu: &Lcu, puuid: &str) -> Result<Summoner> {
    Ok(model::summoner_from(&lcu.get_value(&format!("/lol-summoner/v2/summoners/puuid/{puuid}")).await?))
}

/// Riot ID ("Name#TAG") to puuid.
pub async fn lookup(lcu: &Lcu, riot_id: &str) -> Result<String> {
    // Chinese input types a full-width ＃.
    let riot_id = riot_id.replace('＃', "#");
    let (name, tag) = riot_id.rsplit_once('#').ok_or_else(|| anyhow!("[bad-riot-id] Use the full Riot ID, like Name#NA1"))?;
    let (name, tag) = (name.trim(), tag.trim());
    if name.is_empty() || tag.is_empty() {
        return Err(anyhow!("[bad-riot-id] Use the full Riot ID, like Name#NA1"));
    }
    let v = lcu
        .get_value(&format!(
            "/lol-summoner/v1/alias/lookup?gameName={}&tagLine={}",
            urlencoding::encode(name),
            urlencoding::encode(tag)
        ))
        .await
        .map_err(|e| if crate::lcu::is_not_found(&e) { anyhow!("[no-player:{name}#{tag}] No player named {name}#{tag}") } else { e })?;
    v.get("puuid").and_then(Value::as_str).filter(|p| !p.is_empty()).map(str::to_string).ok_or_else(|| anyhow!("[no-player:{name}#{tag}] No player named {name}#{tag}"))
}

pub async fn ranked(lcu: &Lcu, puuid: &str) -> Result<Ranked> {
    let r = model::ranked_from(&lcu.get_value(&format!("/lol-ranked/v1/ranked-stats/{puuid}")).await?);
    let day = chrono::Local::now().format("%Y-%m-%d").to_string();
    for q in [&r.solo, &r.flex].into_iter().flatten() {
        let _ = store::get().snapshot_rank(puuid, &day, q);
    }
    Ok(r)
}

/// The client's ranked stats, completed with exact wins and losses when a Riot API key is set.
async fn exact(ranked: Result<Ranked>, puuid: &str, summ: &Summoner) -> Ranked {
    let mut r = ranked.unwrap_or_else(|e| {
        log::warn!("ranked for {puuid}: {e:#}");
        Ranked::default()
    });
    let region = crate::core::get().and_then(|c| c.region());
    crate::riot::complete(&mut r, &summ.game_name, &summ.tag_line, region.as_deref()).await;
    r
}

/// Fresh games from the client merged with cached ones, most recent first, and whether the
/// client answered. When it cannot, the cached games come back alone (an error only when
/// nothing is cached): right after a game the client often answers 500 for a while.
pub async fn history(lcu: &Lcu, puuid: &str) -> Result<(Vec<GameRow>, bool)> {
    let fetched = lcu.get_value(&format!("/lol-match-history/v1/products/lol/{puuid}/matches?begIndex=0&endIndex=39")).await;
    let st = store::get();
    let answered = match fetched {
        Ok(v) => {
            let rows = model::rows_from_history(&v);
            st.save_rows(puuid, &rows)?;
            true
        }
        Err(e) => {
            let cached = st.rows(puuid, HISTORY)?;
            if cached.is_empty() {
                return Err(e);
            }
            log::warn!("history for {puuid} from cache: {e:#}");
            false
        }
    };
    Ok((st.rows(puuid, HISTORY)?, answered))
}

/// How long after a game ends `follow_finished_game` keeps looking for it.
const FOLLOW_FOR: Duration = Duration::from_secs(10 * 60);
const FOLLOW_EVERY: Duration = Duration::from_secs(15);

/// A game just ended. The client adds it to the match history only after a while (and answers
/// 500 for a bit before that), so the refresh at the end of the game does not have it yet. Ask
/// until it is there, then tell the pages (`profile-stale`). The full game is often readable
/// before the list has it: its row goes in at once.
pub async fn follow_finished_game(app: AppHandle, puuid: String, game_id: u64) {
    let started = Instant::now();
    let have = || store::get().rows(&puuid, HISTORY).is_ok_and(|r| r.iter().any(|g| g.game_id == game_id));
    while started.elapsed() < FOLLOW_FOR {
        tokio::time::sleep(FOLLOW_EVERY).await;
        let Some(lcu) = crate::lcu::current() else { return };
        let found = match history(&lcu, &puuid).await {
            Ok((rows, _)) if rows.iter().any(|g| g.game_id == game_id) => "match history",
            _ => match game(game_id).await.ok().and_then(|g| model::row_for(&g, &puuid)) {
                Some(row) => {
                    let _ = store::get().save_rows(&puuid, &[row]);
                    "full game"
                }
                None if have() => "cache",
                None => continue,
            },
        };
        log::info!("finished game {game_id} in the history after {:.0}s (from the {found})", started.elapsed().as_secs_f32());
        let _ = app.emit("profile-stale", ());
        return;
    }
    log::warn!("finished game {game_id} still not in the match history after {} min", FOLLOW_FOR.as_secs() / 60);
}

pub async fn masteries(lcu: &Lcu, puuid: &str) -> Vec<Mastery> {
    match lcu.get_value(&format!("/lol-champion-mastery/v1/{puuid}/champion-mastery")).await {
        Ok(v) => model::masteries_from(&v),
        Err(e) => {
            log::warn!("mastery for {puuid}: {e:#}");
            Vec::new()
        }
    }
}

/// Everything fetched about one player; tags are computed from it per game context.
#[derive(Clone, Debug, Default)]
pub struct Inputs {
    pub summoner: Summoner,
    pub ranked: Ranked,
    pub history: Vec<GameRow>,
    pub masteries: Vec<Mastery>,
    /// The history was just fetched (not only what the cache had).
    pub fresh: bool,
    /// The client answered for the games (false: they are only the ones saved before).
    pub history_ok: bool,
}

impl Inputs {
    pub fn card(&self, ctx: &Context) -> PlayerCard {
        let mut card = analysis::build_card(self.summoner.clone(), self.ranked.clone(), &self.history, &self.masteries, ctx);
        card.fresh = self.fresh;
        card
    }
}

/// The fast part of a lookup: summoner, rank and mastery (each well under a second), with the
/// games already in the cache. The client serves other players' histories one at a time
/// (about 5 s each), so those follow separately.
pub async fn inputs_quick(lcu: &Lcu, puuid: &str) -> Result<Inputs> {
    let (summ, ranked, masteries) = tokio::join!(summoner(lcu, puuid), ranked(lcu, puuid), masteries(lcu, puuid));
    let summ = summ?;
    let ranked = exact(ranked, puuid, &summ).await;
    Ok(Inputs {
        summoner: summ,
        ranked,
        history: store::get().rows(puuid, HISTORY).unwrap_or_default(),
        masteries,
        fresh: false,
        history_ok: false,
    })
}

pub async fn inputs(lcu: &Lcu, puuid: &str) -> Result<Inputs> {
    let (summ, ranked, history, masteries) =
        tokio::join!(summoner(lcu, puuid), ranked(lcu, puuid), history(lcu, puuid), masteries(lcu, puuid));
    let (history, history_ok) = match history {
        Ok((rows, answered)) => (rows, answered),
        Err(e) => {
            log::warn!("history for {puuid}: {e:#}");
            (Vec::new(), false)
        }
    };
    // A rank or history failure leaves those parts empty rather than hiding the player.
    let summ = summ?;
    let ranked = exact(ranked, puuid, &summ).await;
    let inputs = Inputs {
        summoner: summ,
        ranked,
        history,
        masteries,
        fresh: true,
        history_ok,
    };
    let saved = Saved { summoner: inputs.summoner.clone(), ranked: inputs.ranked.clone(), masteries: inputs.masteries.clone() };
    if let Ok(v) = serde_json::to_value(&saved) {
        let _ = store::get().save_player(puuid, &v, now_ms());
    }
    Ok(inputs)
}

/// What a lookup learns besides the games (those have their own table).
#[derive(Serialize, Deserialize)]
struct Saved {
    summoner: Summoner,
    ranked: Ranked,
    masteries: Vec<Mastery>,
}

/// The last full lookup of a player from the local cache, with when it was made (ms); no
/// client needed. Finished games never change, so this is shown as is and refreshed behind.
pub fn inputs_saved(puuid: &str) -> Option<(Inputs, i64)> {
    let (v, updated) = store::get().player(puuid)?;
    let saved: Saved = serde_json::from_value(v).ok()?;
    let history = store::get().rows(puuid, HISTORY).unwrap_or_default();
    Some((Inputs { summoner: saved.summoner, ranked: saved.ranked, history, masteries: saved.masteries, fresh: false, history_ok: true }, updated))
}

/// Full game (all 10 players), cached forever; the client is asked only for games not seen yet.
pub async fn game(game_id: u64) -> Result<Value> {
    if let Some(g) = store::get().game(game_id) {
        return Ok(g);
    }
    let lcu = crate::lcu::require()?;
    let g = lcu.get_value(&format!("/lol-match-history/v1/games/{game_id}")).await?;
    if g.get("participants").and_then(Value::as_array).is_some_and(|p| !p.is_empty()) {
        let _ = store::get().save_game(game_id, &g);
    }
    Ok(g)
}

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
