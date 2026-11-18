//! Data Dragon: Riot's public CDN of champion, item, summoner spell and rune data.
//! A slim combined copy per patch and language is cached in %APPDATA%\Mattofessor\dd.
//! Item roles (the shop's Fighter, Marksman, ... filter) come from CommunityDragon's copy of the
//! game data, which Data Dragon leaves out.

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
    /// Sold in Hextech ARAM under this id (see `hextech_id`).
    #[serde(default)]
    pub aram: bool,
    /// A finished item in Hextech ARAM: exactly what its players finish (AramHex's item data).
    #[serde(default)]
    pub aram_legendary: bool,
    /// The shop's roles for it (`ROLES`), none for many components and consumables.
    #[serde(default)]
    pub classes: Vec<String>,
    /// Data Dragon's description in the language: its stats and effects, with the game's markup
    /// (`<stats>`, `<passive>`...).
    #[serde(default)]
    pub desc: String,
}

/// The shop's item roles: bits of the game data's `mItemAttributes`.
const ROLES: [(u64, &str); 6] = [(1, "Fighter"), (2, "Marksman"), (4, "Assassin"), (8, "Tank"), (16, "Mage"), (32, "Support")];

/// Item id to its roles, from CommunityDragon's `items.cdtb.bin.json`.
fn item_roles(bin: &Value) -> Roles {
    let Some(m) = bin.as_object() else { return HashMap::new() };
    m.values()
        .filter_map(|v| {
            let id = v.get("itemID")?.as_i64()? as i32;
            let bits: u64 = v.get("mItemAttributes")?.as_array()?.iter().filter_map(Value::as_u64).fold(0, |a, b| a | b);
            let roles: Vec<String> = ROLES.iter().filter(|(bit, _)| bits & bit != 0).map(|(_, r)| r.to_string()).collect();
            (!roles.is_empty()).then_some((id, roles))
        })
        .collect()
}

/// Item id to its roles.
type Roles = HashMap<i32, Vec<String>>;

static ROLES_CACHE: tokio::sync::Mutex<Option<(String, Arc<Roles>)>> = tokio::sync::Mutex::const_new(None);

/// The patch's item roles, from memory, disk or CommunityDragon (its patch folder, else
/// "latest" while a new patch is not up yet). The game data is about 16 MB, so only the roles
/// are kept, once per patch for both languages.
async fn roles(version: &str) -> Result<Arc<Roles>> {
    let mut cache = ROLES_CACHE.lock().await;
    if let Some((v, r)) = cache.as_ref().filter(|(v, _)| v == version) {
        log::debug!("item roles for {v} from memory");
        return Ok(r.clone());
    }
    let file = crate::paths::config_dir().join("dd").join(format!("{version}-roles.json"));
    let roles = match std::fs::read_to_string(&file).ok().and_then(|t| serde_json::from_str::<Roles>(&t).ok()) {
        Some(r) => r,
        None => {
            let patch = version.split('.').take(2).collect::<Vec<_>>().join(".");
            let mut last = anyhow!("no game data");
            let mut found = None;
            for folder in [patch.as_str(), "latest"] {
                let url = format!("https://raw.communitydragon.org/{folder}/game/items.cdtb.bin.json");
                let got = async {
                    let r = crate::net::http().get(&url).timeout(Duration::from_secs(120)).send().await.with_context(|| format!("GET {url}"))?;
                    if !r.status().is_success() {
                        return Err(anyhow!("{url}: {}", r.status()));
                    }
                    Ok::<_, anyhow::Error>(item_roles(&r.json::<Value>().await?))
                }
                .await;
                match got {
                    Ok(r) if !r.is_empty() => {
                        found = Some(r);
                        break;
                    }
                    Ok(_) => last = anyhow!("{url}: no item roles"),
                    Err(e) => last = e,
                }
            }
            let r = found.ok_or(last)?;
            if let Ok(t) = serde_json::to_string(&r) {
                let _ = std::fs::write(&file, t);
            }
            r
        }
    };
    let roles = Arc::new(roles);
    *cache = Some((version.to_string(), roles.clone()));
    Ok(roles)
}

