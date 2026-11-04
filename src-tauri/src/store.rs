//! Local SQLite cache (%APPDATA%\Mattofessor\cache.db).
//!
//! A finished game never changes, so game rows and full games are kept forever and only new
//! ones are fetched. Rank snapshots (one per player, queue and day) are how the app builds a
//! rank history the League client itself does not keep.

use crate::model::{GameRow, Rank};
use anyhow::Result;
use parking_lot::Mutex;
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use serde_json::Value;
use std::path::Path;
use std::sync::OnceLock;

pub struct Store {
    db: Mutex<Connection>,
}

static STORE: OnceLock<Store> = OnceLock::new();

pub fn get() -> &'static Store {
    STORE.get_or_init(|| {
        let path = crate::paths::config_dir().join("cache.db");
        Store::open(&path).unwrap_or_else(|e| {
            log::error!("cache database unusable ({e:#}); using memory");
            Store::open_memory().expect("in-memory sqlite")
        })
    })
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RankPoint {
    /// YYYY-MM-DD
    pub day: String,
    pub queue: String,
    pub tier: String,
    pub division: String,
    pub lp: i32,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if path.exists() && !healthy(path) {
            rebuild(path);
        }
        Self::init(Connection::open(path)?)
    }

    pub fn open_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(db: Connection) -> Result<Self> {
        schema(&db)?;
        Ok(Self { db: Mutex::new(db) })
    }
}

/// Tables, and the upgrades of older versions.
fn schema(db: &Connection) -> Result<()> {
    db.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             CREATE TABLE IF NOT EXISTS player_games (
               puuid TEXT NOT NULL, game_id INTEGER NOT NULL, created INTEGER NOT NULL, json TEXT NOT NULL,
               PRIMARY KEY (puuid, game_id));
             CREATE INDEX IF NOT EXISTS player_games_recent ON player_games (puuid, created DESC);
             CREATE TABLE IF NOT EXISTS games (game_id INTEGER PRIMARY KEY, json TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS rank_snapshots (
               puuid TEXT NOT NULL, day TEXT NOT NULL, queue TEXT NOT NULL, tier TEXT NOT NULL, division TEXT NOT NULL, lp INTEGER NOT NULL,
               PRIMARY KEY (puuid, day, queue));
             CREATE TABLE IF NOT EXISTS kv (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS players (puuid TEXT PRIMARY KEY, json TEXT NOT NULL, updated INTEGER NOT NULL);",
    )?;
    // v3: builds come from OP.GG; the old crawler's tables go.
    let version: i64 = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version < 3 {
        db.execute_batch(
            "DROP TABLE IF EXISTS build_rows; DROP TABLE IF EXISTS build_crawl; DELETE FROM kv WHERE key = 'crawl_finished'; PRAGMA user_version = 3;",
        )?;
    }
    Ok(())
}

/// Whether the database file passes SQLite's own consistency check.
fn healthy(path: &Path) -> bool {
    let check = Connection::open(path).and_then(|db| db.query_row("PRAGMA quick_check", [], |r| r.get::<_, String>(0)));
    match check {
        Ok(s) if s == "ok" => true,
        Ok(s) => {
            log::warn!("cache database damaged: {}", s.lines().nth(1).or(s.lines().next()).unwrap_or_default());
            false
        }
        Err(e) => {
            log::warn!("cache database unreadable: {e}");
            false
        }
    }
}

/// Every table with its columns, and which column holds JSON (checked before a row is kept).
const TABLES: [(&str, &[&str], Option<usize>); 5] = [
    ("player_games", &["puuid", "game_id", "created", "json"], Some(3)),
    ("games", &["game_id", "json"], Some(1)),
    ("rank_snapshots", &["puuid", "day", "queue", "tier", "division", "lp"], None),
    ("kv", &["key", "value"], None),
    ("players", &["puuid", "json", "updated"], Some(1)),
];

/// A damaged cache is rebuilt from every row still readable; the rest (games, profiles) comes back
/// from the League client. The damaged file is kept beside it as `cache.db.damaged-<time>`.
fn rebuild(path: &Path) {
    let fresh = side(path, ".rebuild");
    for suffix in ["", "-wal", "-shm", "-journal"] {
        let _ = std::fs::remove_file(side(&fresh, suffix));
    }
    match salvage(path, &fresh) {
        Ok(kept) => log::info!("cache database rebuilt, rows kept: {kept:?}"),
        Err(e) => {
            log::warn!("cache database could not be salvaged ({e:#}); starting empty");
            let _ = std::fs::remove_file(&fresh);
        }
    }
    let damaged = side(path, &format!(".damaged-{}", chrono::Local::now().format("%Y%m%d-%H%M%S")));
    for suffix in ["", "-wal", "-shm"] {
        let from = side(path, suffix);
        if from.exists() {
            if let Err(e) = std::fs::rename(&from, side(&damaged, suffix)) {
                log::warn!("moving {}: {e}", from.display());
            }
        }
    }
    if fresh.exists() {
        if let Err(e) = std::fs::rename(&fresh, path) {
            log::warn!("installing the rebuilt cache: {e}");
        }
    }
}

/// `path` with `suffix` appended to its file name.
fn side(path: &Path, suffix: &str) -> std::path::PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(suffix);
    s.into()
}

