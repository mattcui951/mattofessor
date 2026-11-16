//! League's window mode. A game in Full screen owns the screen: no other window can show over it,
//! so Mattofessor's in-game panel, spell timers and card win rates stay hidden behind it (only
//! Alt+Tab shows them). Borderless looks the same and lets them show. The app reads the mode to
//! warn about it and, when the player asks, switches it through the League client.

use crate::lcu;
use anyhow::Result;
use serde_json::{json, Value};

const SETTINGS: &str = "/lol-game-settings/v1/game-settings";

/// League's `WindowMode` number ("General" section) to a name.
fn name(mode: i64) -> &'static str {
    match mode {
        0 => "fullscreen",
        1 => "windowed",
        2 => "borderless",
        _ => "unknown",
    }
}

/// `WindowMode` from the text of League's `Config/game.cfg`.
pub fn from_cfg(text: &str) -> Option<i64> {
    let mut general = false;
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            general = l.eq_ignore_ascii_case("[General]");
        } else if general {
            if let Some((k, v)) = l.split_once('=') {
                if k.trim() == "WindowMode" {
                    return v.trim().parse().ok();
                }
            }
        }
    }
    None
}

/// The League install folder: the client's (lockfile) or the game's.
fn install_dir() -> Option<std::path::PathBuf> {
    if let Some(d) = lcu::install_dir() {
        return Some(d);
    }
    let procs = crate::win::processes();
    let game = crate::win::find_process(&procs, &crate::win::game_exe()).and_then(crate::win::process_path)?;
    // <install>/Game/League of Legends.exe
    game.parent()?.parent().map(|d| d.to_path_buf())
}

fn cfg_path() -> Option<std::path::PathBuf> {
    Some(install_dir()?.join("Config").join("game.cfg"))
}

/// The mode in the game's config file: what the game uses (the client writes it before a game,
/// the game when its own settings change).
fn file_mode() -> Option<&'static str> {
    from_cfg(&std::fs::read_to_string(cfg_path()?).ok()?).map(name)
}

/// "fullscreen", "windowed", "borderless" or "unknown"; `None` when League cannot be found.
/// The game's config file, else the client's setting.
pub async fn window_mode() -> Option<&'static str> {
    if let Some(m) = file_mode() {
        return Some(m);
    }
    let v = lcu::current()?.get_value(SETTINGS).await.ok()?;
    v.pointer("/General/WindowMode").and_then(Value::as_i64).map(name)
}

/// A game is running in Full screen (the in-game windows then stay closed).
pub fn game_fullscreen() -> bool {
    crate::win::game_running() && file_mode() == Some("fullscreen")
}

/// `text` (a game.cfg) with Borderless as its window mode.
pub fn with_borderless(text: &str) -> String {
    let mut general = false;
    let mut out: Vec<String> = Vec::new();
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            general = l.eq_ignore_ascii_case("[General]");
        } else if general && l.split_once('=').is_some_and(|(k, _)| k.trim() == "WindowMode") {
            out.push("WindowMode=2".into());
            continue;
        }
        out.push(line.to_string());
    }
    let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
    out.join(eol) + if text.ends_with('\n') { eol } else { "" }
}

/// Switch League to Borderless: in the client (what it gives the next game) and, with no game
/// running, in the config file too. A running game keeps its mode until changed in game.
pub async fn set_borderless() -> Result<()> {
    let mut done = false;
    let mut last_err = None;
    if let Some(lcu) = lcu::current() {
        match lcu.patch(SETTINGS, &json!({ "General": { "WindowMode": 2 } })).await {
            Ok(_) => {
                done = true;
                // Writes the change to disk now rather than when the client next saves.
                if let Err(e) = lcu.post("/lol-game-settings/v1/save", &json!({})).await {
                    log::info!("window mode saved later by the client: {e:#}");
                }
            }
            Err(e) => last_err = Some(e),
        }
    }
    if !crate::win::game_running() {
        if let Some(path) = cfg_path() {
            match std::fs::read_to_string(&path).map(|t| with_borderless(&t)).and_then(|t| std::fs::write(&path, t)) {
                Ok(()) => done = true,
                Err(e) => last_err = Some(e.into()),
            }
        }
    }
    if !done {
        return Err(last_err.unwrap_or_else(|| anyhow::anyhow!("League was not found")));
    }
    log::info!("League window mode set to borderless");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_window_mode_from_game_cfg() {
        let cfg = "[FloatingText]\nWindowMode=9\n\n[General]\nHeight=1440\nWindowMode=0\nWidth=3440\n[HUD]\n";
        assert_eq!(from_cfg(cfg), Some(0));
        assert_eq!(name(from_cfg("[General]\r\nWindowMode=2\r\n").unwrap()), "borderless");
        assert_eq!(from_cfg("[General]\nWidth=1\n"), None);
    }

    #[test]
    fn switches_game_cfg_to_borderless() {
        let cfg = "[FloatingText]\r\nWindowMode=0\r\n[General]\r\nWindowMode=0\r\nWidth=3440\r\n";
        let out = with_borderless(cfg);
        assert_eq!(out, "[FloatingText]\r\nWindowMode=0\r\n[General]\r\nWindowMode=2\r\nWidth=3440\r\n");
        assert_eq!(from_cfg(&out), Some(2));
    }
}