/// Ids Hextech ARAM games record: normal ids, and its own items (12xxxx, such as Rite of Ruin and
/// its Hubris). Data Dragon's other map-12 ids are twins for other modes (77xxxx regular ARAM,
/// 22xxxx Arena) that its games never record.
fn hextech_id(id: i32) -> bool {
    id < 10000 || (120000..130000).contains(&id)
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
    /// Rune stat shards' names (5001 to 5013), from CommunityDragon's perks in the language.
    #[serde(default)]
    pub shards: BTreeMap<i32, String>,
}

const TIMEOUT: Duration = Duration::from_secs(20);

/// A language id (`lang`) or Data Dragon locale to a Data Dragon locale.
pub fn dd_locale(lang: &str) -> &'static str {
    crate::lang::get(lang).dd
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
        let v: Value = crate::net::http().get(format!("{CDN}/api/versions.json")).timeout(TIMEOUT).send().await?.json().await?;
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
    let r = crate::net::http().get(url).timeout(TIMEOUT).send().await.with_context(|| format!("GET {url}"))?;
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
                // zh_CN (only) puts the epithet in "name" and the champion's name in "title".
                let (name, alias) = if locale == "zh_CN" { (s(c, "title"), s(c, "name")) } else { (s(c, "name"), s(c, "title")) };
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
            // Upgraded boots: tier 2, which now build into tier 3, and tier 3 itself.
            let boots = tags.contains(&"Boots") && gold >= 900 && purchasable && id < 10000;
            let legendary = into_empty && gold >= 2200 && purchasable && on_rift && !consumable && !boots && id < 10000;
            let aram = purchasable && hextech_id(id) && it.pointer("/maps/12").and_then(Value::as_bool).unwrap_or(false);
            // Finished: nothing to buy after it (an item it only upgrades into is not sold).
            let upgrades_only = it.get("into").and_then(Value::as_array).is_none_or(|a| {
                a.iter().filter_map(Value::as_str).all(|i| m.get(i).and_then(|x| x.pointer("/gold/purchasable")).and_then(Value::as_bool) != Some(true))
            });
            let aram_legendary = aram && upgrades_only && gold >= 2200 && !consumable && !boots;
            item_map.insert(id, Item { name: s(it, "name"), gold, boots, legendary, aram, aram_legendary, classes: Vec::new(), desc: s(it, "description") });
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
    StaticData { version: version.into(), locale: locale.into(), champions, items: item_map, spells: spell_map, trees, shards: BTreeMap::new() }
}

static CACHE: Mutex<Option<HashMap<String, Arc<StaticData>>>> = Mutex::new(None);
static CLASSES: Mutex<Option<Arc<HashMap<i32, String>>>> = Mutex::new(None);

/// Champion id to its kind for the style tag (`analysis::class_of`), from the champion data
/// loaded last (the tags do not depend on the language); empty until any has loaded.
pub fn classes() -> Arc<HashMap<i32, String>> {
    CLASSES.lock().clone().unwrap_or_default()
}

/// Rune stat shards' names in a language: CommunityDragon's perks with ids 5000 to 5999.
async fn shard_names(lang: &str) -> Result<BTreeMap<i32, String>> {
    let url = format!("https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/{}/v1/perks.json", crate::lang::get(lang).cdragon);
    let v = fetch_json(&url).await?;
    Ok(v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|p| Some((p.get("id")?.as_i64()? as i32, p.get("name")?.as_str()?.to_string())))
                .filter(|(id, _)| (5000..6000).contains(id))
                .collect()
        })
        .unwrap_or_default())
}

