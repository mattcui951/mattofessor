//! Debug builds only: a read-mostly JSON bridge on http://127.0.0.1:17431 so the UI can run in
//! a plain browser tab against the running app (see `src/dev/browser-preview.ts`).

use crate::commands;
use crate::core::Core;
use crate::{builds, dd};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const PORT: u16 = 17431;

pub fn start(core: Arc<Core>) {
    tauri::async_runtime::spawn(async move {
        let listener = match tokio::net::TcpListener::bind(("127.0.0.1", PORT)).await {
            Ok(l) => l,
            Err(e) => return log::warn!("dev bridge: {e}"),
        };
        log::info!("dev bridge on http://127.0.0.1:{PORT}");
        while let Ok((stream, _)) = listener.accept().await {
            tauri::async_runtime::spawn(handle(stream, core.clone()));
        }
    });
}

fn query(q: &str) -> HashMap<String, String> {
    q.split('&')
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (k.to_string(), urlencoding::decode(v).map(|s| s.into_owned()).unwrap_or_default()))
        .collect()
}

async fn route(core: &Core, path: &str, p: &HashMap<String, String>) -> Result<Value, String> {
    let arg = |k: &str| p.get(k).cloned().unwrap_or_default();
    let num = |k: &str| arg(k).parse::<i64>().unwrap_or(0);
    let e = |e: anyhow::Error| format!("{e:#}");
    match path {
        "/bootstrap" => Ok(commands::boot_value(core)),
        "/live_state" => Ok(commands::live_value(core)),
        "/static_data" => dd::get(&arg("lang")).await.map(|d| json!(*d)).map_err(e),
        "/profile" => commands::load_profile(core, Some(arg("puuid")).filter(|s| !s.is_empty())).await.map(|v| json!(v)).map_err(e),
        "/search" => {
            let lcu = crate::lcu::require().map_err(e)?;
            crate::players::lookup(&lcu, &arg("riotId")).await.map(|v| json!(v)).map_err(e)
        }
        "/game" => crate::players::game(num("gameId") as u64).await.map_err(e),
        "/scores" => {
            let g = crate::players::game(num("gameId") as u64).await.map_err(e)?;
            Ok(json!(crate::score::score_game(&g)))
        }
        "/profile_quick" => commands::quick_profile(core, &arg("puuid")).await.map(|v| json!(v)).map_err(e),
        "/profile_saved" => Ok(json!(commands::saved_profile(core, Some(arg("puuid")).filter(|s| !s.is_empty())))),
        "/player_scores" => {
            let ids: Vec<u64> = arg("gameIds").split(',').filter_map(|s| s.parse().ok()).collect();
            crate::commands::player_scores(arg("puuid"), ids).await.map(|v| json!(v))
        }
        "/rift_tiers" => crate::opgg::tiers().await.map(|v| json!(*v)).map_err(e),
        "/aram_overview" => {
            let b = arg("bracket");
            let (info, data) = crate::aramhex::bracket(&b).await.map_err(e)?;
            Ok(json!(crate::aramhex::overview(&info, &b, &data)))
        }
        "/aram_champion" => {
            let b = arg("bracket");
            let (info, data) = crate::aramhex::bracket(&b).await.map_err(e)?;
            Ok(json!(crate::aramhex::champion(&info, &b, &data, num("championId") as i32)))
        }
        "/aram_picks" => crate::aramhex::champion_picks(num("championId") as i32, &arg("bracket")).await.map(|v| json!(v)).map_err(e),
        "/card_offer" => Ok(json!(crate::cards::current())),
        "/augment_names" => crate::augments::all(&arg("lang")).await.map(|v| json!(*v)).map_err(e),
        "/build" => builds::build(num("championId") as i32, &arg("role"), &arg("mode")).await.map(|v| json!(v)).map_err(e),
        "/import_build" => builds::import(num("championId") as i32, &arg("role"), &arg("mode"), arg("runes") == "true", arg("items") == "true")
            .await
            .map(|v| json!(v))
            .map_err(e),
        "/set_riot_key" => crate::riot::set_key(&arg("key"), core.region().as_deref()).await.map(|v| json!(v)).map_err(e),
        "/check_update" => Ok(json!(crate::update::check(core).await)),
        "/toggle_overlay" => {
            core.toggle_overlay();
            Ok(Value::Null)
        }
        "/toggle_spells" => {
            core.toggle_spells();
            Ok(Value::Null)
        }
        // The card win rates at a given place (physical pixels) for card middles `xs` (fractions
        // of the width), to check where they land without a game.
        "/show_cards" => {
            let r = crate::win::Rect { x: num("x") as i32, y: num("y") as i32, w: num("w") as i32, h: num("h") as i32 };
            let xs: Vec<f64> = arg("xs").split(',').filter_map(|s| s.parse().ok()).collect();
            let cards = xs
                .iter()
                .enumerate()
                .map(|(i, x)| crate::cards::OfferCard {
                    id: i as i32 + 1,
                    x: *x,
                    stat: Some(crate::aramhex::AugRow { id: i as i32 + 1, games: 1000, win_rate: 0.5, smoothed: 0.5, delta: 0.0, low_sample: false }),
                    best: i == 0,
                })
                .collect();
            core.show_cards(r, &crate::cards::Offer { champion_id: 1, base: Some(0.5), cards });
            Ok(Value::Null)
        }
        // A line from a web view (layout checks in debug builds), into the log.
        "/report" => {
            log::info!("report: {}", arg("text"));
            Ok(Value::Null)
        }
        "/hide_cards" => {
            core.hide_cards();
            Ok(Value::Null)
        }
        _ => Err(format!("unknown {path}")),
    }
}

async fn handle(mut stream: tokio::net::TcpStream, core: Arc<Core>) {
    let mut buf = vec![0u8; 16 * 1024];
    let mut n = 0;
    while n < buf.len() {
        match stream.read(&mut buf[n..]).await {
            Ok(0) | Err(_) => break,
            Ok(k) => {
                n += k;
                if buf[..n].windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
        }
    }
    let head = String::from_utf8_lossy(&buf[..n]);
    let target = head.lines().next().and_then(|l| l.split_whitespace().nth(1)).unwrap_or("/");
    let (path, q) = target.split_once('?').unwrap_or((target, ""));
    let (status, body) = match route(&core, path, &query(q)).await {
        Ok(v) => ("200 OK", v.to_string()),
        Err(msg) => ("500 Internal Server Error", json!({ "error": msg }).to_string()),
    };
    let resp = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(resp.as_bytes()).await;
}
