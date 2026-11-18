//! Refreshed game data that every copy of the app follows: Hextech ARAM champion modifiers, rules
//! and headliners (`mayhem.json`) and the latest patch notes (`patch-notes.json`), with
//! `meta.json` saying when they were built. The `data` branch is written by
//! .github/workflows/refresh-data.yml (tools/refresh-data) every 6 hours when something changed.
//! Apps check it every 30 minutes and start from the copy they shipped with (`data/*.json`) or the
//! last one fetched, whichever was built later. The JSON goes to the page as it is.

use anyhow::{Context as _, Result};
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::Emitter;

const CHECK_EVERY: Duration = Duration::from_secs(30 * 60);
const META: &str = "meta.json";
const MAYHEM: &str = "mayhem.json";
const PATCH_NOTES: &str = "patch-notes.json";
const SHIPPED: [&str; 3] = [include_str!("../data/meta.json"), include_str!("../data/mayhem.json"), include_str!("../data/patch-notes.json")];

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    /// When the data was built (RFC 3339); the later copy wins.
    pub checked_at: String,
    pub mayhem: Value,
    pub patch_notes: Value,
}

impl Data {
    /// From the three files' text; none when any is unreadable.
    fn parse(meta: &[u8], mayhem: &[u8], patch_notes: &[u8]) -> Result<Data> {
        let meta: Value = serde_json::from_slice(meta).context(META)?;
        let checked_at = meta.get("checkedAt").and_then(Value::as_str).unwrap_or_default().to_string();
        let mayhem: Value = serde_json::from_slice(mayhem).context(MAYHEM)?;
        let patch_notes: Value = serde_json::from_slice(patch_notes).context(PATCH_NOTES)?;
        if mayhem.get("champions").and_then(Value::as_object).is_none_or(|c| c.is_empty()) {
            anyhow::bail!("{MAYHEM} has no champions");
        }
        Ok(Data { checked_at, mayhem, patch_notes })
    }
}

/// The branch: `data`, or `MATTOFESSOR_DATA_BRANCH` (to try a test branch).
fn branch() -> String {
    std::env::var("MATTOFESSOR_DATA_BRANCH").ok().filter(|b| !b.is_empty()).unwrap_or_else(|| "data".into())
}

fn dir() -> PathBuf {
    crate::paths::config_dir().join("data")
}

fn shipped() -> Data {
    Data::parse(SHIPPED[0].as_bytes(), SHIPPED[1].as_bytes(), SHIPPED[2].as_bytes()).expect("the shipped data reads")
}

fn fetched() -> Option<Data> {
    let read = |f: &str| std::fs::read(dir().join(f)).ok();
    Data::parse(&read(META)?, &read(MAYHEM)?, &read(PATCH_NOTES)?).ok()
}

static CURRENT: Mutex<Option<Arc<Data>>> = Mutex::new(None);

/// The data in use.
pub fn current() -> Arc<Data> {
    CURRENT
        .lock()
        .get_or_insert_with(|| {
            let s = shipped();
            Arc::new(match fetched() {
                Some(f) if f.checked_at > s.checked_at => f,
                _ => s,
            })
        })
        .clone()
}

/// Keeps a newer copy, on disk first (each file replaced whole), then in use.
fn receive(files: [Vec<u8>; 3]) -> Result<()> {
    let d = Data::parse(&files[0], &files[1], &files[2])?;
    if d.checked_at <= current().checked_at {
        return Ok(());
    }
    std::fs::create_dir_all(dir())?;
    // Meta last: a copy cut short keeps the old meta, so it never looks newer than it is.
    for (name, bytes) in [(MAYHEM, &files[1]), (PATCH_NOTES, &files[2]), (META, &files[0])] {
        let tmp = dir().join(format!("{name}.tmp"));
        std::fs::write(&tmp, bytes)?;
        std::fs::rename(&tmp, dir().join(name))?;
    }
    log::info!("data updated ({})", d.checked_at);
    let checked_at = d.checked_at.clone();
    *CURRENT.lock() = Some(Arc::new(d));
    if let Some(core) = crate::core::get() {
        let _ = core.app.emit("data", checked_at);
    }
    Ok(())
}

async fn check(seen: &mut Option<String>) -> Result<()> {
    let Some(rev) = crate::branch::next(&branch(), seen).await else { return Ok(()) };
    let (meta, mayhem, notes) =
        tokio::try_join!(crate::branch::file(&rev, META), crate::branch::file(&rev, MAYHEM), crate::branch::file(&rev, PATCH_NOTES))?;
    receive([meta, mayhem, notes])?;
    crate::branch::read_done(seen, &rev);
    Ok(())
}

pub fn spawn() {
    tauri::async_runtime::spawn(async {
        tokio::time::sleep(Duration::from_secs(5)).await;
        let mut seen = None;
        loop {
            if let Err(e) = check(&mut seen).await {
                log::info!("data check: {e:#}");
            }
            tokio::time::sleep(CHECK_EVERY).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_data_reads() {
        let d = shipped();
        assert!(!d.checked_at.is_empty());
        let sion = &d.mayhem["champions"]["14"];
        assert!(sion["stats"]["dealt"].as_f64().is_some(), "Sion has his damage dealt modifier");
        assert!(d.patch_notes["patches"].as_array().is_some_and(|p| !p.is_empty()));
    }

    #[test]
    fn data_without_champions_is_refused() {
        assert!(Data::parse(br#"{"checkedAt": "x"}"#, br#"{"champions": {}}"#, b"{}").is_err());
        assert!(Data::parse(br#"{"checkedAt": "x"}"#, b"not json", b"{}").is_err());
    }
}
