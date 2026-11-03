//! Champion builds from OP.GG (see `opgg`), and importing them into the client.
//!
//! Summoner's Rift builds are OP.GG's ranked builds for the chosen lane; ARAM builds are its
//! ARAM builds (classic ARAM: Hextech ARAM has no public item data from OP.GG, so they are
//! labelled as such). A build has the rune page with stat shards, summoner spells, starting
//! items, boots, the three core items and the most picked options after them, and the skill
//! order.

use crate::lcu;
use crate::opgg;
use anyhow::{anyhow, Result};
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Pick<T> {
    pub value: T,
    pub games: u32,
    pub wins: u32,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RunePage {
    pub primary: i32,
    pub sub: i32,
    /// Four primary runes then two secondary.
    pub perks: Vec<i32>,
    /// Offense, flex, defense.
    pub shards: Vec<i32>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Build {
    pub champion_id: i32,
    /// "rift" or "aram".
    pub mode: String,
    /// The lane on the Rift ("top", "jungle", "mid", "adc", "support"); "" in ARAM.
    pub role: String,
    pub games: u32,
    pub wins: u32,
    /// The champion's lanes, most played first.
    pub roles: Vec<Pick<String>>,
    pub runes: Vec<Pick<RunePage>>,
    pub spells: Vec<Pick<[i32; 2]>>,
    pub starters: Vec<Pick<Vec<i32>>>,
    pub boots: Vec<Pick<i32>>,
    /// Core items first (in buy order), then the most picked 4th to 6th items.
    pub items: Vec<Pick<i32>>,
    /// How many of `items` are the core.
    pub core: u32,
    /// Skills to max, e.g. ["Q", "E", "W"].
    pub skill_max: Vec<String>,
    /// The first levels' skill order.
    pub skill_order: Vec<String>,
}

fn u(v: &Value, k: &str) -> u32 {
    v.get(k).and_then(Value::as_f64).unwrap_or(0.0).max(0.0) as u32
}

fn ids(v: &Value, k: &str) -> Vec<i32> {
    v.get(k).and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_i64).map(|x| x as i32).collect()).unwrap_or_default()
}

fn strings(v: &Value, k: &str) -> Vec<String> {
    v.get(k).and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default()
}

fn pick<T>(v: &Value, value: T) -> Pick<T> {
    Pick { value, games: u(v, "play"), wins: u(v, "win") }
}

