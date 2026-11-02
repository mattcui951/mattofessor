//! Champion builds from high-elo games, and importing them into the client.
//!
//! The crawler reads the Challenger and Grandmaster solo queue ladders from the League client
//! and each player's recent ranked games (their own line: champion, final items, runes,
//! spells, result). Builds are the most common choices on a champion in the last 21 days.
//! The client has no item purchase order, so a build lists items by popularity.
//! Crawling is slow on purpose (about one request a second) and pauses in champ select and
//! in game.

use crate::lcu::{self, Lcu};
use crate::model::{self, GameRow};
use crate::store::{self, BuildRow};
use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

const WINDOW_DAYS: i64 = 21;
const RECRAWL_HOURS: i64 = 20;
const LADDER_PLAYERS: usize = 500;

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrawlStatus {
    pub running: bool,
    pub done: u32,
    pub total: u32,
    pub games: u64,
    pub champions: u64,
    pub last_finished: Option<i64>,
    pub error: Option<String>,
}

static STATUS: Mutex<CrawlStatus> = Mutex::new(CrawlStatus {
    running: false,
    done: 0,
    total: 0,
    games: 0,
    champions: 0,
    last_finished: None,
    error: None,
});

/// Set while the player is in champ select or in game: the crawler waits.
pub static PAUSED: AtomicBool = AtomicBool::new(false);

pub fn status() -> CrawlStatus {
    let mut s = STATUS.lock().clone();
    let (g, c) = store::get().build_sample(since());
    s.games = g;
    s.champions = c;
    s.last_finished = store::get().kv_get("crawl_finished").and_then(|v| v.parse().ok());
    s
}

fn since() -> i64 {
    crate::players::now_ms() - WINDOW_DAYS * 86_400_000
}

async fn ladder(lcu: &Lcu) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for tier in ["CHALLENGER", "GRANDMASTER"] {
        let v = lcu.get_value(&format!("/lol-ranked/v1/apex-leagues/RANKED_SOLO_5x5/{tier}")).await?;
        let standings = v.pointer("/divisions/0/standings").and_then(Value::as_array).cloned().unwrap_or_default();
        out.extend(standings.iter().filter_map(|s| s.get("puuid").and_then(Value::as_str).map(str::to_string)));
    }
    out.truncate(LADDER_PLAYERS);
    Ok(out)
}

/// One full pass over the ladder. Players crawled in the last 20 hours are skipped.
pub async fn crawl() -> Result<()> {
    {
        let mut s = STATUS.lock();
        if s.running {
            return Ok(());
        }
        s.running = true;
        s.error = None;
    }
    let result = crawl_inner().await;
    let mut s = STATUS.lock();
    s.running = false;
    match &result {
        Ok(()) => store::get().kv_set("crawl_finished", &crate::players::now_ms().to_string()),
        Err(e) => s.error = Some(format!("{e:#}")),
    }
    result
}

