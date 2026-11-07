//! Where the app keeps its data.

use std::path::PathBuf;

/// %APPDATA%\Mattofessor (settings, cache database, log). Debug builds use Mattofessor-dev, so a
/// test run never opens the installed app's files (two programs writing one SQLite database's
/// side files damages it), or `MATTOFESSOR_DATA_DIR` when set: two test runs started from
/// different places (a sandboxed shell sees an overlay of the real %APPDATA%) keep apart that way.
pub fn config_dir() -> PathBuf {
    let dir = match std::env::var_os("MATTOFESSOR_DATA_DIR") {
        Some(d) if cfg!(debug_assertions) && !d.is_empty() => PathBuf::from(d),
        _ => {
            let name = if cfg!(debug_assertions) { "Mattofessor-dev" } else { "Mattofessor" };
            dirs::config_dir().unwrap_or_else(std::env::temp_dir).join(name)
        }
    };
    let _ = std::fs::create_dir_all(&dir);
    dir
}
