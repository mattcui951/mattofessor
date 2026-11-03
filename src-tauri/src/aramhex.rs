//! Hextech ARAM statistics from AramHex's open data: 海克斯大乱斗统计数据
//! (github.com/czgreat/aramhex-data), CC BY-NC 4.0. Champion win rates and the win rate of
//! every champion × augment pair, for all players and by skill bracket, from millions of games
//! on the CN servers, updated about weekly.
//!
//! `latest.json` names the current data set (`seq`, `path`) and every file's SHA-256. Files are
//! downloaded on first use of a bracket, checked, and kept under %APPDATA%/Mattofessor/aramhex/<seq>;
//! the previous set stays in use until a new one is complete. Checked for updates every 6 hours.

use anyhow::{anyhow, bail, Context as _, Result};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

const BASE: &str = "https://raw.githubusercontent.com/czgreat/aramhex-data/main/";
const CHECK_EVERY: Duration = Duration::from_secs(6 * 3600);
/// Smoothing weight for a champion × augment win rate, as AramHex's notes suggest.
const SMOOTH_GAMES: f64 = 300.0;
pub const BRACKETS: [&str; 4] = ["all", "casual", "mid", "high"];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    pub seq: u64,
    pub patch: String,
    pub path: String,
    pub snapshot_at: String,
    pub matches: u64,
    pub bands: bool,
    /// Relative path -> SHA-256.
    pub files: HashMap<String, String>,
}

/// One row of AramHex's files (their field names are snake_case).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Stat {
    pub games: u64,
    pub wins: u64,
    #[serde(default)]
    pub pick_rate: f64,
    #[serde(default)]
    pub rank: u32,
    #[serde(default)]
    pub low_sample: bool,
}

impl Stat {
    pub fn win_rate(&self) -> f64 {
        if self.games == 0 {
            0.0
        } else {
            self.wins as f64 / self.games as f64
        }
    }
}

#[derive(Debug, Default)]
pub struct Bracket {
    pub heroes: HashMap<i32, Stat>,
    pub hero_augments: HashMap<i32, HashMap<i32, Stat>>,
}

pub fn parse_info(v: &Value) -> Result<Info> {
    let s = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or_default().to_string();
    let files = v
        .get("files")
        .and_then(Value::as_object)
        .map(|m| m.iter().filter_map(|(k, f)| Some((k.clone(), f.get("sha256")?.as_str()?.to_string()))).collect())
        .unwrap_or_default();
    let info = Info {
        seq: v.get("seq").and_then(Value::as_u64).ok_or_else(|| anyhow!("no seq"))?,
        patch: s("patch"),
        path: s("path"),
        snapshot_at: s("snapshot_at"),
        matches: v.get("matches").and_then(Value::as_u64).unwrap_or(0),
        bands: v.get("bands").and_then(Value::as_bool).unwrap_or(false),
        files,
    };
    if info.path.is_empty() || info.files.is_empty() {
        bail!("AramHex index is incomplete");
    }
    Ok(info)
}

fn ids<T>(m: HashMap<String, T>) -> HashMap<i32, T> {
    m.into_iter().filter_map(|(k, v)| Some((k.parse().ok()?, v))).collect()
}

pub fn parse_bracket(heroes: &[u8], hero_augments: &[u8]) -> Result<Bracket> {
    let heroes: HashMap<String, Stat> = serde_json::from_slice(heroes).context("heroes.json")?;
    let pairs: HashMap<String, HashMap<String, Stat>> = serde_json::from_slice(hero_augments).context("hero_augments.json")?;
    Ok(Bracket { heroes: ids(heroes), hero_augments: ids(pairs).into_iter().map(|(h, m)| (h, ids(m))).collect() })
}

fn dir() -> PathBuf {
    crate::paths::config_dir().join("aramhex")
}

fn rel(bracket: &str, file: &str) -> String {
    if bracket == "all" {
        file.to_string()
    } else {
        format!("bands/{bracket}/{file}")
    }
}

fn client() -> reqwest::Client {
    reqwest::Client::builder().timeout(Duration::from_secs(60)).build().expect("http")
}

