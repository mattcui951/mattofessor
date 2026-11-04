//! Where the app keeps its data.

use std::path::PathBuf;

/// %APPDATA%\Mattofessor (settings, cache database, log). Debug builds use Mattofessor-dev, so a
/// test run never opens the installed app's files (two programs writing one SQLite database's
/// side files damages it).
pub fn config_dir() -> PathBuf {
    let name = if cfg!(debug_assertions) { "Mattofessor-dev" } else { "Mattofessor" };
    let dir = dirs::config_dir().unwrap_or_else(std::env::temp_dir).join(name);
    let _ = std::fs::create_dir_all(&dir);
    dir
}
