//! Optional Riot Games API key (developer.riotgames.com), for what the League client hides:
//! other players' ranked losses. Each player costs two requests: their Riot ID to this key's
//! PUUID (Riot encrypts PUUIDs per key, so the client's PUUIDs do not work here; cached), then
//! their ranked entries.
//!
//! The key is stored encrypted for the Windows user (DPAPI) in %APPDATA%\Mattofessor\riot-key.bin
//! and never sent to the UI. Requests stay under the personal / development key limits
//! (20 per second, 100 per 2 minutes).

use crate::model::{Rank, Ranked, TierDiv};
use anyhow::{anyhow, Result};
use parking_lot::{Mutex, RwLock};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum KeyStatus {
    /// No key set.
    None,
    /// Set; not checked yet.
    Unchecked,
    Ok,
    /// Rejected by Riot (wrong, or a development key older than 24 hours).
    Invalid,
    /// Riot could not be reached.
    Unreachable,
}

struct State {
    key: Option<String>,
    status: KeyStatus,
    /// Riot ID (lowercase) -> this key's PUUID.
    puuids: BTreeMap<String, String>,
}

static STATE: RwLock<State> = RwLock::new(State { key: None, status: KeyStatus::None, puuids: BTreeMap::new() });
static SENT: Mutex<VecDeque<Instant>> = Mutex::new(VecDeque::new());

fn key_file() -> std::path::PathBuf {
    crate::paths::config_dir().join("riot-key.bin")
}

mod dpapi {
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB};

    fn run(data: &[u8], protect: bool) -> Option<Vec<u8>> {
        let input = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 };
        let mut out = CRYPT_INTEGER_BLOB::default();
        unsafe {
            let ok = if protect {
                CryptProtectData(&input, windows::core::w!("Mattofessor Riot API key"), None, None, None, 0, &mut out)
            } else {
                CryptUnprotectData(&input, None, None, None, None, 0, &mut out)
            };
            ok.ok()?;
            let bytes = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
            let _ = LocalFree(Some(HLOCAL(out.pbData as *mut _)));
            Some(bytes)
        }
    }

    pub fn protect(data: &[u8]) -> Option<Vec<u8>> {
        run(data, true)
    }

    pub fn unprotect(data: &[u8]) -> Option<Vec<u8>> {
        run(data, false)
    }
}

/// Read the stored key at startup.
pub fn load() {
    let key = std::fs::read(key_file()).ok().and_then(|b| dpapi::unprotect(&b)).and_then(|b| String::from_utf8(b).ok());
    let mut s = STATE.write();
    s.status = if key.is_some() { KeyStatus::Unchecked } else { KeyStatus::None };
    s.key = key;
}

pub fn status() -> KeyStatus {
    STATE.read().status
}

fn set_status(st: KeyStatus) {
    let changed = {
        let mut s = STATE.write();
        let changed = s.status != st;
        s.status = st;
        changed
    };
    if changed {
        if let Some(core) = crate::core::get() {
            core.riot_status_changed();
        }
    }
}

/// Save (or with an empty string, remove) the key, then check it against Riot.
pub async fn set_key(key: &str, region: Option<&str>) -> Result<KeyStatus> {
    let key = key.trim();
    if key.is_empty() {
        let _ = std::fs::remove_file(key_file());
        let mut s = STATE.write();
        *s = State { key: None, status: KeyStatus::None, puuids: BTreeMap::new() };
        return Ok(KeyStatus::None);
    }
    let blob = dpapi::protect(key.as_bytes()).ok_or_else(|| anyhow!("Could not encrypt the key"))?;
    std::fs::write(key_file(), blob)?;
    {
        let mut s = STATE.write();
        *s = State { key: Some(key.to_string()), status: KeyStatus::Unchecked, puuids: BTreeMap::new() };
    }
    Ok(check(region).await)
}