async fn fetch(url: &str) -> Result<Vec<u8>> {
    let r = client().get(url).send().await.with_context(|| format!("AramHex data not reachable ({url})"))?;
    if !r.status().is_success() {
        bail!("AramHex data answered {}", r.status());
    }
    Ok(r.bytes().await?.to_vec())
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

struct State {
    info: Option<Arc<Info>>,
    brackets: BTreeMap<String, Arc<Bracket>>,
}

static STATE: Mutex<State> = Mutex::new(State { info: None, brackets: BTreeMap::new() });
static FETCH: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn saved_info() -> Option<Info> {
    let t = std::fs::read_to_string(dir().join("current.json")).ok()?;
    serde_json::from_str(&t).ok()
}

/// The data set in use: the one on disk, or the latest when there is none yet.
async fn info() -> Result<Arc<Info>> {
    if let Some(i) = STATE.lock().info.clone() {
        return Ok(i);
    }
    let info = match saved_info() {
        Some(i) => i,
        None => latest().await?,
    };
    let info = Arc::new(info);
    STATE.lock().info = Some(info.clone());
    Ok(info)
}

async fn latest() -> Result<Info> {
    let bytes = fetch(&format!("{BASE}latest.json")).await?;
    parse_info(&serde_json::from_slice(&bytes)?)
}

/// One file of a data set, from disk or downloaded and checked.
async fn file(info: &Info, rel_path: &str) -> Result<Vec<u8>> {
    let local = dir().join(info.seq.to_string()).join(rel_path);
    let want = info.files.get(rel_path).ok_or_else(|| anyhow!("AramHex has no {rel_path}"))?;
    if let Ok(b) = std::fs::read(&local) {
        if &sha256(&b) == want {
            return Ok(b);
        }
    }
    let b = fetch(&format!("{BASE}{}/{rel_path}", info.path)).await?;
    if &sha256(&b) != want {
        bail!("AramHex {rel_path} failed its checksum");
    }
    if let Some(parent) = local.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&local, &b)?;
    Ok(b)
}

async fn load(info: &Info, bracket: &str) -> Result<Bracket> {
    let b = if bracket == "all" || info.bands { bracket } else { "all" };
    let heroes = file(info, &rel(b, "heroes.json")).await?;
    let pairs = file(info, &rel(b, "hero_augments.json")).await?;
    parse_bracket(&heroes, &pairs)
}

/// Statistics for a skill bracket ("all", "casual", "mid", "high"), with the data set they are from.
pub async fn bracket(name: &str) -> Result<(Arc<Info>, Arc<Bracket>)> {
    let name = if BRACKETS.contains(&name) { name } else { "all" };
    let info = info().await?;
    if let Some(b) = STATE.lock().brackets.get(name).cloned() {
        return Ok((info, b));
    }
    let _one = FETCH.lock().await;
    if let Some(b) = STATE.lock().brackets.get(name).cloned() {
        return Ok((info, b));
    }
    let b = Arc::new(load(&info, name).await?);
    if saved_info().map(|i| i.seq) != Some(info.seq) {
        let _ = std::fs::create_dir_all(dir());
        let _ = std::fs::write(dir().join("current.json"), serde_json::to_string(&*info)?);
    }
    STATE.lock().brackets.insert(name.to_string(), b.clone());
    Ok((info, b))
}

/// Switch to a newer data set once the brackets in use are downloaded; drop older sets.
async fn update() -> Result<()> {
    let newest = latest().await?;
    let current = STATE.lock().info.clone();
    if current.as_ref().is_some_and(|c| c.seq >= newest.seq) {
        return Ok(());
    }
    let in_use: Vec<String> = {
        let s = STATE.lock();
        let mut v: Vec<String> = s.brackets.keys().cloned().collect();
        if v.is_empty() {
            v.push("all".into());
        }
        v
    };
    let mut loaded = BTreeMap::new();
    for b in &in_use {
        loaded.insert(b.clone(), Arc::new(load(&newest, b).await?));
    }
    std::fs::create_dir_all(dir())?;
    std::fs::write(dir().join("current.json"), serde_json::to_string(&newest)?)?;
    log::info!("AramHex data {} (patch {}, {} games)", newest.seq, newest.patch, newest.matches);
    let keep = newest.seq.to_string();
    {
        let mut s = STATE.lock();
        s.info = Some(Arc::new(newest));
        s.brackets = loaded;
    }
    if let Ok(entries) = std::fs::read_dir(dir()) {
        for e in entries.flatten() {
            if e.path().is_dir() && e.file_name().to_string_lossy() != keep {
                let _ = std::fs::remove_dir_all(e.path());
            }
        }
    }
    Ok(())
}

pub fn spawn() {
    tauri::async_runtime::spawn(async {
        tokio::time::sleep(Duration::from_secs(30)).await;
        loop {
            if let Err(e) = update().await {
                log::warn!("AramHex update: {e:#}");
            }
            tokio::time::sleep(CHECK_EVERY).await;
        }
    });
}

