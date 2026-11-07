//! Data Dragon: Riot's public CDN of champion, item, summoner spell and rune data.
//! A slim combined copy per patch and language is cached in %APPDATA%\Mattofessor\dd.

use anyhow::{anyhow, Context as _, Result};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Duration;

const CDN: &str = "https://ddragon.leagueoflegends.com";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Champ {
    pub key: String,
    pub name: String,
    /// The other well-known name: the epithet in Chinese (探险家 for Ezreal), the title in English.
    pub alias: String,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub name: String,
    pub gold: u32,
    pub boots: bool,
    /// A finished, full-price item (what a build is made of).
    pub legendary: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Spell {
    pub key: String,
    pub name: String,
    pub cooldown: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rune {
    pub id: i32,
    pub name: String,
    pub icon: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tree {
    pub id: i32,
    pub name: String,
    pub icon: String,
    pub slots: Vec<Vec<Rune>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StaticData {
    pub version: String,
    pub locale: String,
    pub champions: BTreeMap<i32, Champ>,
    pub items: BTreeMap<i32, Item>,
    pub spells: BTreeMap<i32, Spell>,
    pub trees: Vec<Tree>,
}

fn client() -> reqwest::Client {
    reqwest::Client::builder().timeout(Duration::from_secs(20)).build().expect("http")
}

/// "en" / "zh" (or a Data Dragon locale) to a Data Dragon locale.
pub fn dd_locale(lang: &str) -> &'static str {
    if lang.to_lowercase().starts_with("zh") {
        "zh_CN"
    } else {
        "en_US"
    }
}

async fn latest_version() -> Result<String> {
    let st = crate::store::get();
    let now = chrono::Utc::now().timestamp();
    if let (Some(v), Some(at)) = (st.kv_get("dd_version"), st.kv_get("dd_version_at").and_then(|s| s.parse::<i64>().ok())) {
        if now - at < 6 * 3600 {
            return Ok(v);
        }
    }
    let fetched: Result<String> = async {
        let v: Value = client().get(format!("{CDN}/api/versions.json")).send().await?.json().await?;
        v.get(0).and_then(Value::as_str).map(str::to_string).ok_or_else(|| anyhow!("no versions"))
    }
    .await;
    match fetched {
        Ok(v) => {
            st.kv_set("dd_version", &v);
            st.kv_set("dd_version_at", &now.to_string());
            Ok(v)
        }
        // Offline: keep using the last patch we know.
        Err(e) => st.kv_get("dd_version").ok_or(e),
    }
}

async fn fetch_json(url: &str) -> Result<Value> {
    let r = client().get(url).send().await.with_context(|| format!("GET {url}"))?;
    if !r.status().is_success() {
        return Err(anyhow!("{url}: {}", r.status()));
    }
    Ok(r.json().await?)
}

fn parse(version: &str, locale: &str, champs: &Value, items: &Value, spells: &Value, runes: &Value) -> StaticData {
    let s = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let mut champions = BTreeMap::new();
    if let Some(m) = champs.get("data").and_then(Value::as_object) {
        for c in m.values() {
            if let Ok(id) = s(c, "key").parse::<i32>() {
                let tags = c.get("tags").and_then(Value::as_array).map(|a| a.iter().filter_map(|t| t.as_str().map(str::to_string)).collect()).unwrap_or_default();
                // zh_CN puts the epithet in "name" and the champion's name in "title".
                let (name, alias) = if locale.starts_with("zh") { (s(c, "title"), s(c, "name")) } else { (s(c, "name"), s(c, "title")) };
                champions.insert(id, Champ { key: s(c, "id"), name, alias, tags });
            }
        }
    }
    let mut item_map = BTreeMap::new();
    if let Some(m) = items.get("data").and_then(Value::as_object) {
        for (id, it) in m {
            let Ok(id) = id.parse::<i32>() else { continue };
            let gold = it.pointer("/gold/total").and_then(Value::as_u64).unwrap_or(0) as u32;
            let purchasable = it.pointer("/gold/purchasable").and_then(Value::as_bool).unwrap_or(false);
            let on_rift = it.pointer("/maps/11").and_then(Value::as_bool).unwrap_or(false);
            let into_empty = it.get("into").and_then(Value::as_array).is_none_or(|a| a.is_empty());
            let tags: Vec<&str> = it.get("tags").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
            let consumable = tags.contains(&"Consumable") || tags.contains(&"Trinket");
            let boots = tags.contains(&"Boots") && into_empty && gold >= 900 && id < 10000;
            let legendary = into_empty && gold >= 2200 && purchasable && on_rift && !consumable && !boots && id < 10000;
            item_map.insert(id, Item { name: s(it, "name"), gold, boots, legendary });
        }
    }
    let mut spell_map = BTreeMap::new();
    if let Some(m) = spells.get("data").and_then(Value::as_object) {
        for sp in m.values() {
            if let Ok(id) = s(sp, "key").parse::<i32>() {
                let cooldown = sp.get("cooldown").and_then(Value::as_array).and_then(|a| a.first()).and_then(Value::as_f64).unwrap_or(0.0);
                spell_map.insert(id, Spell { key: s(sp, "id"), name: s(sp, "name"), cooldown });
            }
        }
    }
    let trees = runes
        .as_array()
        .map(|a| {
            a.iter()
                .map(|t| Tree {
                    id: t.get("id").and_then(Value::as_i64).unwrap_or(0) as i32,
                    name: s(t, "name"),
                    icon: s(t, "icon"),
                    slots: t
                        .get("slots")
                        .and_then(Value::as_array)
                        .map(|slots| {
                            slots
                                .iter()
                                .map(|sl| {
                                    sl.get("runes")
                                        .and_then(Value::as_array)
                                        .map(|rs| {
                                            rs.iter()
                                                .map(|r| Rune { id: r.get("id").and_then(Value::as_i64).unwrap_or(0) as i32, name: s(r, "name"), icon: s(r, "icon") })
                                                .collect()
                                        })
                                        .unwrap_or_default()
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default();
    StaticData { version: version.into(), locale: locale.into(), champions, items: item_map, spells: spell_map, trees }
}

static CACHE: Mutex<Option<HashMap<String, Arc<StaticData>>>> = Mutex::new(None);
static CLASSES: Mutex<Option<Arc<HashMap<i32, String>>>> = Mutex::new(None);

/// Champion id to its kind for the style tag (`analysis::class_of`), from the champion data
/// loaded last (the tags do not depend on the language); empty until any has loaded.
pub fn classes() -> Arc<HashMap<i32, String>> {
    CLASSES.lock().clone().unwrap_or_default()
}

/// Static data for a language ("en" / "zh"), from memory, disk or the CDN.
pub async fn get(lang: &str) -> Result<Arc<StaticData>> {
    let locale = dd_locale(lang);
    let version = latest_version().await?;
    let key = format!("{version}/{locale}");
    if let Some(d) = CACHE.lock().as_ref().and_then(|m| m.get(&key).cloned()) {
        return Ok(d);
    }
    let dir = crate::paths::config_dir().join("dd");
    let file = dir.join(format!("{version}-{locale}.json"));
    let data = match std::fs::read_to_string(&file).ok().and_then(|t| serde_json::from_str::<StaticData>(&t).ok()) {
        Some(d) => d,
        None => {
            let base = format!("{CDN}/cdn/{version}/data/{locale}");
            let urls = ["champion", "item", "summoner", "runesReforged"].map(|f| format!("{base}/{f}.json"));
            let (c, i, s, r) = tokio::try_join!(fetch_json(&urls[0]), fetch_json(&urls[1]), fetch_json(&urls[2]), fetch_json(&urls[3]))?;
            let d = parse(&version, locale, &c, &i, &s, &r);
            let _ = std::fs::create_dir_all(&dir);
            if let Ok(t) = serde_json::to_string(&d) {
                let _ = std::fs::write(&file, t);
            }
            d
        }
    };
    let d = Arc::new(data);
    let classes = d.champions.iter().map(|(id, c)| (*id, crate::analysis::class_of(&c.key, &c.tags))).collect();
    *CLASSES.lock() = Some(Arc::new(classes));
    CACHE.lock().get_or_insert_with(HashMap::new).insert(key, d.clone());
    Ok(d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn classifies_items() {
        let items = json!({"data": {
            "3071": {"name": "Black Cleaver", "gold": {"total": 3000, "purchasable": true}, "maps": {"11": true}, "tags": ["Damage"]},
            "3047": {"name": "Plated Steelcaps", "gold": {"total": 1200, "purchasable": true}, "maps": {"11": true}, "tags": ["Boots"]},
            "1001": {"name": "Boots", "gold": {"total": 300, "purchasable": true}, "maps": {"11": true}, "tags": ["Boots"], "into": ["3047"]},
            "3044": {"name": "Phage", "gold": {"total": 1100, "purchasable": true}, "maps": {"11": true}, "into": ["3071"]},
            "2003": {"name": "Health Potion", "gold": {"total": 50, "purchasable": true}, "maps": {"11": true}, "tags": ["Consumable"]},
            "223071": {"name": "Black Cleaver", "gold": {"total": 3000, "purchasable": true}, "maps": {"11": false, "30": true}}
        }});
        let champs = json!({"data": {"Ezreal": {"key": "81", "id": "Ezreal", "name": "探险家", "title": "伊泽瑞尔", "tags": ["Marksman"]}}});
        let zh = parse("1", "zh_CN", &champs, &json!({}), &json!({}), &json!([]));
        assert_eq!(zh.champions[&81].name, "伊泽瑞尔");
        assert_eq!(zh.champions[&81].alias, "探险家");
        let d = parse("1", "en_US", &json!({}), &items, &json!({}), &json!([]));
        assert!(d.items[&3071].legendary);
        assert!(d.items[&3047].boots && !d.items[&3047].legendary);
        assert!(!d.items[&1001].boots);
        assert!(!d.items[&3044].legendary);
        assert!(!d.items[&2003].legendary);
        assert!(!d.items[&223071].legendary);
    }
}