/// Static data for a language (`lang` id), from memory, disk or the CDN.
pub async fn get(lang: &str) -> Result<Arc<StaticData>> {
    let locale = dd_locale(lang);
    let version = latest_version().await?;
    let key = format!("{version}/{locale}");
    if let Some(d) = CACHE.lock().as_ref().and_then(|m| m.get(&key).cloned()) {
        return Ok(d);
    }
    let dir = crate::paths::config_dir().join("dd");
    let file = dir.join(format!("{version}-{locale}-7.json"));
    let data = match std::fs::read_to_string(&file).ok().and_then(|t| serde_json::from_str::<StaticData>(&t).ok()) {
        Some(d) => d,
        None => {
            let base = format!("{CDN}/cdn/{version}/data/{locale}");
            let urls = ["champion", "item", "summoner", "runesReforged"].map(|f| format!("{base}/{f}.json"));
            let _ = std::fs::create_dir_all(&dir);
            let (c, i, s, r) = tokio::try_join!(fetch_json(&urls[0]), fetch_json(&urls[1]), fetch_json(&urls[2]), fetch_json(&urls[3]))?;
            let mut d = parse(&version, locale, &c, &i, &s, &r);
            match shard_names(lang).await {
                Ok(s) => d.shards = s,
                Err(e) => log::warn!("shard names: {e:#}"),
            }
            match roles(&version).await {
                Ok(roles) => {
                    for (id, item) in d.items.iter_mut() {
                        item.classes = roles.get(id).cloned().unwrap_or_default();
                    }
                    if let Ok(t) = serde_json::to_string(&d) {
                        let _ = std::fs::write(&file, t);
                    }
                }
                // Used without roles this time and not kept, so the next start tries again.
                Err(e) => log::warn!("item roles: {e:#}"),
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
    fn reads_item_roles() {
        let bin = json!({
            "Items/3075": {"itemID": 3075, "mItemAttributes": [8, 32]},
            "Items/3031": {"itemID": 3031, "mItemAttributes": [2]},
            "Items/1001": {"itemID": 1001},
            "Items/Spells/X": {"__type": "SpellObject"}
        });
        let r = item_roles(&bin);
        assert_eq!(r[&3075], ["Tank", "Support"]);
        assert_eq!(r[&3031], ["Marksman"]);
        assert!(!r.contains_key(&1001));
    }

    #[test]
    fn classifies_items() {
        let items = json!({"data": {
            "3071": {"name": "Black Cleaver", "gold": {"total": 3000, "purchasable": true}, "maps": {"11": true}, "tags": ["Damage"]},
            "3047": {"name": "Plated Steelcaps", "gold": {"total": 1200, "purchasable": true}, "maps": {"11": true}, "tags": ["Boots"], "into": ["3174"]},
            "1001": {"name": "Boots", "gold": {"total": 300, "purchasable": true}, "maps": {"11": true}, "tags": ["Boots"], "into": ["3047"]},
            "3044": {"name": "Phage", "gold": {"total": 1100, "purchasable": true}, "maps": {"11": true}, "into": ["3071"]},
            "2003": {"name": "Health Potion", "gold": {"total": 50, "purchasable": true}, "maps": {"11": true}, "tags": ["Consumable"]},
            "223071": {"name": "Black Cleaver", "gold": {"total": 3000, "purchasable": true}, "maps": {"11": false, "30": true}},
            "123430": {"name": "Rite of Ruin", "gold": {"total": 3000, "purchasable": true}, "maps": {"12": true}},
            "773123": {"name": "Executioner's Calling", "gold": {"total": 1900, "purchasable": true}, "maps": {"12": true}},
            "2526": {"name": "Whispering Circlet", "gold": {"total": 2250, "purchasable": true}, "maps": {"11": true, "12": true}, "into": ["2530"]},
            "2530": {"name": "Diadem of Songs", "gold": {"total": 2250, "purchasable": false}, "maps": {"11": true, "12": true}},
            "3004": {"name": "Manamune", "gold": {"total": 2900, "purchasable": true}, "maps": {"12": true}, "into": ["3003"]},
            "3003": {"name": "Archangel's Staff", "gold": {"total": 2900, "purchasable": true}, "maps": {"12": true}}
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
        // Hextech ARAM: its own items count, other modes' twins and upgrades do not.
        assert!(d.items[&123430].aram_legendary, "an ARAM-only item");
        assert!(!d.items[&773123].aram, "a regular ARAM twin");
        assert!(d.items[&2526].aram_legendary, "builds only into an upgrade nobody buys");
        assert!(!d.items[&2530].aram, "an upgrade is not sold");
        assert!(!d.items[&3004].aram_legendary, "builds into an item that is sold");
        assert!(!d.items[&3071].aram && !d.items[&3047].aram);
    }
}