/// League client region ("NA", "EUW"...) to the API's platform host.
pub fn platform(region: &str) -> Option<&'static str> {
    Some(match region.to_uppercase().as_str() {
        "NA" | "NA1" => "na1",
        "EUW" | "EUW1" => "euw1",
        "EUNE" | "EUN1" => "eun1",
        "KR" => "kr",
        "JP" | "JP1" => "jp1",
        "BR" | "BR1" => "br1",
        "LAN" | "LA1" => "la1",
        "LAS" | "LA2" => "la2",
        "OCE" | "OC1" => "oc1",
        "TR" | "TR1" => "tr1",
        "RU" => "ru",
        "ME" | "ME1" => "me1",
        "PH" | "PH2" => "ph2",
        "SG" | "SG2" => "sg2",
        "TH" | "TH2" => "th2",
        "TW" | "TW2" => "tw2",
        "VN" | "VN2" => "vn2",
        _ => return None,
    })
}

/// Account lookups work from any of these clusters; use the nearest.
fn account_cluster(platform: &str) -> &'static str {
    match platform {
        "na1" | "br1" | "la1" | "la2" => "americas",
        "kr" | "jp1" | "oc1" | "ph2" | "sg2" | "th2" | "tw2" | "vn2" => "asia",
        _ => "europe",
    }
}

fn client() -> &'static reqwest::Client {
    static C: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    C.get_or_init(|| reqwest::Client::builder().timeout(Duration::from_secs(10)).build().expect("http client"))
}

/// Wait until a request fits in 20/s and 100/120 s.
async fn throttle() {
    loop {
        let wait = {
            let mut q = SENT.lock();
            let now = Instant::now();
            while q.front().is_some_and(|t| now.duration_since(*t) > Duration::from_secs(120)) {
                q.pop_front();
            }
            let last_second = q.iter().filter(|t| now.duration_since(**t) < Duration::from_secs(1)).count();
            if q.len() < 100 && last_second < 20 {
                q.push_back(now);
                None
            } else if q.len() >= 100 {
                Some(Duration::from_secs(120).saturating_sub(now.duration_since(q[0])) + Duration::from_millis(50))
            } else {
                Some(Duration::from_millis(100))
            }
        };
        match wait {
            None => return,
            Some(d) => tokio::time::sleep(d).await,
        }
    }
}

async fn get(url: &str) -> Result<Option<Value>> {
    let key = STATE.read().key.clone().ok_or_else(|| anyhow!("no Riot API key"))?;
    for _ in 0..3 {
        throttle().await;
        let resp = match client().get(url).header("X-Riot-Token", &key).send().await {
            Ok(r) => r,
            Err(e) => {
                set_status(KeyStatus::Unreachable);
                return Err(e.into());
            }
        };
        match resp.status().as_u16() {
            200 => {
                set_status(KeyStatus::Ok);
                return Ok(Some(resp.json().await?));
            }
            404 => {
                set_status(KeyStatus::Ok);
                return Ok(None);
            }
            401 | 403 => {
                set_status(KeyStatus::Invalid);
                return Err(anyhow!("Riot API key rejected"));
            }
            429 => {
                let secs = resp.headers().get("retry-after").and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok()).unwrap_or(2u64);
                tokio::time::sleep(Duration::from_secs(secs.min(30))).await;
            }
            s => return Err(anyhow!("Riot API returned {s}")),
        }
    }
    Err(anyhow!("Riot API rate limit"))
}

/// Ask Riot whether the key works (one cheap request).
pub async fn check(region: Option<&str>) -> KeyStatus {
    let Some(platform) = region.and_then(platform) else { return status() };
    if STATE.read().key.is_none() {
        return KeyStatus::None;
    }
    let _ = get(&format!("https://{platform}.api.riotgames.com/lol/status/v4/platform-data")).await;
    status()
}

pub fn enabled() -> bool {
    matches!(status(), KeyStatus::Ok | KeyStatus::Unchecked | KeyStatus::Unreachable)
}