/// A build from OP.GG's champion analysis answer.
pub fn from_analysis(champion_id: i32, mode: &str, role: &str, a: &Value) -> Option<Build> {
    let d = a.get("data")?;
    let positions: Vec<Value> = d.pointer("/summary/positions").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut roles: Vec<Pick<String>> = positions
        .iter()
        .filter_map(|p| {
            let name = p.get("name").and_then(Value::as_str)?.to_lowercase();
            let s = p.get("stats")?;
            let games = u(s, "play");
            let rate = s.get("win_rate").and_then(Value::as_f64).unwrap_or(0.0);
            Some(Pick { value: name, games, wins: (games as f64 * rate).round() as u32 })
        })
        .collect();
    roles.sort_by_key(|r| std::cmp::Reverse(r.games));
    let (games, wins) = match roles.iter().find(|r| r.value == role) {
        Some(r) if mode == "rift" => (r.games, r.wins),
        _ => {
            let s = d.pointer("/summary/average_stats");
            let games = s.map(|s| u(s, "play")).unwrap_or(0);
            let rate = s.and_then(|s| s.get("win_rate")).and_then(Value::as_f64).unwrap_or(0.0);
            (games, (games as f64 * rate).round() as u32)
        }
    };

    let mut runes = Vec::new();
    if let Some(r) = d.get("runes") {
        let mut perks = ids(r, "primary_rune_ids");
        perks.extend(ids(r, "secondary_rune_ids"));
        let primary = r.get("primary_page_id").and_then(Value::as_i64).unwrap_or(0) as i32;
        if perks.len() == 6 && primary > 0 {
            let page = RunePage { primary, sub: r.get("secondary_page_id").and_then(Value::as_i64).unwrap_or(0) as i32, perks, shards: ids(r, "stat_mod_ids") };
            runes.push(pick(r, page));
        }
    }
    let spells = d
        .get("summoner_spells")
        .and_then(|s| {
            let v = ids(s, "ids");
            (v.len() == 2).then(|| pick(s, [v[0], v[1]]))
        })
        .into_iter()
        .collect();
    let starters = d.get("starter_items").map(|s| pick(s, ids(s, "ids"))).filter(|p| !p.value.is_empty()).into_iter().collect();
    let boots = d.get("boots").and_then(|b| ids(b, "ids").first().map(|id| pick(b, *id))).into_iter().collect();

    let mut items: Vec<Pick<i32>> = Vec::new();
    if let Some(c) = d.get("core_items") {
        items.extend(ids(c, "ids").into_iter().map(|id| pick(c, id)));
    }
    let core = items.len() as u32;
    let mut later: Vec<Pick<i32>> = Vec::new();
    for slot in ["fourth_items", "fifth_items", "sixth_items"] {
        for opt in d.get(slot).and_then(Value::as_array).cloned().unwrap_or_default() {
            for id in ids(&opt, "ids") {
                if !items.iter().chain(&later).any(|p| p.value == id) {
                    later.push(pick(&opt, id));
                }
            }
        }
    }
    later.sort_by_key(|p| std::cmp::Reverse(p.games));
    later.truncate(6);
    items.extend(later);
    if items.is_empty() && runes.is_empty() {
        return None;
    }
    Some(Build {
        champion_id,
        mode: mode.to_string(),
        role: if mode == "rift" { role.to_string() } else { String::new() },
        games,
        wins,
        roles: if mode == "rift" { roles } else { Vec::new() },
        runes,
        spells,
        starters,
        boots,
        items,
        core,
        skill_max: d.get("skill_masteries").map(|s| strings(s, "ids")).unwrap_or_default(),
        skill_order: d.get("skills").map(|s| strings(s, "order")).unwrap_or_default(),
    })
}

/// The lane to show when none was asked for: the champion's most played lane on OP.GG.
async fn main_lane(champion_id: i32) -> String {
    match opgg::tiers().await {
        Ok(t) => t
            .rows
            .iter()
            .filter(|r| r.champion_id == champion_id)
            .max_by_key(|r| r.games)
            .map(|r| r.position.clone())
            .unwrap_or_else(|| "mid".into()),
        Err(_) => "mid".into(),
    }
}

/// `mode` "rift" or "aram"; `role` "" picks the champion's main lane.
pub async fn build(champion_id: i32, role: &str, mode: &str) -> Result<Option<Build>> {
    let dd = crate::dd::get("en").await?;
    let champ = dd.champions.get(&champion_id).ok_or_else(|| anyhow!("Unknown champion {champion_id}"))?;
    let (game_mode, role) = if mode == "aram" {
        // OP.GG asks for a lane even in ARAM and ignores it.
        ("aram", "mid".to_string())
    } else {
        let role = if opgg::POSITIONS.contains(&role) { role.to_string() } else { main_lane(champion_id).await };
        ("ranked", role)
    };
    let a = opgg::analysis(&champ.key, &role, game_mode).await?;
    Ok(from_analysis(champion_id, if mode == "aram" { "aram" } else { "rift" }, &role, &a))
}

/// Fallback stat shards (offense, flex, defense) when a build has none.
fn shards(champ_tags: &[String]) -> [i32; 3] {
    let offense = if champ_tags.iter().any(|t| t == "Marksman") { 5005 } else { 5008 };
    [offense, 5008, 5011]
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub rune_page: Option<String>,
    pub item_set: Option<String>,
}

const PAGE_PREFIX: &str = "MF ";

