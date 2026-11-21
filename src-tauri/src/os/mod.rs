//! What the app needs from the operating system: running processes, other apps' windows, its
//! own in-game windows over the game, and what the screen shows. One backend per system with the
//! same functions (`windows.rs`, `macos.rs`); what both share is here.
//!
//! Screen coordinates (`Rect`) are the system's own: physical pixels on Windows, points on macOS
//! (both from the top left of the main display). `screen_scale` says how many of them one CSS
//! pixel takes there.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "macos")]
pub use macos::*;
#[cfg(windows)]
pub use windows::*;

use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct ProcInfo {
    pub pid: u32,
    pub name: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// A screen capture: RGBA rows top to bottom. It can have more pixels than the captured `Rect`
/// has units (a Retina display has two pixels per point).
pub struct Image {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

/// A window of another app (the game, the League client): its id on this system.
pub type WindowId = isize;

/// Executable names of the League client (the client's own window, and its lockfile's folder).
#[cfg(windows)]
pub const CLIENT_EXES: [&str; 2] = ["LeagueClientUx.exe", "LeagueClient.exe"];
#[cfg(target_os = "macos")]
pub const CLIENT_EXES: [&str; 2] = ["LeagueClientUx", "LeagueClient"];

#[cfg(windows)]
const GAME_EXE: &str = "League of Legends.exe";
/// `League of Legends.app/Contents/LoL/Game/LeagueofLegends.app/Contents/MacOS/LeagueofLegends`.
#[cfg(target_os = "macos")]
const GAME_EXE: &str = "LeagueofLegends";

pub fn find_process(procs: &[ProcInfo], name: &str) -> Option<u32> {
    procs.iter().find(|p| p.name.eq_ignore_ascii_case(name)).map(|p| p.pid)
}

/// The game's executable name (`MATTOFESSOR_GAME_EXE` overrides it for the mock game, so
/// other League tools on the computer do not react to tests).
pub fn game_exe() -> String {
    std::env::var("MATTOFESSOR_GAME_EXE").unwrap_or_else(|_| GAME_EXE.into())
}

pub fn game_running() -> bool {
    find_process(&processes(), &game_exe()).is_some()
}

/// Full path of the running League client's executable.
pub fn client_exe() -> Option<PathBuf> {
    let procs = processes();
    CLIENT_EXES.iter().filter_map(|n| find_process(&procs, n)).find_map(process_path)
}

/// Full path of the running game's executable.
pub fn game_exe_path() -> Option<PathBuf> {
    find_process(&processes(), &game_exe()).and_then(process_path)
}

/// The League client's window, while it runs.
pub fn client_window() -> Option<WindowId> {
    let procs = processes();
    CLIENT_EXES.iter().filter_map(|n| find_process(&procs, n)).find_map(main_window_of)
}

/// The game window, when League's game process is running.
pub fn game_window() -> Option<WindowId> {
    let pid = find_process(&processes(), &game_exe())?;
    main_window_of(pid)
}

/// The nearest folder above `exe` holding `name`. League keeps its files a different number of
/// folders above its executables on each system: next to `LeagueClientUx.exe` on Windows, four
/// folders up from the client inside its app bundle on macOS.
pub fn folder_above_with(exe: &Path, name: &str) -> Option<PathBuf> {
    exe.ancestors().skip(1).take(8).find(|d| d.join(name).exists()).map(Path::to_path_buf)
}

/// The game's settings file, under the League install folder.
pub const GAME_CFG: &str = "Config/game.cfg";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_league_folders_above_executables() {
        let root = std::env::temp_dir().join(format!("mf-os-{}", std::process::id()));
        let lol = root.join("League of Legends.app/Contents/LoL");
        let client = lol.join("League of Legends.app/Contents/MacOS");
        let game = lol.join("Game/LeagueofLegends.app/Contents/MacOS");
        std::fs::create_dir_all(&client).unwrap();
        std::fs::create_dir_all(&game).unwrap();
        std::fs::create_dir_all(lol.join("Config")).unwrap();
        std::fs::write(lol.join(GAME_CFG), "").unwrap();
        std::fs::write(lol.join("lockfile"), "x").unwrap();
        assert_eq!(folder_above_with(&client.join("LeagueClientUx"), "lockfile"), Some(lol.clone()));
        assert_eq!(folder_above_with(&game.join("LeagueofLegends"), GAME_CFG), Some(lol.clone()));
        // Windows: the lockfile sits next to the client, Config one folder above the game.
        let win = root.join("Riot Games/League of Legends");
        std::fs::create_dir_all(win.join("Game")).unwrap();
        std::fs::create_dir_all(win.join("Config")).unwrap();
        std::fs::write(win.join(GAME_CFG), "").unwrap();
        std::fs::write(win.join("lockfile"), "x").unwrap();
        assert_eq!(folder_above_with(&win.join("LeagueClientUx.exe"), "lockfile"), Some(win.clone()));
        assert_eq!(folder_above_with(&win.join("Game/League of Legends.exe"), GAME_CFG), Some(win.clone()));
        assert_eq!(folder_above_with(&win.join("LeagueClientUx.exe"), "nothing"), None);
        let _ = std::fs::remove_dir_all(root);
    }
}