async fn puuid_for(platform: &str, game_name: &str, tag: &str) -> Result<Option<String>> {
    let id = format!("{game_name}#{tag}").to_lowercase();
    if let Some(p) = STATE.read().puuids.get(&id) {
        return Ok(Some(p.clone()));
    }
    let url = format!(
        "https://{}.api.riotgames.com/riot/account/v1/accounts/by-riot-id/{}/{}",
        account_cluster(platform),
        urlencoding::encode(game_name),
        urlencoding::encode(tag)
    );
    let puuid = get(&url).await?.and_then(|v| v.get("puuid").and_then(Value::as_str).map(str::to_string));
    if let Some(p) = &puuid {
        STATE.write().puuids.insert(id, p.clone());
    }
    Ok(puuid)
}

fn rank_entry(e: &Value, queue: &str) -> Option<Rank> {
    let s = |k: &str| e.get(k).and_then(Value::as_str).unwrap_or("");
    let td = TierDiv::new(s("tier"), s("rank"))?;
    let n = |k: &str| e.get(k).and_then(Value::as_i64).unwrap_or(0);
    Some(Rank { queue: queue.into(), tier: td.tier, division: td.division, lp: n("leaguePoints") as i32, wins: n("wins") as u32, losses: n("losses") as u32 })
}

/// Apply league-v4 entries to what the client reported.
pub fn merge(ranked: &mut Ranked, entries: &Value) {
    let find = |q: &str| entries.as_array().and_then(|a| a.iter().find(|e| e.get("queueType").and_then(Value::as_str) == Some(q))).cloned();
    if let Some(r) = find("RANKED_SOLO_5x5").and_then(|e| rank_entry(&e, "solo")) {
        ranked.solo = Some(r);
    }
    if let Some(r) = find("RANKED_FLEX_SR").and_then(|e| rank_entry(&e, "flex")) {
        ranked.flex = Some(r);
    }
    ranked.exact = true;
}

/// Fill in exact ranked wins and losses when a key is set. Leaves `ranked` as it was on any failure.
pub async fn complete(ranked: &mut Ranked, game_name: &str, tag: &str, region: Option<&str>) {
    if !enabled() || game_name.is_empty() || tag.is_empty() {
        return;
    }
    let Some(platform) = region.and_then(platform) else { return };
    let result: Result<()> = async {
        let Some(puuid) = puuid_for(platform, game_name, tag).await? else { return Ok(()) };
        if let Some(entries) = get(&format!("https://{platform}.api.riotgames.com/lol/league/v4/entries/by-puuid/{puuid}")).await? {
            merge(ranked, &entries);
        }
        Ok(())
    }
    .await;
    if let Err(e) = result {
        log::warn!("Riot API for {game_name}#{tag}: {e:#}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn merges_league_entries() {
        let mut r = Ranked { solo: Some(Rank { queue: "solo".into(), tier: "GOLD".into(), division: "I".into(), lp: 2, wins: 69, losses: 0 }), ..Default::default() };
        merge(
            &mut r,
            &json!([
                {"queueType": "RANKED_SOLO_5x5", "tier": "GOLD", "rank": "I", "leaguePoints": 2, "wins": 69, "losses": 61},
                {"queueType": "RANKED_FLEX_SR", "tier": "SILVER", "rank": "II", "leaguePoints": 40, "wins": 3, "losses": 5},
                {"queueType": "CHERRY", "wins": 1, "losses": 1}
            ]),
        );
        assert!(r.exact);
        assert_eq!(r.solo.as_ref().unwrap().losses, 61);
        assert_eq!(r.flex.as_ref().unwrap().tier, "SILVER");
    }

    #[test]
    fn regions() {
        assert_eq!(platform("NA"), Some("na1"));
        assert_eq!(platform("euw"), Some("euw1"));
        assert_eq!(account_cluster("kr"), "asia");
        assert_eq!(account_cluster("tr1"), "europe");
        assert!(platform("TENCENT").is_none());
    }

    #[test]
    fn dpapi_roundtrip() {
        let blob = dpapi::protect(b"RGAPI-test").unwrap();
        assert_ne!(blob, b"RGAPI-test");
        assert_eq!(dpapi::unprotect(&blob).unwrap(), b"RGAPI-test");
    }
}