/// Write the build's rune page and/or item set into the client.
pub async fn import(champion_id: i32, role: &str, mode: &str, runes: bool, items: bool) -> Result<ImportResult> {
    let lcu = lcu::require()?;
    let b = build(champion_id, role, mode).await?.ok_or_else(|| anyhow!("[no-build] No build for this champion yet"))?;
    let dd = crate::dd::get("en").await?;
    let champ = dd.champions.get(&champion_id).ok_or_else(|| anyhow!("Unknown champion {champion_id}"))?;
    let role_name = match b.role.as_str() {
        "top" => "Top",
        "jungle" => "Jungle",
        "mid" => "Mid",
        "adc" => "Bot",
        "support" => "Support",
        _ if b.mode == "aram" => "ARAM",
        _ => "",
    };
    let mut out = ImportResult::default();

    if runes {
        let page = b.runes.first().ok_or_else(|| anyhow!("[no-runes] No rune data for this champion yet"))?;
        let name = format!("{PAGE_PREFIX}{} {role_name}", champ.name).trim().to_string();
        let mut perks = page.value.perks.clone();
        if page.value.shards.len() == 3 {
            perks.extend(&page.value.shards);
        } else {
            perks.extend(shards(&champ.tags));
        }
        let pages = lcu.get_value("/lol-perks/v1/pages").await?;
        let pages = pages.as_array().cloned().unwrap_or_default();
        let id_of = |p: &Value| p.get("id").and_then(Value::as_i64).unwrap_or(0);
        let deletable = |p: &Value| p.get("isDeletable").and_then(Value::as_bool).unwrap_or(false);
        // Reuse our own page; otherwise take a free slot; otherwise replace the current page.
        let ours = pages.iter().find(|p| deletable(p) && p.get("name").and_then(Value::as_str).is_some_and(|n| n.starts_with(PAGE_PREFIX)));
        if let Some(p) = ours {
            lcu.delete(&format!("/lol-perks/v1/pages/{}", id_of(p))).await?;
        } else {
            let inv = lcu.get_value("/lol-perks/v1/inventory").await?;
            if !inv.get("canAddCustomPage").and_then(Value::as_bool).unwrap_or(false) {
                let current = pages
                    .iter()
                    .find(|p| deletable(p) && p.get("current").and_then(Value::as_bool).unwrap_or(false))
                    .or_else(|| pages.iter().find(|p| deletable(p)))
                    .ok_or_else(|| anyhow!("[no-rune-page] No rune page can be replaced"))?;
                lcu.delete(&format!("/lol-perks/v1/pages/{}", id_of(current))).await?;
            }
        }
        lcu.post(
            "/lol-perks/v1/pages",
            &json!({ "name": name, "primaryStyleId": page.value.primary, "subStyleId": page.value.sub, "selectedPerkIds": perks, "current": true }),
        )
        .await?;
        out.rune_page = Some(name);
    }

    if items {
        let me = crate::players::me(&lcu).await?;
        let path = format!("/lol-item-sets/v1/item-sets/{}/sets", me.summoner_id);
        let mut sets = lcu.get_value(&path).await?;
        let uid = format!("mattofessor-{champion_id}");
        let title = format!("Mattofessor {} {role_name}", champ.name).trim().to_string();
        let pct = |p: &Pick<i32>| p.games as f64 * 100.0 / b.games.max(1) as f64;
        let block = |name: String, ids: Vec<i32>| {
            json!({ "type": name, "hideIfSummonerSpell": "", "showIfSummonerSpell": "",
                    "items": ids.iter().map(|i| json!({"id": i.to_string(), "count": 1})).collect::<Vec<_>>() })
        };
        let valid = |id: &i32| dd.items.contains_key(id);
        let mut blocks = Vec::new();
        if let Some(s) = b.starters.first() {
            blocks.push(block("Starting items".into(), s.value.iter().copied().filter(valid).collect()));
        }
        let core: Vec<&Pick<i32>> = b.items.iter().take(b.core as usize).collect();
        if let Some(first) = core.first() {
            blocks.push(block(format!("Core items ({:.0}% of games)", pct(first)), core.iter().map(|p| p.value).filter(valid).collect()));
        }
        if !b.boots.is_empty() {
            blocks.push(block("Boots".into(), b.boots.iter().map(|p| p.value).filter(valid).collect()));
        }
        if b.items.len() > b.core as usize {
            blocks.push(block("More options".into(), b.items.iter().skip(b.core as usize).map(|p| p.value).filter(valid).collect()));
        }
        blocks.push(block("Consumables".into(), vec![2003, 2055]));
        let map = if b.mode == "aram" { 12 } else { 11 };
        let new_set = json!({
            "uid": uid, "title": title, "type": "custom", "map": "any", "mode": "any", "sortrank": 0, "startedFrom": "blank",
            "associatedChampions": [champion_id], "associatedMaps": [map], "preferredItemSlots": [], "blocks": blocks,
        });
        let list = sets.get_mut("itemSets").and_then(Value::as_array_mut).ok_or_else(|| anyhow!("Item sets not available"))?;
        list.retain(|s| s.get("uid").and_then(Value::as_str) != Some(uid.as_str()));
        list.insert(0, new_set);
        sets["timestamp"] = json!(crate::players::now_ms());
        lcu.put(&path, &sets).await?;
        out.item_set = Some(title);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const JAX: &str = "class LolGetChampionAnalysis: data\nclass Data: summary,runes,summoner_spells,starter_items,boots,core_items,fourth_items,fifth_items,skill_masteries,skills\nclass Summary: positions,average_stats\nclass Position: name,stats\nclass Stats: play,win_rate,role_rate\nclass AverageStats: play,win_rate\nclass Runes: play,win,primary_page_id,primary_rune_ids,secondary_page_id,secondary_rune_ids,stat_mod_ids\nclass SummonerSpells: ids,play,win\nclass StarterItems: ids,play,win\nclass Boots: ids,play,win\nclass CoreItems: ids,play,win\nclass FourthItem: ids,play,win\nclass FifthItem: ids,play,win\nclass SkillMasteries: ids,play,win\nclass Skills: order,play,win\n\nLolGetChampionAnalysis(Data(Summary([Position(\"TOP\",Stats(70000,0.5,0.8)),Position(\"JUNGLE\",Stats(9000,0.49,0.1))],AverageStats(80000,0.5)),Runes(45548,21825,8000,[8010,9111,9105,8299],8400,[8473,8242],[5008,5008,5011]),SummonerSpells([4,12],60000,30000),StarterItems([1055,2003],40000,20000),Boots([3047],50000,25000),CoreItems([3078,6610,3157],10614,5939),[FourthItem([6333],3000,1700),FourthItem([3078],100,50)],[FifthItem([3053],2000,1100)],SkillMasteries([\"Q\",\"E\",\"W\"],50000,25000),Skills([\"Q\",\"E\",\"W\",\"Q\"],20000,10000)))";

    #[test]
    fn builds_from_opgg_analysis() {
        let a = opgg::parse(JAX).unwrap();
        let b = from_analysis(24, "rift", "top", &a).unwrap();
        assert_eq!((b.games, b.wins), (70000, 35000), "the lane's own numbers");
        assert_eq!(b.roles.iter().map(|r| r.value.as_str()).collect::<Vec<_>>(), ["top", "jungle"]);
        let page = &b.runes[0].value;
        assert_eq!((page.primary, page.sub, page.perks.len(), page.shards.clone()), (8000, 8400, 6, vec![5008, 5008, 5011]));
        assert_eq!(b.spells[0].value, [4, 12]);
        assert_eq!(b.starters[0].value, vec![1055, 2003]);
        assert_eq!(b.boots[0].value, 3047);
        assert_eq!(b.core, 3);
        let item_ids: Vec<i32> = b.items.iter().map(|p| p.value).collect();
        assert_eq!(item_ids, [3078, 6610, 3157, 6333, 3053], "core first, then options without repeats");
        assert_eq!(b.skill_max, ["Q", "E", "W"]);
    }

    #[test]
    fn aram_build_has_no_lane() {
        let a = opgg::parse(JAX).unwrap();
        let b = from_analysis(24, "aram", "mid", &a).unwrap();
        assert_eq!((b.role.as_str(), b.games, b.roles.len()), ("", 80000, 0));
        assert!(from_analysis(24, "rift", "top", &json!({"data": {}})).is_none());
    }
}
