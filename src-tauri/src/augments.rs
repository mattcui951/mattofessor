//! Augment names, rarity and icons (Arena and Hextech ARAM share one list), from the public
//! mirror of Riot's game data (CommunityDragon), in English or Chinese, cached for a day.

use anyhow::{bail, Result};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

const CDRAGON: &str = "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global";
const TTL_MS: i64 = 24 * 3_600_000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Augment {
    pub name: String,
    /// "silver", "gold" or "prismatic".
    pub rarity: String,
    pub icon: String,
}

/// "en" / "zh" to the mirror's folder.
fn folder(lang: &str) -> &'static str {
    if lang.starts_with("zh") {
        "zh_cn"
    } else {
        "default"
    }
}

/// Icons live under the "default" folder whatever the language.
fn icon_url(path: &str) -> String {
    let rest = path.trim_start_matches("/lol-game-data/assets/").to_lowercase();
    if rest.is_empty() {
        String::new()
    } else {
        format!("{CDRAGON}/default/{rest}")
    }
}

pub fn parse(v: &Value) -> HashMap<i32, Augment> {
    let mut out = HashMap::new();
    for a in v.as_array().cloned().unwrap_or_default() {
        let Some(id) = a.get("id").and_then(Value::as_i64) else { continue };
        let s = |k: &str| a.get(k).and_then(Value::as_str).unwrap_or_default().to_string();
        let rarity = match s("rarity").as_str() {
            "kSilver" => "silver",
            "kGold" => "gold",
            "kPrismatic" => "prismatic",
            _ => "",
        };
        out.insert(id as i32, Augment { name: s("nameTRA"), rarity: rarity.into(), icon: icon_url(&s("augmentSmallIconPath")) });
    }
    out
}

pub type Augments = Arc<HashMap<i32, Augment>>;

static CACHE: Mutex<Option<HashMap<String, Augments>>> = Mutex::new(None);

/// Every augment in a language ("en" or "zh").
pub async fn all(lang: &str) -> Result<Augments> {
    let f = folder(lang);
    if let Some(m) = CACHE.lock().as_ref().and_then(|c| c.get(f).cloned()) {
        return Ok(m);
    }
    let key = format!("augments:{f}");
    let st = crate::store::get();
    let now = crate::players::now_ms();
    let old: Option<(i64, Value)> = st.kv_get(&key).and_then(|t| serde_json::from_str(&t).ok());
    let v = match &old {
        Some((at, v)) if now - at < TTL_MS => v.clone(),
        _ => {
            let fetched: Result<Value> = async {
                let c = reqwest::Client::builder().timeout(Duration::from_secs(30)).build()?;
                let r = c.get(format!("{CDRAGON}/{f}/v1/cherry-augments.json")).send().await?;
                if !r.status().is_success() {
                    bail!("augment list answered {}", r.status());
                }
                Ok(r.json().await?)
            }
            .await;
            match (fetched, old) {
                (Ok(v), _) => {
                    st.kv_set(&key, &serde_json::to_string(&(now, &v))?);
                    v
                }
                (Err(e), Some((_, v))) => {
                    log::warn!("augment list: {e:#}; using the last copy");
                    v
                }
                (Err(e), None) => return Err(e),
            }
        }
    };
    let m = Arc::new(parse(&v));
    CACHE.lock().get_or_insert_with(HashMap::new).insert(f.to_string(), m.clone());
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_names_rarity_and_icons() {
        let v = serde_json::json!([{"id": 1384, "nameTRA": "Donation", "rarity": "kGold",
            "augmentSmallIconPath": "/lol-game-data/assets/ASSETS/UX/Kiwi/Augments/Icons/Donation_small.png"}]);
        let m = parse(&v);
        let a = &m[&1384];
        assert_eq!((a.name.as_str(), a.rarity.as_str()), ("Donation", "gold"));
        assert_eq!(a.icon, format!("{CDRAGON}/default/assets/ux/kiwi/augments/icons/donation_small.png"));
        assert_eq!(folder("zh"), "zh_cn");
    }
}
