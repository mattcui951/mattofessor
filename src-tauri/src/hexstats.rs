//! The Hextech ARAM pages' numbers, from AramHex (see `aramhex`) for any patch it has: the
//! champion board, the augment board, the champion × augment ranking, and one champion's page
//! (augments, finished items, boots, summoner spells, three-item builds, its win rate by skill
//! bracket and by patch).
//!
//! Item numbers come from games of at least 5 minutes; finished items and three-item builds
//! leave out players whose augments hand out or upgrade items (AramHex's notes). Only all
//! players and the top 5% have item data; other brackets show all players' items.

use crate::aramhex::{self, Info, Source, Stat};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

type Heroes = HashMap<i32, Stat>;
type Pairs = HashMap<i32, HashMap<i32, Stat>>;
type Spells = HashMap<i32, HashMap<String, Stat>>;
type Cores = HashMap<i32, Vec<RawCore>>;

#[derive(Debug, Deserialize)]
pub struct RawCore {
    pub games: u64,
    pub wins: u64,
    pub items: Vec<String>,
    #[serde(default)]
    pub pick: f64,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct ItemMeta {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub types: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchInfo {
    pub patch: String,
    pub snapshot_at: String,
    pub matches: u64,
    /// Has the skill brackets.
    pub bands: bool,
    /// Has item data (16.22 on); `items_high`: for the top 5% too.
    pub items: bool,
    pub items_high: bool,
}

fn patch_info(i: &Info) -> PatchInfo {
    PatchInfo {
        patch: i.patch.clone(),
        snapshot_at: i.snapshot_at.clone(),
        matches: i.matches,
        bands: i.bands,
        items: i.files.contains_key("hero_items.json"),
        items_high: i.files.contains_key("bands/high/hero_items.json"),
    }
}

fn source(i: &Info, bracket: &str) -> Source {
    Source { patch: i.patch.clone(), snapshot_at: i.snapshot_at.clone(), matches: i.matches, bracket: bracket.to_string() }
}

/// A bracket's copy of a file, parsed, with the bracket it is for ("all" when the bracket has none).
async fn read<T: serde::de::DeserializeOwned + Send + Sync + 'static>(info: &Info, bracket: &str, file: &str) -> Result<Option<(String, Arc<T>)>> {
    match info.locate(bracket, file) {
        Some((b, path)) => Ok(Some((b, aramhex::table::<T>(info, &path).await?))),
        None => Ok(None),
    }
}

fn bracket_name(b: &str) -> &str {
    if aramhex::BRACKETS.contains(&b) {
        b
    } else {
        "all"
    }
}

/// The set for `patch` and the one before it.
async fn sets(patch: &str) -> Result<(Vec<Arc<Info>>, Arc<Info>, Option<Arc<Info>>)> {
    let all = aramhex::datasets().await?;
    let info = aramhex::dataset(patch).await?;
    let prev = all.iter().position(|i| i.patch == info.patch).and_then(|n| all.get(n + 1)).cloned();
    Ok((all, info, prev))
}

fn rate(wins: u64, games: u64) -> f64 {
    if games == 0 {
        0.0
    } else {
        wins as f64 / games as f64
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Core {
    /// Three finished items in the order they were made.
    pub items: Vec<i32>,
    pub games: u64,
    pub win_rate: f64,
    /// Share of the champion's players (games of 5+ minutes) who made these three in this order.
    pub pick_rate: f64,
}

fn cores(raw: Option<&Vec<RawCore>>) -> Vec<Core> {
    let mut v: Vec<Core> = raw
        .map(|r| {
            r.iter()
                .map(|c| Core { items: c.items.iter().filter_map(|s| s.parse().ok()).collect(), games: c.games, win_rate: rate(c.wins, c.games), pick_rate: c.pick })
                .collect()
        })
        .unwrap_or_default();
    v.sort_by(|a, b| b.pick_rate.total_cmp(&a.pick_rate));
    v
}

// ---------------------------------------------------------------------------------------------
// Champion board

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardChamp {
    pub champion_id: i32,
    pub games: u64,
    pub win_rate: f64,
    pub pick_rate: f64,
    pub rank: u32,
    /// Its win rate the patch before (same bracket), when that patch has it.
    pub previous: Option<f64>,
    /// Its three-item build with the best win rate.
    pub best: Option<Core>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Board {
    pub source: Source,
    pub patches: Vec<PatchInfo>,
    /// The patch `previous` is from.
    pub previous_patch: Option<String>,
    /// The bracket the builds are for ("all" when the bracket has no item data), or None
    /// when the patch has no builds.
    pub builds_bracket: Option<String>,
    pub champions: Vec<BoardChamp>,
}

/// The best build of each champion: the highest win rate among its (up to three) most made ones.
fn best_core(raw: Option<&Vec<RawCore>>) -> Option<Core> {
    cores(raw).into_iter().max_by(|a, b| a.win_rate.total_cmp(&b.win_rate))
}

pub fn board_from(
    info: &Info,
    bracket: &str,
    heroes: &Heroes,
    prev: Option<(&str, &Heroes)>,
    builds: Option<(&str, &Cores)>,
    patches: Vec<PatchInfo>,
) -> Board {
    let mut champions: Vec<BoardChamp> = heroes
        .iter()
        .map(|(id, s)| BoardChamp {
            champion_id: *id,
            games: s.games,
            win_rate: s.win_rate(),
            pick_rate: s.pick_rate,
            rank: s.rank,
            previous: prev.and_then(|(_, p)| p.get(id)).filter(|p| p.games > 0).map(Stat::win_rate),
            best: builds.and_then(|(_, c)| best_core(c.get(id))),
        })
        .collect();
    champions.sort_by(|a, b| b.win_rate.total_cmp(&a.win_rate));
    Board {
        source: source(info, bracket),
        patches,
        previous_patch: prev.map(|(p, _)| p.to_string()),
        builds_bracket: builds.map(|(b, _)| b.to_string()),
        champions,
    }
}

pub async fn board(patch: &str, bracket: &str) -> Result<Board> {
    aramhex::fresh(|| board_once(patch, bracket)).await
}

async fn board_once(patch: &str, bracket: &str) -> Result<Board> {
    let (all, info, prev) = sets(patch).await?;
    let want = bracket_name(bracket);
    let (used, heroes) = read::<Heroes>(&info, want, "heroes.json").await?.ok_or_else(|| anyhow::anyhow!("AramHex {} has no champions", info.patch))?;
    // The patch before, in the same bracket only (all players' numbers would not compare).
    let mut before = None;
    if let Some(p) = &prev {
        if let Some((b, h)) = read::<Heroes>(p, &used, "heroes.json").await? {
            if b == used {
                before = Some((p.patch.clone(), h));
            }
        }
    }
    let builds = read::<Cores>(&info, &used, "core_builds.json").await?;
    Ok(board_from(
        &info,
        &used,
        &heroes,
        before.as_ref().map(|(p, h)| (p.as_str(), h.as_ref())),
        builds.as_ref().map(|(b, c)| (b.as_str(), c.as_ref())),
        all.iter().map(|i| patch_info(i)).collect(),
    ))
}

// ---------------------------------------------------------------------------------------------
// Augment board

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AugStat {
    pub id: i32,
    pub games: u64,
    pub win_rate: f64,
    /// Win rate minus the champion's own (overall: each champion's, weighted by games).
    pub delta: f64,
    /// Share of players (overall: of all champions' players) who had it.
    pub pick_rate: f64,
    pub low_sample: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AugBoard {
    pub source: Source,
    pub patches: Vec<PatchInfo>,
    pub augments: Vec<AugStat>,
    /// Augments the patch before had and this one has no games of (removed from the mode),
    /// with their numbers then.
    pub removed: Vec<AugStat>,
    pub previous_patch: Option<String>,
}

/// Every augment over all champions: its win rate, and how much better its players did than
/// their champion usually does.
pub fn augment_totals(heroes: &Heroes, pairs: &Pairs) -> Vec<AugStat> {
    let players: u64 = heroes.values().map(|s| s.games).sum();
    let mut totals: HashMap<i32, (u64, u64, f64)> = HashMap::new();
    for (hero, augs) in pairs {
        let base = heroes.get(hero).map(Stat::win_rate).unwrap_or(0.5);
        for (id, s) in augs {
            let t = totals.entry(*id).or_default();
            t.0 += s.games;
            t.1 += s.wins;
            t.2 += s.wins as f64 - s.games as f64 * base;
        }
    }
    let mut v: Vec<AugStat> = totals
        .into_iter()
        .filter(|(_, t)| t.0 > 0)
        .map(|(id, (games, wins, lift))| AugStat {
            id,
            games,
            win_rate: rate(wins, games),
            delta: lift / games as f64,
            pick_rate: rate(games, players),
            low_sample: games < 3000,
        })
        .collect();
    v.sort_by(|a, b| b.delta.total_cmp(&a.delta));
    v
}

pub async fn augment_board(patch: &str, bracket: &str) -> Result<AugBoard> {
    aramhex::fresh(|| augment_board_once(patch, bracket)).await
}

async fn augment_board_once(patch: &str, bracket: &str) -> Result<AugBoard> {
    let (all, info, prev) = sets(patch).await?;
    let want = bracket_name(bracket);
    let (used, heroes) = read::<Heroes>(&info, want, "heroes.json").await?.ok_or_else(|| anyhow::anyhow!("no champions"))?;
    let (_, pairs) = read::<Pairs>(&info, &used, "hero_augments.json").await?.ok_or_else(|| anyhow::anyhow!("no augments"))?;
    let augments = augment_totals(&heroes, &pairs);
    // Removed: judged on all players (a bracket can miss a rare augment by chance).
    let mut removed = Vec::new();
    if let Some(p) = &prev {
        let now = if used == "all" { augments.clone() } else { all_players_augments(&info).await? };
        let (Some((_, ph)), Some((_, pp))) = (read::<Heroes>(p, "all", "heroes.json").await?, read::<Pairs>(p, "all", "hero_augments.json").await?) else {
            return Ok(AugBoard { source: source(&info, &used), patches: all.iter().map(|i| patch_info(i)).collect(), augments, removed, previous_patch: None });
        };
        removed = augment_totals(&ph, &pp).into_iter().filter(|a| a.games >= 3000 && !now.iter().any(|n| n.id == a.id)).collect();
    }
    Ok(AugBoard {
        source: source(&info, &used),
        patches: all.iter().map(|i| patch_info(i)).collect(),
        augments,
        removed,
        previous_patch: prev.map(|p| p.patch.clone()),
    })
}

async fn all_players_augments(info: &Info) -> Result<Vec<AugStat>> {
    let (Some((_, h)), Some((_, p))) = (read::<Heroes>(info, "all", "heroes.json").await?, read::<Pairs>(info, "all", "hero_augments.json").await?) else {
        return Ok(Vec::new());
    };
    Ok(augment_totals(&h, &p))
}

// ---------------------------------------------------------------------------------------------
// Champion × augment ranking

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Combo {
    pub champion_id: i32,
    pub augment_id: i32,
    pub games: u64,
    pub win_rate: f64,
    /// Win rate minus the champion's own.
    pub delta: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Combos {
    pub source: Source,
    pub patches: Vec<PatchInfo>,
    /// How many pairs match (`rows` holds the first `LIMIT` of them).
    pub total: usize,
    pub rows: Vec<Combo>,
}

const COMBO_LIMIT: usize = 200;

/// Pairs with at least `min_games` games (and not marked a small sample), for the champions
/// and augments asked for (empty: all), sorted by `sort` ("win", "delta" or "games").
pub fn combo_rows(heroes: &Heroes, pairs: &Pairs, min_games: u64, sort: &str, champions: &[i32], augments: &[i32]) -> (usize, Vec<Combo>) {
    let mut v: Vec<Combo> = Vec::new();
    for (hero, augs) in pairs {
        if !champions.is_empty() && !champions.contains(hero) {
            continue;
        }
        let base = heroes.get(hero).map(Stat::win_rate).unwrap_or(0.5);
        for (id, s) in augs {
            if s.low_sample || s.games < min_games.max(1) || (!augments.is_empty() && !augments.contains(id)) {
                continue;
            }
            v.push(Combo { champion_id: *hero, augment_id: *id, games: s.games, win_rate: s.win_rate(), delta: s.win_rate() - base });
        }
    }
    let key = |c: &Combo| match sort {
        "delta" => c.delta,
        "games" => c.games as f64,
        _ => c.win_rate,
    };
    v.sort_by(|a, b| key(b).total_cmp(&key(a)).then(b.games.cmp(&a.games)));
    let total = v.len();
    v.truncate(COMBO_LIMIT);
    (total, v)
}

pub async fn combos(patch: &str, bracket: &str, min_games: u64, sort: &str, champions: &[i32], augments: &[i32]) -> Result<Combos> {
    aramhex::fresh(|| combos_once(patch, bracket, min_games, sort, champions, augments)).await
}

async fn combos_once(patch: &str, bracket: &str, min_games: u64, sort: &str, champions: &[i32], augments: &[i32]) -> Result<Combos> {
    let (all, info, _) = sets(patch).await?;
    let (used, heroes) = read::<Heroes>(&info, bracket_name(bracket), "heroes.json").await?.ok_or_else(|| anyhow::anyhow!("no champions"))?;
    let (_, pairs) = read::<Pairs>(&info, &used, "hero_augments.json").await?.ok_or_else(|| anyhow::anyhow!("no augments"))?;
    let (total, rows) = combo_rows(&heroes, &pairs, min_games, sort, champions, augments);
    Ok(Combos { source: source(&info, &used), patches: all.iter().map(|i| patch_info(i)).collect(), total, rows })
}

// ---------------------------------------------------------------------------------------------
// One champion

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ItemStat {
    /// 0: no boots (boots only).
    pub id: i32,
    /// AramHex's (Chinese) name and icon, for an item Data Dragon does not have.
    pub name: String,
    pub icon: String,
    pub games: u64,
    pub win_rate: f64,
    /// Win rate minus the champion's (same games).
    pub delta: f64,
    /// Share of the champion's players who had it.
    pub pick_rate: f64,
    /// The champion's win rate in the games without it.
    pub without: f64,
    pub low_sample: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpellStat {
    pub ids: [i32; 2],
    pub games: u64,
    pub win_rate: f64,
    pub delta: f64,
    pub pick_rate: f64,
    pub low_sample: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BandRow {
    pub bracket: String,
    pub games: u64,
    pub win_rate: f64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PatchRow {
    pub patch: String,
    pub games: u64,
    pub win_rate: f64,
}

/// How often the champion's most taken augments are taken in each bracket.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BandPicks {
    pub bracket: String,
    /// (augment id, share of the bracket's players of the champion who had it).
    pub picks: Vec<(i32, f64)>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChampionPage {
    pub source: Source,
    pub patches: Vec<PatchInfo>,
    pub champion: Option<aramhex::ChampRow>,
    /// How many champions the patch has (for "rank 3 of 172").
    pub champions: usize,
    /// All players and each skill bracket (when the patch has them).
    pub bands: Vec<BandRow>,
    /// All players, patch by patch, oldest first.
    pub history: Vec<PatchRow>,
    /// Every augment it was seen with.
    pub augments: Vec<AugStat>,
    pub band_picks: Vec<BandPicks>,
    /// The bracket the items are for ("all" when the bracket has none), None without item data.
    pub items_bracket: Option<String>,
    /// Finished items, most made first.
    pub items: Vec<ItemStat>,
    /// Boots (id 0: none), most worn first.
    pub boots: Vec<ItemStat>,
    pub spells: Vec<SpellStat>,
    /// Up to three three-item builds, most made first.
    pub cores: Vec<Core>,
}

fn item_stat(id: i32, s: &Stat, base: &Stat, meta: &HashMap<i32, ItemMeta>) -> ItemStat {
    let m = meta.get(&id).cloned().unwrap_or_default();
    let (og, ow) = (base.games.saturating_sub(s.games), base.wins.saturating_sub(s.wins));
    ItemStat {
        id,
        name: m.name,
        icon: m.icon,
        games: s.games,
        win_rate: s.win_rate(),
        delta: s.win_rate() - base.win_rate(),
        pick_rate: rate(s.games, base.games),
        without: if og > 0 { rate(ow, og) } else { base.win_rate() },
        low_sample: s.low_sample,
    }
}

pub fn items_of(champion: i32, finished: &Pairs, finished_base: &Heroes, meta: &HashMap<i32, ItemMeta>) -> Vec<ItemStat> {
    let Some(base) = finished_base.get(&champion) else { return Vec::new() };
    let mut v: Vec<ItemStat> =
        finished.get(&champion).map(|m| m.iter().filter(|(id, s)| s.games > 0 && !crate::builds::junk(**id)).map(|(id, s)| item_stat(*id, s, base, meta)).collect()).unwrap_or_default();
    v.sort_by_key(|x| std::cmp::Reverse(x.games));
    v
}

/// Boots from all items held: each kind, and no boots at all. The plain Boots count as boots
/// but are not listed (`builds::JUNK`).
pub fn boots_of(champion: i32, holdings: &Pairs, base: &Heroes, meta: &HashMap<i32, ItemMeta>) -> Vec<ItemStat> {
    let Some(base) = base.get(&champion) else { return Vec::new() };
    let mut v: Vec<ItemStat> = holdings
        .get(&champion)
        .map(|m| m.iter().filter(|(id, s)| s.games > 0 && meta.get(id).is_some_and(|m| m.types.iter().any(|t| t == "Boots"))).map(|(id, s)| item_stat(*id, s, base, meta)).collect())
        .unwrap_or_default();
    let (g, w) = v.iter().fold((0u64, 0u64), |(g, w), b| (g + b.games, w + (b.win_rate * b.games as f64).round() as u64));
    let none = Stat { games: base.games.saturating_sub(g), wins: base.wins.saturating_sub(w), low_sample: base.games.saturating_sub(g) < 3000, ..Default::default() };
    v.retain(|b| !crate::builds::junk(b.id));
    if none.games > 0 {
        v.push(ItemStat { name: String::new(), icon: String::new(), ..item_stat(0, &none, base, &HashMap::new()) });
    }
    v.sort_by_key(|x| std::cmp::Reverse(x.games));
    v
}

pub fn spells_of(champion: i32, spells: &Spells, heroes: &Heroes) -> Vec<SpellStat> {
    let Some(base) = heroes.get(&champion) else { return Vec::new() };
    let mut v: Vec<SpellStat> = spells
        .get(&champion)
        .map(|m| {
            m.iter()
                .filter_map(|(k, s)| {
                    let (a, b) = k.split_once('+')?;
                    Some(SpellStat {
                        ids: [a.parse().ok()?, b.parse().ok()?],
                        games: s.games,
                        win_rate: s.win_rate(),
                        delta: s.win_rate() - base.win_rate(),
                        pick_rate: rate(s.games, base.games),
                        low_sample: s.low_sample,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    v.sort_by_key(|x| std::cmp::Reverse(x.games));
    v
}

pub fn augments_of(champion: i32, pairs: &Pairs, heroes: &Heroes) -> Vec<AugStat> {
    let Some(base) = heroes.get(&champion) else { return Vec::new() };
    let mut v: Vec<AugStat> = pairs
        .get(&champion)
        .map(|m| {
            m.iter()
                .filter(|(_, s)| s.games > 0)
                .map(|(id, s)| AugStat { id: *id, games: s.games, win_rate: s.win_rate(), delta: s.win_rate() - base.win_rate(), pick_rate: rate(s.games, base.games), low_sample: s.low_sample })
                .collect()
        })
        .unwrap_or_default();
    v.sort_by_key(|x| std::cmp::Reverse(x.games));
    v
}

/// The augments shown by bracket: the champion's most taken ones over all players.
const BAND_AUGMENTS: usize = 5;

pub async fn champion(champion_id: i32, patch: &str, bracket: &str) -> Result<ChampionPage> {
    aramhex::fresh(|| champion_once(champion_id, patch, bracket)).await
}

async fn champion_once(champion_id: i32, patch: &str, bracket: &str) -> Result<ChampionPage> {
    let (all, info, _) = sets(patch).await?;
    let want = bracket_name(bracket);
    let (used, heroes) = read::<Heroes>(&info, want, "heroes.json").await?.ok_or_else(|| anyhow::anyhow!("no champions"))?;
    let hero = heroes.get(&champion_id).cloned();
    let pairs = read::<Pairs>(&info, &used, "hero_augments.json").await?.map(|(_, p)| p);
    let augments = pairs.as_ref().map(|p| augments_of(champion_id, p, &heroes)).unwrap_or_default();

    // Skill brackets and their augment picks.
    let mut bands = Vec::new();
    let mut band_picks = Vec::new();
    let all_heroes = read::<Heroes>(&info, "all", "heroes.json").await?.map(|(_, h)| h);
    if let Some(s) = all_heroes.as_ref().and_then(|h| h.get(&champion_id)) {
        bands.push(BandRow { bracket: "all".into(), games: s.games, win_rate: s.win_rate() });
    }
    if info.bands {
        let top: Vec<i32> = match read::<Pairs>(&info, "all", "hero_augments.json").await? {
            Some((_, p)) => {
                let h = all_heroes.clone().unwrap_or_default();
                augments_of(champion_id, &p, &h).into_iter().take(BAND_AUGMENTS).map(|a| a.id).collect()
            }
            None => Vec::new(),
        };
        for b in ["casual", "mid", "high"] {
            let (Some((hb, h)), Some((_, p))) = (read::<Heroes>(&info, b, "heroes.json").await?, read::<Pairs>(&info, b, "hero_augments.json").await?) else { continue };
            if hb != b {
                continue;
            }
            let Some(s) = h.get(&champion_id) else { continue };
            bands.push(BandRow { bracket: b.into(), games: s.games, win_rate: s.win_rate() });
            let mine = p.get(&champion_id);
            band_picks.push(BandPicks {
                bracket: b.into(),
                picks: top.iter().map(|id| (*id, mine.and_then(|m| m.get(id)).map(|a| rate(a.games, s.games)).unwrap_or(0.0))).collect(),
            });
        }
    }

    // Patch by patch, all players.
    let mut history = Vec::new();
    for set in all.iter().rev() {
        if let Ok(Some((_, h))) = read::<Heroes>(set, "all", "heroes.json").await {
            if let Some(s) = h.get(&champion_id) {
                history.push(PatchRow { patch: set.patch.clone(), games: s.games, win_rate: s.win_rate() });
            }
        }
    }

    // Items, boots and builds (all players or the top 5%).
    let meta = match aramhex::table::<HashMap<i32, ItemMeta>>(&info, "item_metadata.json").await {
        Ok(m) => m,
        Err(_) => Arc::new(HashMap::new()),
    };
    let mut items_bracket = None;
    let (mut items, mut boots, mut core_list) = (Vec::new(), Vec::new(), Vec::new());
    if let (Some((ib, finished)), Some((_, fbase))) =
        (read::<Pairs>(&info, &used, "hero_items.json").await?, read::<Heroes>(&info, &used, "equipment_recommendation_heroes.json").await?)
    {
        items = items_of(champion_id, &finished, &fbase, &meta);
        if let (Some((_, held)), Some((_, hbase))) =
            (read::<Pairs>(&info, &ib, "hero_item_holdings.json").await?, read::<Heroes>(&info, &ib, "equipment_heroes.json").await?)
        {
            boots = boots_of(champion_id, &held, &hbase, &meta);
        }
        if let Some((_, c)) = read::<Cores>(&info, &ib, "core_builds.json").await? {
            core_list = cores(c.get(&champion_id));
        }
        items_bracket = Some(ib);
    }
    let spells = match read::<Spells>(&info, &used, "hero_spells.json").await? {
        Some((_, s)) => spells_of(champion_id, &s, &heroes),
        None => Vec::new(),
    };

    Ok(ChampionPage {
        source: source(&info, &used),
        patches: all.iter().map(|i| patch_info(i)).collect(),
        champion: hero.map(|s| aramhex::ChampRow { champion_id, games: s.games, win_rate: s.win_rate(), pick_rate: s.pick_rate, rank: s.rank }),
        champions: heroes.len(),
        bands,
        history,
        augments,
        band_picks,
        items_bracket,
        items,
        boots,
        spells,
        cores: core_list,
    })
}

/// The Hextech ARAM build for importing and the Builds page's summary: the most made
/// three-item build, then the most made other finished items; the most worn boots; the most
/// taken summoner spells. All players, current patch. None when AramHex has no item data.
pub struct AramBuild {
    pub core: Vec<i32>,
    /// The (up to three) most made three-item builds, most made first: items, games, wins and the
    /// share of the champion's players who made it.
    pub routes: Vec<(Vec<i32>, u64, u64, f64)>,
    pub items: Vec<(i32, u64, u64)>,
    pub boots: Vec<(i32, u64, u64)>,
    pub spells: Vec<([i32; 2], u64, u64)>,
    pub games: u64,
    pub wins: u64,
}

pub async fn aram_build(champion_id: i32) -> Result<Option<AramBuild>> {
    let page = champion(champion_id, "", "all").await?;
    if page.items_bracket.is_none() || page.items.is_empty() {
        return Ok(None);
    }
    let wins = |s: f64, g: u64| (s * g as f64).round() as u64;
    let core = page.cores.first().map(|c| c.items.clone()).unwrap_or_else(|| page.items.iter().take(3).map(|i| i.id).collect());
    let mut items: Vec<(i32, u64, u64)> = core
        .iter()
        .map(|id| page.items.iter().find(|i| i.id == *id).map(|i| (i.id, i.games, wins(i.win_rate, i.games))).unwrap_or((*id, 0, 0)))
        .collect();
    items.extend(page.items.iter().filter(|i| !core.contains(&i.id) && !i.low_sample).take(6).map(|i| (i.id, i.games, wins(i.win_rate, i.games))));
    Ok(Some(AramBuild {
        routes: page.cores.iter().map(|c| (c.items.clone(), c.games, wins(c.win_rate, c.games), c.pick_rate)).collect(),
        core,
        items,
        boots: page.boots.iter().filter(|b| b.id != 0 && !b.low_sample).take(2).map(|b| (b.id, b.games, wins(b.win_rate, b.games))).collect(),
        spells: page.spells.iter().filter(|s| !s.low_sample).take(2).map(|s| (s.ids, s.games, wins(s.win_rate, s.games))).collect(),
        games: page.champion.as_ref().map(|c| c.games).unwrap_or(0),
        wins: page.champion.as_ref().map(|c| wins(c.win_rate, c.games)).unwrap_or(0),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn heroes() -> Heroes {
        serde_json::from_str(r#"{"24": {"games": 1000, "wins": 600, "pick_rate": 0.01, "rank": 1}, "1": {"games": 1000, "wins": 400, "rank": 2}}"#).unwrap()
    }

    #[test]
    fn augment_lift_is_against_each_champion() {
        let pairs: Pairs = serde_json::from_str(r#"{"24": {"7": {"games": 100, "wins": 70}}, "1": {"7": {"games": 100, "wins": 50}, "8": {"games": 10, "wins": 9}}}"#).unwrap();
        let a = augment_totals(&heroes(), &pairs);
        let seven = a.iter().find(|x| x.id == 7).unwrap();
        // 60% raw, but +10 points over Jax (70 vs 60) and over Annie (50 vs 40).
        assert!((seven.win_rate - 0.6).abs() < 1e-9 && (seven.delta - 0.1).abs() < 1e-9);
        assert!((seven.pick_rate - 0.1).abs() < 1e-9);
        assert!(a.iter().find(|x| x.id == 8).unwrap().low_sample);
    }

    #[test]
    fn items_have_the_rate_without_them() {
        let finished: Pairs = serde_json::from_str(r#"{"24": {"3031": {"games": 400, "wins": 300}, "3089": {"games": 100, "wins": 40}, "2031": {"games": 900, "wins": 500}}}"#).unwrap();
        let meta: HashMap<i32, ItemMeta> = serde_json::from_str(r#"{"3031": {"name": "无尽之刃", "icon": "x", "types": ["CriticalStrike"]}}"#).unwrap();
        let v = items_of(24, &finished, &heroes(), &meta);
        assert_eq!(v.iter().map(|i| i.id).collect::<Vec<_>>(), [3031, 3089], "most made first, no potion");
        let ie = &v[0];
        assert!((ie.win_rate - 0.75).abs() < 1e-9 && (ie.delta - 0.15).abs() < 1e-9 && (ie.pick_rate - 0.4).abs() < 1e-9);
        // 600 wins of 1000; without it 300 of 600.
        assert!((ie.without - 0.5).abs() < 1e-9);
        assert_eq!(ie.name, "无尽之刃");
    }

    #[test]
    fn boots_include_none() {
        let held: Pairs = serde_json::from_str(r#"{"24": {"3006": {"games": 600, "wins": 380}, "1001": {"games": 100, "wins": 70}, "3031": {"games": 400, "wins": 300}, "3047": {"games": 200, "wins": 100}}}"#).unwrap();
        let meta: HashMap<i32, ItemMeta> = serde_json::from_str(r#"{"3006": {"types": ["Boots"]}, "1001": {"types": ["Boots"]}, "3047": {"types": ["Boots"]}, "3031": {"types": []}}"#).unwrap();
        let v = boots_of(24, &held, &heroes(), &meta);
        assert_eq!(v.iter().map(|i| i.id).collect::<Vec<_>>(), [3006, 3047, 0], "plain Boots not listed");
        let none = &v[2];
        assert_eq!(none.games, 100, "plain Boots still count as boots");
        assert!((none.win_rate - 0.5).abs() < 1e-9, "1000 - 900 games, 600 - 550 wins");
    }

    #[test]
    fn spells_and_combos() {
        let spells: Spells = serde_json::from_str(r#"{"24": {"4+32": {"games": 900, "wins": 560}, "4+14": {"games": 100, "wins": 40, "low_sample": true}}}"#).unwrap();
        let s = spells_of(24, &spells, &heroes());
        assert_eq!(s[0].ids, [4, 32]);
        assert!((s[0].pick_rate - 0.9).abs() < 1e-9);
        let pairs: Pairs = serde_json::from_str(r#"{"24": {"7": {"games": 5000, "wins": 3500}, "8": {"games": 9000, "wins": 5400}}, "1": {"7": {"games": 4000, "wins": 2200}, "9": {"games": 100, "wins": 90}}}"#).unwrap();
        let (n, rows) = combo_rows(&heroes(), &pairs, 3000, "win", &[], &[]);
        assert_eq!(n, 3, "the 100-game pair is left out");
        assert_eq!((rows[0].champion_id, rows[0].augment_id), (24, 7));
        let (_, by_delta) = combo_rows(&heroes(), &pairs, 3000, "delta", &[], &[]);
        assert_eq!((by_delta[0].champion_id, by_delta[0].augment_id), (1, 7), "55% on a 40% champion");
        let (n, only) = combo_rows(&heroes(), &pairs, 0, "games", &[], &[7]);
        assert_eq!((n, only[0].champion_id), (2, 24));
    }

    #[test]
    fn board_marks_the_patch_before_and_best_build() {
        let info = aramhex::parse_info(&serde_json::json!({"seq": 3, "patch": "16.22", "path": "p", "files": {"heroes.json": {"sha256": "x"}}})).unwrap();
        let before: Heroes = serde_json::from_str(r#"{"24": {"games": 10, "wins": 5}}"#).unwrap();
        let cores: Cores = serde_json::from_str(r#"{"24": [{"games": 50, "wins": 30, "items": ["1", "2", "3"], "pick": 0.3}, {"games": 20, "wins": 16, "items": ["1", "4", "3"], "pick": 0.1}]}"#).unwrap();
        let b = board_from(&info, "all", &heroes(), Some(("16.21", &before)), Some(("all", &cores)), Vec::new());
        let jax = &b.champions[0];
        assert_eq!((jax.champion_id, jax.previous), (24, Some(0.5)));
        assert_eq!(jax.best.as_ref().map(|c| c.items.clone()), Some(vec![1, 4, 3]), "80% beats 60%");
        assert_eq!(b.champions[1].previous, None);
    }

    #[test]
    fn index_keeps_regular_patches() {
        let v = serde_json::json!({"datasets": [
            {"patch": "16.21", "path": "patches/16.21/normal", "mode": "normal", "seq": 12, "files": {"heroes.json": {"sha256": "a"}}},
            {"patch": "16.21", "path": "patches/16.21-prehotfix/normal", "mode": "normal", "files": {"heroes.json": {"sha256": "b"}}},
            {"patch": "16.20", "path": "patches/16.21-pbe/normal", "mode": "normal", "provisional": true, "files": {"heroes.json": {"sha256": "c"}}},
            {"patch": "16.9", "path": "patches/16.9/normal", "mode": "normal", "files": {"heroes.json": {"sha256": "d"}}},
            {"patch": "16.19", "path": "patches/16.19/first-prismatic", "mode": "first-prismatic", "files": {"heroes.json": {"sha256": "e"}}}
        ]});
        let i = aramhex::parse_index(&v);
        assert_eq!(i.iter().map(|x| x.patch.as_str()).collect::<Vec<_>>(), ["16.21", "16.9"]);
        assert_eq!(i[0].dir_key(), "h-16.21");
        assert_eq!(i[0].locate("high", "heroes.json"), Some(("all".into(), "heroes.json".into())));
    }
}
