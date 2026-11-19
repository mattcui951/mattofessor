//! Loads what the pages ask for first, in the background soon after start, so the first visit to
//! Meta, Builds, the Hextech ARAM pages or champ select does not wait on disk or the network:
//! the OP.GG tier list, the AramHex brackets, boards and champion tables of the current patch
//! (the user's skill bracket), augment names in the UI language and the champion keywords.
//!
//! Each piece goes through its own cache, so this costs nothing when the copies are fresh and only
//! starts a background download when one is old. Failures are left to the pages to report.

use crate::core::Core;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// After the client connection and the first window have settled.
const AFTER: Duration = Duration::from_secs(8);

pub fn spawn(core: Arc<Core>) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(AFTER).await;
        let started = Instant::now();
        let bracket = core.settings().aram_bracket;
        let lang = core.language();
        let mut failed = Vec::new();
        let mut note = |what: &str, r: anyhow::Result<()>| {
            if let Err(e) = r {
                failed.push(format!("{what}: {e:#}"));
            }
        };
        note("tier list", crate::opgg::tiers().await.map(drop));
        note("augment names", crate::augments::all(&lang).await.map(drop));
        note("keywords", crate::keywords::all().await.map(drop));
        note("bracket", crate::aramhex::bracket(&bracket).await.map(drop));
        note("champion board", crate::hexstats::board("", &bracket).await.map(drop));
        note("augment board", crate::hexstats::augment_board("", &bracket).await.map(drop));
        // Any champion: the page reads the tables every champion's page shares.
        note("champion tables", crate::hexstats::champion(1, "", &bracket).await.map(drop));
        if failed.is_empty() {
            log::info!("warmed the caches in {:.1}s", started.elapsed().as_secs_f32());
        } else {
            log::info!("warmed the caches in {:.1}s; not ready: {}", started.elapsed().as_secs_f32(), failed.join("; "));
        }
    });
}
