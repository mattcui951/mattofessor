//! App updates from the latest GitHub release: the release workflow publishes a signed
//! `latest.json` next to the installer, so checking and downloading need no GitHub account.
//! Updates are only offered; the user starts the install.

use crate::core::Core;
use parking_lot::Mutex;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tauri::Emitter;
use tauri_plugin_updater::{Update, UpdaterExt};

/// How often to look for a new version while the app runs.
const INTERVAL: Duration = Duration::from_secs(6 * 3600);
/// First check after start, once the client connection has settled.
const FIRST_CHECK: Duration = Duration::from_secs(20);

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateState {
    /// "idle", "checking", "available", "downloading", "installing" or "error".
    pub phase: &'static str,
    /// The newer version, once one was found.
    pub version: Option<String>,
    /// Download progress 0-1 (`None` while the size is unknown).
    pub progress: Option<f64>,
    pub error: Option<String>,
    /// When the last check finished (ms since epoch).
    pub checked_at: Option<i64>,
}

static STATE: Mutex<UpdateState> = Mutex::new(UpdateState { phase: "idle", version: None, progress: None, error: None, checked_at: None });
static PENDING: Mutex<Option<Update>> = Mutex::new(None);
/// The version this install was updated from, when this is the first start after an update.
static UPDATED_FROM: Mutex<Option<String>> = Mutex::new(None);

fn marker() -> std::path::PathBuf {
    crate::paths::config_dir().join("updated-from")
}

/// Call once at startup: true when the app was just restarted by an update.
pub fn take_marker() -> bool {
    let Ok(from) = std::fs::read_to_string(marker()) else { return false };
    let _ = std::fs::remove_file(marker());
    log::info!("updated from {from} to {}", env!("CARGO_PKG_VERSION"));
    *UPDATED_FROM.lock() = Some(from.trim().to_string());
    true
}

/// The previous version once after an update (for a "Updated to ..." note), then `None`.
pub fn updated_from() -> Option<String> {
    UPDATED_FROM.lock().take()
}

pub fn state() -> UpdateState {
    STATE.lock().clone()
}

fn set(core: &Core, f: impl FnOnce(&mut UpdateState)) {
    let st = {
        let mut st = STATE.lock();
        f(&mut st);
        st.clone()
    };
    let _ = core.app.emit("update", st);
}

fn busy() -> bool {
    matches!(STATE.lock().phase, "checking" | "downloading" | "installing")
}

/// Checks in the background: shortly after start, then every few hours (if enabled).
pub fn spawn(core: Arc<Core>) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK).await;
        loop {
            if core.settings().check_updates {
                check(&core).await;
            }
            tokio::time::sleep(INTERVAL).await;
        }
    });
}

/// Looks for a newer release. Errors end up in the state, not here.
pub async fn check(core: &Core) -> UpdateState {
    if busy() {
        return state();
    }
    set(core, |s| {
        s.phase = "checking";
        s.error = None;
    });
    // The installer closes this app; the marker tells the next start it was an update.
    let updater = core.app.updater_builder().on_before_exit(|| {
        let _ = std::fs::write(marker(), env!("CARGO_PKG_VERSION"));
    });
    let found = match updater.build() {
        Ok(u) => u.check().await,
        Err(e) => Err(e),
    };
    let now = Some(chrono::Utc::now().timestamp_millis());
    match found {
        Ok(Some(update)) => {
            log::info!("update available: {} -> {}", update.current_version, update.version);
            let version = update.version.clone();
            *PENDING.lock() = Some(update);
            set(core, |s| {
                s.phase = "available";
                s.version = Some(version);
                s.checked_at = now;
            });
        }
        Ok(None) => {
            *PENDING.lock() = None;
            set(core, |s| {
                s.phase = "idle";
                s.version = None;
                s.checked_at = now;
            });
        }
        Err(e) => {
            log::warn!("update check failed: {e}");
            // Keep offering an update found earlier.
            let had = PENDING.lock().is_some();
            set(core, |s| {
                s.phase = if had { "available" } else { "error" };
                s.error = Some(e.to_string());
                s.checked_at = now;
            });
        }
    }
    state()
}

/// Downloads the update, verifies its signature and runs the installer, which closes this app
/// and starts the new version when it is done.
pub async fn install(core: &Core) -> anyhow::Result<()> {
    if crate::game::in_game_phase(&core.live.lock().phase) {
        anyhow::bail!("[update-in-game] Update after your game");
    }
    if busy() {
        return Ok(());
    }
    let Some(update) = PENDING.lock().clone() else {
        anyhow::bail!("[update-none] No update to install");
    };
    set(core, |s| {
        s.phase = "downloading";
        s.progress = Some(0.0);
        s.error = None;
    });
    let mut done = 0u64;
    let mut shown = -1i64;
    let result = update
        .download_and_install(
            |chunk, total| {
                done += chunk as u64;
                let p = total.filter(|t| *t > 0).map(|t| (done as f64 / t as f64).min(1.0));
                // One event per percent.
                let pct = p.map_or(-1, |p| (p * 100.0) as i64);
                if pct != shown {
                    shown = pct;
                    set(core, |s| s.progress = p);
                }
            },
            || set(core, |s| s.phase = "installing"),
        )
        .await;
    // On Windows a successful install exits the process before this point.
    if let Err(e) = result {
        log::warn!("update install failed: {e}");
        set(core, |s| {
            s.phase = "available";
            s.progress = None;
            s.error = Some(e.to_string());
        });
        anyhow::bail!("[update-failed] Update failed: {e}");
    }
    Ok(())
}
