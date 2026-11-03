//! Local SQLite cache (%APPDATA%\Mattofessor\cache.db).
//!
//! A finished game never changes, so game rows and full games are kept forever and only new
//! ones are fetched. Rank snapshots (one per player, queue and day) are how the app builds a
//! rank history the League client itself does not keep.

use crate::model::{GameRow, Rank};
use anyhow::Result;
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
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

#[derive(Clone, Debug)]
pub struct BuildRow {
    pub champion_id: i32,
    pub role: String,
    pub win: bool,
    pub items: Vec<i32>,
    pub perks: Vec<i32>,
    pub perk_primary: i32,
    pub perk_sub: i32,
    pub spells: [i32; 2],
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        Self::init(Connection::open(path)?)
    }

    pub fn open_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(db: Connection) -> Result<Self> {
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
             CREATE TABLE IF NOT EXISTS build_rows (
               game_id INTEGER NOT NULL, puuid TEXT NOT NULL, champion_id INTEGER NOT NULL, role TEXT NOT NULL,
               win INTEGER NOT NULL, created INTEGER NOT NULL, items TEXT NOT NULL, perks TEXT NOT NULL,
               perk_primary INTEGER NOT NULL, perk_sub INTEGER NOT NULL, spells TEXT NOT NULL,
               PRIMARY KEY (game_id, puuid));
             CREATE INDEX IF NOT EXISTS build_rows_champ ON build_rows (champion_id, created);
             CREATE TABLE IF NOT EXISTS build_crawl (puuid TEXT PRIMARY KEY, crawled_at INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS kv (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS players (puuid TEXT PRIMARY KEY, json TEXT NOT NULL, updated INTEGER NOT NULL);",
        )?;
        // v2: build rows store role groups from `positions` (v1 trusted the client's lane labels).
        let version: i64 = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version < 2 {
            db.execute_batch("DELETE FROM build_rows; DELETE FROM build_crawl; DELETE FROM kv WHERE key = 'crawl_finished'; PRAGMA user_version = 2;")?;
        }
        Ok(Self { db: Mutex::new(db) })
    }

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

    pub fn crawled_at(&self, puuid: &str) -> Option<i64> {
        self.db.lock().query_row("SELECT crawled_at FROM build_crawl WHERE puuid = ?1", params![puuid], |r| r.get(0)).optional().ok()?
    }

    /// Rows with their role (group) for builds.
    pub fn save_build_rows(&self, puuid: &str, rows: &[(GameRow, String)], now: i64) -> Result<()> {
        let mut db = self.db.lock();
        let tx = db.transaction()?;
        {
            let mut st = tx.prepare_cached(
                "INSERT OR IGNORE INTO build_rows (game_id, puuid, champion_id, role, win, created, items, perks, perk_primary, perk_sub, spells)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            )?;
            for (r, role) in rows {
                st.execute(params![
                    r.game_id as i64,
                    puuid,
                    r.champion_id,
                    role,
                    r.win,
                    r.created,
                    serde_json::to_string(&r.items)?,
                    serde_json::to_string(&r.perks)?,
                    r.perk_primary,
                    r.perk_sub,
                    serde_json::to_string(&r.spells)?,
                ])?;
            }
            tx.execute("INSERT OR REPLACE INTO build_crawl (puuid, crawled_at) VALUES (?1, ?2)", params![puuid, now])?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn build_rows(&self, champion_id: i32, since: i64) -> Result<Vec<BuildRow>> {
        let db = self.db.lock();
        let mut st = db.prepare_cached(
            "SELECT champion_id, role, win, items, perks, perk_primary, perk_sub, spells FROM build_rows WHERE champion_id = ?1 AND created >= ?2",
        )?;
        let rows = st
            .query_map(params![champion_id, since], |r| {
                let items: String = r.get(3)?;
                let perks: String = r.get(4)?;
                let spells: String = r.get(7)?;
                Ok(BuildRow {
                    champion_id: r.get(0)?,
                    role: r.get(1)?,
                    win: r.get(2)?,
                    items: serde_json::from_str(&items).unwrap_or_default(),
                    perks: serde_json::from_str(&perks).unwrap_or_default(),
                    perk_primary: r.get(5)?,
                    perk_sub: r.get(6)?,
                    spells: serde_json::from_str(&spells).unwrap_or([0, 0]),
                })
            })?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }

    /// (games, champions) in the build sample since `since`.
    pub fn build_sample(&self, since: i64) -> (u64, u64) {
        self.db
            .lock()
            .query_row("SELECT COUNT(*), COUNT(DISTINCT champion_id) FROM build_rows WHERE created >= ?1", params![since], |r| {
                Ok((r.get::<_, i64>(0)? as u64, r.get::<_, i64>(1)? as u64))
            })
            .unwrap_or((0, 0))
    }

    pub fn prune_build_rows(&self, before: i64) {
        let _ = self.db.lock().execute("DELETE FROM build_rows WHERE created < ?1", params![before]);
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