/// Copy the readable rows of `src` into a new database at `dst`; rows kept per table.
fn salvage(src: &Path, dst: &Path) -> Result<Vec<(&'static str, usize)>> {
    let old = Connection::open_with_flags(src, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let new = Connection::open(dst)?;
    // A plain journal while copying: the result is one self-contained file to move into place.
    new.execute_batch("PRAGMA journal_mode=DELETE;")?;
    schema(&new)?;
    let mut kept = Vec::new();
    for (table, cols, json_col) in TABLES {
        let list = cols.join(", ");
        let insert = format!("INSERT OR IGNORE INTO {table} ({list}) VALUES ({})", vec!["?"; cols.len()].join(", "));
        let tx = new.unchecked_transaction()?;
        let mut n = 0;
        // Reading stops at the first damaged page, so read from both ends.
        for order in ["ASC", "DESC"] {
            let Ok(mut st) = old.prepare(&format!("SELECT {list} FROM {table} ORDER BY rowid {order}")) else { break };
            let Ok(mut rows) = st.query([]) else { break };
            loop {
                let row = match rows.next() {
                    Ok(Some(r)) => r,
                    Ok(None) => break,
                    Err(e) => {
                        log::warn!("salvaging {table} ({order}): stopped at {e}");
                        break;
                    }
                };
                let vals: Vec<rusqlite::types::Value> = (0..cols.len()).filter_map(|i| row.get(i).ok()).collect();
                if vals.len() != cols.len() {
                    continue;
                }
                if let Some(c) = json_col {
                    let rusqlite::types::Value::Text(j) = &vals[c] else { continue };
                    if serde_json::from_str::<Value>(j).is_err() {
                        continue;
                    }
                }
                n += tx.execute(&insert, rusqlite::params_from_iter(vals))?;
            }
        }
        tx.commit()?;
        kept.push((table, n));
    }
    Ok(kept)
}

impl Store {

    pub fn save_rows(&self, puuid: &str, rows: &[GameRow]) -> Result<()> {
        let mut db = self.db.lock();
        let tx = db.transaction()?;
        {
            let mut st = tx.prepare_cached("INSERT OR REPLACE INTO player_games (puuid, game_id, created, json) VALUES (?1, ?2, ?3, ?4)")?;
            for r in rows {
                st.execute(params![puuid, r.game_id as i64, r.created, serde_json::to_string(r)?])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Most recent rows first.
    pub fn rows(&self, puuid: &str, limit: usize) -> Result<Vec<GameRow>> {
        let db = self.db.lock();
        let mut st = db.prepare_cached("SELECT json FROM player_games WHERE puuid = ?1 ORDER BY created DESC LIMIT ?2")?;
        let rows = st
            .query_map(params![puuid, limit as i64], |r| r.get::<_, String>(0))?
            .filter_map(|j| j.ok().and_then(|j| serde_json::from_str(&j).ok()))
            .collect();
        Ok(rows)
    }

    /// The last lookup of a player (summoner, rank, mastery) and when it happened (ms).
    pub fn save_player(&self, puuid: &str, json: &Value, updated: i64) -> Result<()> {
        self.db.lock().execute(
            "INSERT OR REPLACE INTO players (puuid, json, updated) VALUES (?1, ?2, ?3)",
            params![puuid, json.to_string(), updated],
        )?;
        Ok(())
    }

    pub fn player(&self, puuid: &str) -> Option<(Value, i64)> {
        let db = self.db.lock();
        let row: Option<(String, i64)> = db
            .query_row("SELECT json, updated FROM players WHERE puuid = ?1", params![puuid], |r| Ok((r.get(0)?, r.get(1)?)))
            .optional()
            .ok()?;
        row.and_then(|(j, at)| serde_json::from_str(&j).ok().map(|v| (v, at)))
    }

    pub fn save_game(&self, game_id: u64, json: &Value) -> Result<()> {
        self.db.lock().execute("INSERT OR REPLACE INTO games (game_id, json) VALUES (?1, ?2)", params![game_id as i64, json.to_string()])?;
        Ok(())
    }

    pub fn game(&self, game_id: u64) -> Option<Value> {
        let db = self.db.lock();
        let j: Option<String> =
            db.query_row("SELECT json FROM games WHERE game_id = ?1", params![game_id as i64], |r| r.get(0)).optional().ok()?;
        j.and_then(|j| serde_json::from_str(&j).ok())
    }

    pub fn snapshot_rank(&self, puuid: &str, day: &str, rank: &Rank) -> Result<()> {
        self.db.lock().execute(
            "INSERT OR REPLACE INTO rank_snapshots (puuid, day, queue, tier, division, lp) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![puuid, day, rank.queue, rank.tier, rank.division, rank.lp],
        )?;
        Ok(())
    }

    /// Oldest first.
    pub fn rank_history(&self, puuid: &str) -> Result<Vec<RankPoint>> {
        let db = self.db.lock();
        let mut st = db.prepare_cached("SELECT day, queue, tier, division, lp FROM rank_snapshots WHERE puuid = ?1 ORDER BY day")?;
        let rows = st
            .query_map(params![puuid], |r| {
                Ok(RankPoint { day: r.get(0)?, queue: r.get(1)?, tier: r.get(2)?, division: r.get(3)?, lp: r.get(4)? })
            })?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }

    pub fn kv_get(&self, key: &str) -> Option<String> {
        self.db.lock().query_row("SELECT value FROM kv WHERE key = ?1", params![key], |r| r.get(0)).optional().ok()?
    }

    pub fn kv_set(&self, key: &str, value: &str) {
        let _ = self.db.lock().execute("INSERT OR REPLACE INTO kv (key, value) VALUES (?1, ?2)", params![key, value]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damaged_database_is_rebuilt_from_readable_rows() {
        let dir = std::env::temp_dir().join(format!("mattofessor-store-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("cache.db");
        {
            let s = Store::open(&path).unwrap();
            let rows: Vec<GameRow> = (0..400).map(|i| GameRow { game_id: i, created: i as i64, ..Default::default() }).collect();
            s.save_rows("p", &rows).unwrap();
            s.kv_set("last_me", "p");
            s.db.lock().execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").unwrap();
        }
        // Overwrite a page in the middle of the game rows with garbage.
        let mut bytes = std::fs::read(&path).unwrap();
        let page = 4096;
        let mid = (bytes.len() / page / 2) * page;
        for b in &mut bytes[mid..mid + page] {
            *b = 0xA5;
        }
        std::fs::write(&path, &bytes).unwrap();
        assert!(!healthy(&path));

        let s = Store::open(&path).unwrap();
        assert!(healthy(&path));
        let kept = s.rows("p", 1000).unwrap().len();
        assert!(kept >= 200, "kept {kept}");
        assert_eq!(s.kv_get("last_me").as_deref(), Some("p"));
        let damaged = std::fs::read_dir(&dir).unwrap().filter_map(Result::ok).any(|e| e.file_name().to_string_lossy().starts_with("cache.db.damaged-"));
        assert!(damaged);
        drop(s);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn players_are_kept_per_account() {
        let s = Store::open_memory().unwrap();
        assert!(s.player("a").is_none());
        s.save_player("a", &serde_json::json!({"summoner": {"gameName": "A"}}), 100).unwrap();
        s.save_player("b", &serde_json::json!({"summoner": {"gameName": "B"}}), 200).unwrap();
        s.save_player("a", &serde_json::json!({"summoner": {"gameName": "A2"}}), 300).unwrap();
        let (a, at) = s.player("a").unwrap();
        assert_eq!((a["summoner"]["gameName"].as_str(), at), (Some("A2"), 300));
        assert_eq!(s.player("b").unwrap().1, 200, "another account's copy is untouched");
    }

    #[test]
    fn rows_roundtrip_newest_first() {
        let s = Store::open_memory().unwrap();
        let mk = |id, created| GameRow { game_id: id, created, champion_id: 1, ..Default::default() };
        s.save_rows("p", &[mk(1, 100), mk(2, 300), mk(3, 200)]).unwrap();
        s.save_rows("p", &[mk(2, 300)]).unwrap();
        let got: Vec<u64> = s.rows("p", 10).unwrap().iter().map(|r| r.game_id).collect();
        assert_eq!(got, vec![2, 3, 1]);
        assert_eq!(s.rows("p", 1).unwrap().len(), 1);
        assert!(s.rows("other", 10).unwrap().is_empty());
    }

    #[test]
    fn rank_snapshots_one_per_day() {
        let s = Store::open_memory().unwrap();
        let r = |lp| Rank { queue: "solo".into(), tier: "GOLD".into(), division: "I".into(), lp, wins: 0, losses: 0 };
        s.snapshot_rank("p", "2026-10-01", &r(10)).unwrap();
        s.snapshot_rank("p", "2026-10-01", &r(30)).unwrap();
        s.snapshot_rank("p", "2026-10-02", &r(50)).unwrap();
        let h = s.rank_history("p").unwrap();
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].lp, 30);
    }
}
