//! Augment card samples, for building and testing the in-game augment reader on real screens.
//!
//! Only when "Collect augment card samples" is on in Settings (off by default) and only in
//! Hextech ARAM: while you are dead at level 3 or higher, about once a second, the middle band
//! of the game window (where the augment cards appear) is saved as a PNG, and what the game's
//! live data says about you (level, death, runes, where held augments are listed) is logged on
//! every change. Nothing leaves the PC: %APPDATA%/Mattofessor/augment-captures/<game start>/.

use crate::win;
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::io::Write;
use std::path::PathBuf;

/// The part of the window height kept: the cards sit in the middle.
const BAND: (f64, f64) = (0.12, 0.88);
const MAX_PER_DEATH: u32 = 40;

struct Session {
    dir: PathBuf,
    last_state: String,
    shots: u32,
}

static SESSION: Mutex<Option<Session>> = Mutex::new(None);

pub fn root() -> PathBuf {
    crate::paths::config_dir().join("augment-captures")
}

/// Files saved so far (all games).
pub fn count() -> usize {
    walkdir(&root()).iter().filter(|p| p.extension().is_some_and(|e| e == "png")).count()
}

fn walkdir(dir: &PathBuf) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(walkdir(&p));
            } else {
                out.push(p);
            }
        }
    }
    out
}

/// The game ended: the next game gets its own folder.
pub fn game_over() {
    *SESSION.lock() = None;
}

/// Your own entry among the game's players.
fn me(raw: &Value) -> Option<&Value> {
    let ap = raw.get("activePlayer")?;
    let id = ap.get("riotId").or_else(|| ap.get("summonerName"))?.as_str()?;
    raw.get("allPlayers")?.as_array()?.iter().find(|p| {
        p.get("riotId").and_then(Value::as_str) == Some(id) || p.get("summonerName").and_then(Value::as_str) == Some(id)
    })
}

/// Called once a second with the game's live data while a game runs.
pub fn tick(enabled: bool, raw: &Value) {
    if !enabled || raw.pointer("/gameData/gameMode").and_then(Value::as_str) != Some("KIWI") {
        return;
    }
    let Some(me) = me(raw) else { return };
    let level = me.get("level").and_then(Value::as_i64).unwrap_or(0);
    let dead = me.get("isDead").and_then(Value::as_bool).unwrap_or(false);
    let mut guard = SESSION.lock();
    let s = guard.get_or_insert_with(|| Session {
        dir: root().join(chrono::Local::now().format("%Y%m%d-%H%M%S").to_string()),
        last_state: String::new(),
        shots: 0,
    });
    let _ = std::fs::create_dir_all(&s.dir);
    let game_time = raw.pointer("/gameData/gameTime").and_then(Value::as_f64).unwrap_or(0.0);
    let state = json!([level, dead, me.get("runes"), raw.pointer("/activePlayer/fullRunes")]).to_string();
    if state != s.last_state {
        s.last_state = state;
        if !dead {
            s.shots = 0;
        }
        let line = json!({
            "gameTime": (game_time * 10.0).round() / 10.0,
            "level": level,
            "isDead": dead,
            "champion": me.get("championName"),
            "runes": me.get("runes"),
            "activeFullRunes": raw.pointer("/activePlayer/fullRunes"),
        });
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(s.dir.join("live.jsonl")) {
            let _ = writeln!(f, "{line}");
        }
    }
    if !(dead && level >= 3 && s.shots < MAX_PER_DEATH) {
        return;
    }
    let Some(r) = win::game_window().and_then(win::window_rect) else { return };
    let band = win::Rect { x: r.x, y: r.y + (r.h as f64 * BAND.0) as i32, w: r.w, h: (r.h as f64 * (BAND.1 - BAND.0)) as i32 };
    let Some(px) = win::capture(band) else { return };
    let path = s.dir.join(format!("t{:06.0}_lvl{level:02}_{:02}.png", game_time, s.shots));
    if let Err(e) = save_png(&path, band.w as u32, band.h as u32, &px) {
        log::warn!("augment capture: {e:#}");
        return;
    }
    s.shots += 1;
}

fn save_png(path: &PathBuf, w: u32, h: u32, rgba: &[u8]) -> anyhow::Result<()> {
    let file = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut enc = png::Encoder::new(file, w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_compression(png::Compression::Fast);
    enc.write_header()?.write_image_data(rgba)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_me_and_ignores_other_modes() {
        let raw = json!({"gameData": {"gameMode": "ARAM"}, "activePlayer": {"riotId": "Me#NA1"},
            "allPlayers": [{"riotId": "Other#NA1"}, {"riotId": "Me#NA1", "level": 7, "isDead": true}]});
        assert_eq!(me(&raw).and_then(|p| p.get("level")).and_then(Value::as_i64), Some(7));
        tick(true, &raw);
        assert!(SESSION.lock().is_none(), "classic ARAM is not captured");
        tick(false, &json!({"gameData": {"gameMode": "KIWI"}}));
        assert!(SESSION.lock().is_none(), "off does nothing");
    }

    #[test]
    fn png_roundtrip() {
        let dir = std::env::temp_dir().join("mf-capture-test");
        let _ = std::fs::create_dir_all(&dir);
        let p = dir.join("x.png");
        save_png(&p, 2, 1, &[255, 0, 0, 255, 0, 255, 0, 255]).unwrap();
        let bytes = std::fs::read(&p).unwrap();
        assert_eq!(&bytes[1..4], b"PNG");
        let _ = std::fs::remove_dir_all(dir);
    }
}