async fn crawl_inner() -> Result<()> {
    let lcu = lcu::require()?;
    crate::positions::load(&lcu).await;
    let players = ladder(&lcu).await?;
    let st = store::get();
    st.prune_build_rows(crate::players::now_ms() - 30 * 86_400_000);
    {
        let mut s = STATUS.lock();
        s.total = players.len() as u32;
        s.done = 0;
    }
    log::info!("build crawl: {} ladder players", players.len());
    for puuid in players {
        while PAUSED.load(Ordering::Relaxed) {
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
        let now = crate::players::now_ms();
        let fresh = st.crawled_at(&puuid).is_some_and(|t| now - t < RECRAWL_HOURS * 3_600_000);
        if !fresh {
            let lcu = lcu::require()?;
            match lcu.get_value(&format!("/lol-match-history/v1/products/lol/{puuid}/matches?begIndex=0&endIndex=39")).await {
                Ok(v) => {
                    let rows: Vec<(GameRow, String)> = model::rows_from_history(&v)
                        .into_iter()
                        .filter(|r| r.queue_id == 420 && !r.remake && r.created >= since() && r.champion_id > 0)
                        .map(|r| {
                            let role = crate::positions::role_of(&r).group;
                            (r, role)
                        })
                        .collect();
                    st.save_build_rows(&puuid, &rows, now)?;
                }
                Err(e) => log::warn!("build crawl {puuid}: {e:#}"),
            }
            tokio::time::sleep(Duration::from_millis(1200)).await;
        }
        STATUS.lock().done += 1;
    }
    log::info!("build crawl finished");
    Ok(())
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Pick<T> {
    pub value: T,
    pub games: u32,
    pub wins: u32,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RunePage {
    pub primary: i32,
    pub sub: i32,
    pub perks: Vec<i32>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Build {
    pub champion_id: i32,
    pub role: String,
    pub games: u32,
    pub wins: u32,
    pub roles: Vec<Pick<String>>,
    pub runes: Vec<Pick<RunePage>>,
    pub spells: Vec<Pick<[i32; 2]>>,
    pub boots: Vec<Pick<i32>>,
    pub items: Vec<Pick<i32>>,
}

fn top<K: Clone + Eq + std::hash::Hash + Ord>(counts: HashMap<K, (u32, u32)>, n: usize) -> Vec<Pick<K>> {
    let mut v: Vec<Pick<K>> = counts.into_iter().map(|(value, (games, wins))| Pick { value, games, wins }).collect();
    v.sort_by(|a, b| b.games.cmp(&a.games).then(a.value.cmp(&b.value)));
    v.truncate(n);
    v
}

fn bump<K: Eq + std::hash::Hash>(m: &mut HashMap<K, (u32, u32)>, k: K, win: bool) {
    let e = m.entry(k).or_default();
    e.0 += 1;
    e.1 += win as u32;
}

/// Most common choices. `role` "" means the champion's most played role.
pub fn aggregate(champion_id: i32, role: &str, rows: &[BuildRow], items: &std::collections::BTreeMap<i32, crate::dd::Item>) -> Option<Build> {
    let mut roles = HashMap::new();
    for r in rows.iter().filter(|r| !r.role.is_empty()) {
        bump(&mut roles, r.role.clone(), r.win);
    }
    let roles = top(roles, 5);
    // A requested single role (e.g. "top" from champ select) selects the group containing it.
    let role = if role.is_empty() {
        roles.first().map(|r| r.value.clone()).unwrap_or_default()
    } else {
        roles.iter().find(|r| crate::positions::group_has(&r.value, role)).map(|r| r.value.clone()).unwrap_or_else(|| role.to_string())
    };
    let in_role: Vec<&BuildRow> = rows.iter().filter(|r| r.role == role || crate::positions::group_has(&r.role, &role)).collect();
    // Too few games in that role: use every game on the champion.
    let sample: Vec<&BuildRow> = if in_role.len() >= 8 { in_role } else { rows.iter().collect() };
    if sample.is_empty() {
        return None;
    }
    let (mut runes, mut spells, mut boots, mut legendaries) = (HashMap::new(), HashMap::new(), HashMap::new(), HashMap::new());
    for r in &sample {
        if r.perks.len() == 6 && r.perks.iter().all(|p| *p > 0) {
            bump(&mut runes, RunePageKey(r.perk_primary, r.perk_sub, r.perks.clone()), r.win);
        }
        let mut sp = r.spells;
        sp.sort();
        bump(&mut spells, sp, r.win);
        let mut seen = Vec::new();
        for it in &r.items {
            if seen.contains(it) || *it <= 0 {
                continue;
            }
            seen.push(*it);
            match items.get(it) {
                Some(i) if i.boots => bump(&mut boots, *it, r.win),
                Some(i) if i.legendary => bump(&mut legendaries, *it, r.win),
                _ => {}
            }
        }
    }
    Some(Build {
        champion_id,
        role,
        games: sample.len() as u32,
        wins: sample.iter().filter(|r| r.win).count() as u32,
        roles,
        runes: top(runes, 2)
            .into_iter()
            .map(|p| Pick { value: RunePage { primary: p.value.0, sub: p.value.1, perks: p.value.2 }, games: p.games, wins: p.wins })
            .collect(),
        spells: top(spells, 2),
        boots: top(boots, 3),
        items: top(legendaries, 9),
    })
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct RunePageKey(i32, i32, Vec<i32>);

pub async fn build(champion_id: i32, role: &str) -> Result<Option<Build>> {
    let rows = store::get().build_rows(champion_id, since())?;
    let dd = crate::dd::get("en").await?;
    Ok(aggregate(champion_id, role, &rows, &dd.items))
}

/// Stat shards: offense, flex, defense. The client's history does not record them, so they
/// follow the champion's class.
fn shards(champ_tags: &[String]) -> [i32; 3] {
    let offense = if champ_tags.iter().any(|t| t == "Marksman") { 5005 } else { 5008 };
    [offense, 5008, 5011]
}

fn starters(role: &str, champ_tags: &[String]) -> Vec<i32> {
    match role {
        "jungle" => vec![1101, 1102, 1103],
        "support" => vec![3865],
        _ if champ_tags.first().is_some_and(|t| t == "Tank") => vec![1054],
        _ if champ_tags.first().is_some_and(|t| t == "Mage" || t == "Support") => vec![1056],
        _ => vec![1055],
    }
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub rune_page: Option<String>,
    pub item_set: Option<String>,
}

const PAGE_PREFIX: &str = "MF ";

/// Write the build's rune page and/or item set into the client.
pub async fn import(champion_id: i32, role: &str, runes: bool, items: bool) -> Result<ImportResult> {
    let lcu = lcu::require()?;
    let b = build(champion_id, role).await?.ok_or_else(|| anyhow!("[no-build] No high-elo games on this champion yet"))?;
    let dd = crate::dd::get("en").await?;
    let champ = dd.champions.get(&champion_id).ok_or_else(|| anyhow!("Unknown champion {champion_id}"))?;
    let role_name = b
        .role
        .split('+')
        .map(|r| match r {
            "top" => "Top",
            "jungle" => "Jungle",
            "mid" => "Mid",
            "adc" => "Bot",
            "support" => "Support",
            _ => "",
        })
        .collect::<Vec<_>>()
        .join("/");
    let mut out = ImportResult::default();

    if runes {
        let page = b.runes.first().ok_or_else(|| anyhow!("[no-runes] No rune data for this champion yet"))?;
        let name = format!("{PAGE_PREFIX}{} {role_name}", champ.name).trim().to_string();
        let mut perks = page.value.perks.clone();
        perks.extend(shards(&champ.tags));
        let pages = lcu.get_value("/lol-perks/v1/pages").await?;
        let pages = pages.as_array().cloned().unwrap_or_default();
        let id_of = |p: &Value| p.get("id").and_then(Value::as_i64).unwrap_or(0);
        let deletable = |p: &Value| p.get("isDeletable").and_then(Value::as_bool).unwrap_or(false);
        // Reuse our own page; otherwise take a free slot; otherwise replace the current page.
        let ours = pages.iter().find(|p| deletable(p) && p.get("name").and_then(Value::as_str).is_some_and(|n| n.starts_with(PAGE_PREFIX)));
        if let Some(p) = ours {
            lcu.delete(&format!("/lol-perks/v1/pages/{}", id_of(p))).await?;
        } else {
            let inv = lcu.get_value("/lol-perks/v1/inventory").await?;
            if !inv.get("canAddCustomPage").and_then(Value::as_bool).unwrap_or(false) {
                let current = pages
                    .iter()
                    .find(|p| deletable(p) && p.get("current").and_then(Value::as_bool).unwrap_or(false))
                    .or_else(|| pages.iter().find(|p| deletable(p)))
                    .ok_or_else(|| anyhow!("[no-rune-page] No rune page can be replaced"))?;
                lcu.delete(&format!("/lol-perks/v1/pages/{}", id_of(current))).await?;
            }
        }
        lcu.post(
            "/lol-perks/v1/pages",
            &json!({ "name": name, "primaryStyleId": page.value.primary, "subStyleId": page.value.sub, "selectedPerkIds": perks, "current": true }),
        )
        .await?;
        out.rune_page = Some(name);
    }

    if items {
        let me = crate::players::me(&lcu).await?;
        let path = format!("/lol-item-sets/v1/item-sets/{}/sets", me.summoner_id);
        let mut sets = lcu.get_value(&path).await?;
        let uid = format!("mattofessor-{champion_id}");
        let title = format!("Mattofessor {} {role_name}", champ.name).trim().to_string();
        let pct = |p: &Pick<i32>| p.games as f64 * 100.0 / b.games.max(1) as f64;
        let block = |name: String, ids: Vec<i32>| {
            json!({ "type": name, "hideIfSummonerSpell": "", "showIfSummonerSpell": "",
                    "items": ids.iter().map(|i| json!({"id": i.to_string(), "count": 1})).collect::<Vec<_>>() })
        };
        let valid = |id: &i32| dd.items.contains_key(id);
        let mut blocks = vec![block("Starting items".into(), starters(b.role.split('+').next().unwrap_or(""), &champ.tags).into_iter().filter(valid).chain([2003]).collect())];
        let core: Vec<&Pick<i32>> = b.items.iter().take(3).collect();
        if let Some(first) = core.first() {
            blocks.push(block(format!("Core items ({:.0}% of games)", pct(first)), core.iter().map(|p| p.value).collect()));
        }
        if !b.boots.is_empty() {
            blocks.push(block("Boots".into(), b.boots.iter().map(|p| p.value).collect()));
        }
        if b.items.len() > 3 {
            blocks.push(block("More options".into(), b.items.iter().skip(3).map(|p| p.value).collect()));
        }
        blocks.push(block("Consumables".into(), vec![2003, 2055]));
        let new_set = json!({
            "uid": uid, "title": title, "type": "custom", "map": "any", "mode": "any", "sortrank": 0, "startedFrom": "blank",
            "associatedChampions": [champion_id], "associatedMaps": [11], "preferredItemSlots": [], "blocks": blocks,
        });
        let list = sets.get_mut("itemSets").and_then(Value::as_array_mut).ok_or_else(|| anyhow!("Item sets not available"))?;
        list.retain(|s| s.get("uid").and_then(Value::as_str) != Some(uid.as_str()));
        list.insert(0, new_set);
        sets["timestamp"] = json!(crate::players::now_ms());
        lcu.put(&path, &sets).await?;
        out.item_set = Some(title);
    }
    Ok(out)
}

/// Background loop: crawl a few minutes after start, then whenever the data is 20+ hours old.
pub fn spawn() {
    tauri::async_runtime::spawn(async {
        tokio::time::sleep(Duration::from_secs(90)).await;
        loop {
            let last = store::get().kv_get("crawl_finished").and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
            if crate::players::now_ms() - last > RECRAWL_HOURS * 3_600_000 && lcu::current().is_some() {
                if let Err(e) = crawl().await {
                    log::warn!("build crawl stopped: {e:#}");
                }
            }
            tokio::time::sleep(Duration::from_secs(600)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dd::Item;
    use std::collections::BTreeMap;

    fn items() -> BTreeMap<i32, Item> {
        let mut m = BTreeMap::new();
        let it = |boots, legendary| Item { name: String::new(), gold: 3000, boots, legendary };
        m.insert(3071, it(false, true));
        m.insert(3053, it(false, true));
        m.insert(6333, it(false, true));
        m.insert(3047, it(true, false));
        m.insert(1036, it(false, false));
        m
    }

    fn row(role: &str, win: bool, its: Vec<i32>, keystone: i32) -> BuildRow {
        BuildRow { champion_id: 1, role: role.into(), win, items: its, perks: vec![keystone, 2, 3, 4, 5, 6], perk_primary: 8000, perk_sub: 8400, spells: [11, 4] }
    }

    #[test]
    fn aggregates_most_common() {
        let mut rows = Vec::new();
        for i in 0..10 {
            rows.push(row("top", i % 2 == 0, vec![3071, 3047, 1036, 3071], 8010));
        }
        for _ in 0..4 {
            rows.push(row("top", true, vec![3053, 6333], 8437));
        }
        rows.push(row("jungle", true, vec![6333], 8010));
        let b = aggregate(1, "", &rows, &items()).unwrap();
        assert_eq!(b.role, "top");
        assert_eq!(b.games, 14);
        assert_eq!(b.runes[0].value.perks[0], 8010);
        assert_eq!(b.runes[0].games, 10);
        assert_eq!(b.spells[0].value, [4, 11]);
        assert_eq!(b.boots[0].value, 3047);
        assert_eq!(b.items[0], Pick { value: 3071, games: 10, wins: 5 }, "duplicates count once per game");
        assert!(!b.items.iter().any(|p| p.value == 1036), "components are not build items");
    }

    #[test]
    fn grouped_roles_match_a_single_role() {
        let rows: Vec<BuildRow> = (0..10).map(|_| row("mid+top", true, vec![3071], 8010)).collect();
        let b = aggregate(1, "top", &rows, &items()).unwrap();
        assert_eq!(b.role, "mid+top");
        assert_eq!(b.games, 10);
    }

    #[test]
    fn small_role_sample_uses_all_games() {
        let rows = vec![row("support", true, vec![3071], 8010), row("mid", false, vec![3071], 8010)];
        let b = aggregate(1, "support", &rows, &items()).unwrap();
        assert_eq!(b.games, 2);
        assert!(aggregate(1, "", &[], &items()).is_none());
    }
}
