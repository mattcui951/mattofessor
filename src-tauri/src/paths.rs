//! Where the app keeps its data.

use std::path::PathBuf;

/// %APPDATA%\Mattofessor (settings, cache database, log).
pub fn config_dir() -> PathBuf {
    let dir = dirs::config_dir().unwrap_or_else(std::env::temp_dir).join("Mattofessor");
    let _ = std::fs::create_dir_all(&dir);
    dir
}