// ---------------------------------------------------------------------------------------------
// Views for the UI

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub patch: String,
    pub snapshot_at: String,
    pub matches: u64,
    pub bracket: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChampRow {
    pub champion_id: i32,
    pub games: u64,
    pub win_rate: f64,
    pub pick_rate: f64,
    pub rank: u32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AugRow {
    pub id: i32,
    pub games: u64,
    pub win_rate: f64,
    /// Win rate pulled toward the baseline by `SMOOTH_GAMES` games (small samples count less).
    pub smoothed: f64,
    /// `smoothed` minus the baseline (the champion's win rate, or 50% overall).
    pub delta: f64,
    pub low_sample: bool,
}

fn aug_row(id: i32, s: &Stat, base: f64) -> AugRow {
    let smoothed = (s.wins as f64 + base * SMOOTH_GAMES) / (s.games as f64 + SMOOTH_GAMES);
    AugRow { id, games: s.games, win_rate: s.win_rate(), smoothed, delta: smoothed - base, low_sample: s.low_sample }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub source: Source,
    pub champions: Vec<ChampRow>,
    /// Every augment over all champions, best first.
    pub augments: Vec<AugRow>,
}

fn source(info: &Info, bracket: &str) -> Source {
    Source { patch: info.patch.clone(), snapshot_at: info.snapshot_at.clone(), matches: info.matches, bracket: bracket.to_string() }
}

pub fn overview(info: &Info, bracket_name: &str, b: &Bracket) -> Overview {
    let mut champions: Vec<ChampRow> = b
        .heroes
        .iter()
        .map(|(id, s)| ChampRow { champion_id: *id, games: s.games, win_rate: s.win_rate(), pick_rate: s.pick_rate, rank: s.rank })
        .collect();
    champions.sort_by(|a, b| b.win_rate.total_cmp(&a.win_rate));
    let mut totals: HashMap<i32, Stat> = HashMap::new();
    for pairs in b.hero_augments.values() {
        for (aug, s) in pairs {
            let t = totals.entry(*aug).or_default();
            t.games += s.games;
            t.wins += s.wins;
        }
    }
    let mut augments: Vec<AugRow> = totals.iter().map(|(id, s)| aug_row(*id, s, 0.5)).collect();
    augments.sort_by(|a, b| b.smoothed.total_cmp(&a.smoothed));
    Overview { source: source(info, bracket_name), champions, augments }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Champion {
    pub source: Source,
    pub champion: Option<ChampRow>,
    /// The champion's augments, best (by difference from the champion's own win rate) first.
    pub augments: Vec<AugRow>,
}

pub fn champion(info: &Info, bracket_name: &str, b: &Bracket, champion_id: i32) -> Champion {
    let hero = b.heroes.get(&champion_id);
    let base = hero.map(Stat::win_rate).filter(|w| *w > 0.0).unwrap_or(0.5);
    let mut augments: Vec<AugRow> =
        b.hero_augments.get(&champion_id).map(|m| m.iter().map(|(id, s)| aug_row(*id, s, base)).collect()).unwrap_or_default();
    augments.sort_by(|a, b| b.delta.total_cmp(&a.delta));
    Champion {
        source: source(info, bracket_name),
        champion: hero.map(|s| ChampRow { champion_id, games: s.games, win_rate: s.win_rate(), pick_rate: s.pick_rate, rank: s.rank }),
        augments,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEROES: &str = r#"{"24": {"games": 1000, "wins": 560, "win_rate": 0.56, "pick_rate": 0.01, "rank": 3, "name": "贾克斯"},
                             "1": {"games": 500, "wins": 240, "win_rate": 0.48, "pick_rate": 0.004, "rank": 90}}"#;
    const PAIRS: &str = r#"{"24": {"1001": {"games": 400, "wins": 260, "win_rate": 0.65, "low_sample": false},
                                   "1002": {"games": 10, "wins": 10, "win_rate": 1.0, "low_sample": true},
                                   "1003": {"games": 300, "wins": 150, "win_rate": 0.5, "low_sample": false}},
                            "1": {"1001": {"games": 100, "wins": 50, "win_rate": 0.5, "low_sample": false}}}"#;

    fn info() -> Info {
        parse_info(&serde_json::json!({"seq": 11, "patch": "16.21", "path": "patches/16.21/normal", "snapshot_at": "x", "matches": 9, "bands": true,
            "files": {"heroes.json": {"sha256": "ab"}, "bands/high/heroes.json": {"sha256": "cd"}}}))
        .unwrap()
    }

    #[test]
    fn reads_index_and_paths() {
        let i = info();
        assert_eq!((i.seq, i.patch.as_str(), i.files.len()), (11, "16.21", 2));
        assert_eq!(rel("all", "heroes.json"), "heroes.json");
        assert_eq!(rel("high", "heroes.json"), "bands/high/heroes.json");
        assert!(parse_info(&serde_json::json!({"seq": 1})).is_err());
        assert_eq!(sha256(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn champion_augments_ranked_by_smoothed_difference() {
        let b = parse_bracket(HEROES.as_bytes(), PAIRS.as_bytes()).unwrap();
        let c = champion(&info(), "all", &b, 24);
        assert_eq!(c.champion.as_ref().map(|h| h.games), Some(1000));
        let order: Vec<i32> = c.augments.iter().map(|a| a.id).collect();
        assert_eq!(order, [1001, 1002, 1003], "10 straight wins are pulled close to Jax's 56%");
        let small = &c.augments[1];
        assert!(small.low_sample && small.delta < 0.02 && small.delta > 0.0);
        assert!(c.augments[2].delta < 0.0, "50% on a 56% champion is below average");
    }

    #[test]
    fn overview_sums_augments_over_champions() {
        let b = parse_bracket(HEROES.as_bytes(), PAIRS.as_bytes()).unwrap();
        let o = overview(&info(), "all", &b);
        assert_eq!(o.champions[0].champion_id, 24, "highest win rate first");
        let a = o.augments.iter().find(|a| a.id == 1001).unwrap();
        assert_eq!((a.games, (a.win_rate * 100.0).round()), (500, 62.0));
    }
}
