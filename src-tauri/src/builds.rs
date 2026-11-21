//! Champion builds from OP.GG (see `opgg`), and importing them into the client.
//!
//! Summoner's Rift builds are OP.GG's ranked builds for the chosen lane. Hextech ARAM builds take
//! their summoner spells, boots and items from AramHex (Hextech ARAM's own games, see `hexstats`)
//! and the starting items and skill order from OP.GG's ARAM builds (classic ARAM; `item_source`
//! says "opgg" when AramHex has no item data). A build has the rune page with stat shards,
//! summoner spells, starting items, boots, the three core items and the most picked options
//! after them, and the skill order.

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

/// A three-item build: its items in the order made, its games and wins, and the share of the
/// champion's players who made it.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Route {
    pub items: Vec<i32>,
    pub games: u32,
    pub wins: u32,
    pub share: f64,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
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
    /// A lane OP.GG lists for neither the champion nor the lane (an off-meta pick): its few games
    /// are all there is, so the numbers are a rough guide.
    pub rare: bool,
    /// Hextech ARAM: the hand-written build (`content`), used instead of the statistics' items.
    pub mine: Option<crate::content::MayhemBuild>,
    /// Where the spells, boots and items are from: "opgg" or "aramhex".
    pub item_source: String,
    /// Hextech ARAM: the most made three-item builds (AramHex), most made first. Each first item
    /// is a style of its own (`styles`), imported as an item set of its own.
    pub routes: Vec<Route>,
}

/// Never part of a recommended build (the user's rule): potions, wards, elixirs, trinkets and
/// other consumables, and the plain Boots (unfinished).
pub const JUNK: [i32; 21] = [
    1001, // Boots
    2003, // Health Potion
    2010, // Total Biscuit of Everlasting Will
    2031, // Refillable Potion
    2033, // Corrupting Potion
    2052, // Poro-Snax
    2055, // Control Ward
    2138, // Elixir of Iron
    2139, // Elixir of Sorcery
    2140, // Elixir of Wrath
    2150, // Elixir of Skill
    2151, // Elixir of Avarice
    2152, // Elixir of Force
    2419, // Commencing Stopwatch
    2420, // Stopwatch
    2421, // Broken Stopwatch
    3330, // Scarecrow Effigy
    3340, // Stealth Ward
    3363, // Farsight Alteration
    3364, // Oracle Lens
    3400, // Your Cut
];

pub fn junk(id: i32) -> bool {
    JUNK.contains(&id)
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
    let starters = d
        .get("starter_items")
        .map(|s| pick(s, ids(s, "ids").into_iter().filter(|id| !junk(*id)).collect::<Vec<_>>()))
        .filter(|p| !p.value.is_empty())
        .into_iter()
        .collect();
    let boots = d.get("boots").and_then(|b| ids(b, "ids").into_iter().find(|id| !junk(*id)).map(|id| pick(b, id))).into_iter().collect();

    let mut items: Vec<Pick<i32>> = Vec::new();
    if let Some(c) = d.get("core_items") {
        items.extend(ids(c, "ids").into_iter().filter(|id| !junk(*id)).map(|id| pick(c, id)));
    }
    let core = items.len() as u32;
    let mut later: Vec<Pick<i32>> = Vec::new();
    for slot in ["fourth_items", "fifth_items", "sixth_items"] {
        for opt in d.get(slot).and_then(Value::as_array).cloned().unwrap_or_default() {
            for id in ids(&opt, "ids") {
                if !junk(id) && !items.iter().chain(&later).any(|p| p.value == id) {
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
        rare: false,
        mine: None,
        item_source: "opgg".into(),
        routes: Vec::new(),
    })
}

/// Hextech ARAM's own spells, boots and items in place of classic ARAM's.
pub fn apply_aram(b: &mut Build, a: &crate::hexstats::AramBuild) {
    let pick = |(value, games, wins): (i32, u64, u64)| Pick { value, games: games.min(u32::MAX as u64) as u32, wins: wins.min(u32::MAX as u64) as u32 };
    b.games = a.games.min(u32::MAX as u64) as u32;
    b.wins = a.wins.min(u32::MAX as u64) as u32;
    if !a.spells.is_empty() {
        b.spells = a.spells.iter().map(|(ids, g, w)| Pick { value: *ids, games: (*g).min(u32::MAX as u64) as u32, wins: (*w).min(u32::MAX as u64) as u32 }).collect();
    }
    b.boots = a.boots.iter().copied().map(pick).collect();
    b.items = a.items.iter().copied().map(pick).collect();
    b.core = a.core.len().min(b.items.len()) as u32;
    b.routes = a.routes.iter().map(|(items, g, w, share)| Route { items: items.clone(), games: (*g).min(u32::MAX as u64) as u32, wins: (*w).min(u32::MAX as u64) as u32, share: *share }).collect();
    b.item_source = "aramhex".into();
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
    let mine = if mode == "aram" { crate::content::build(champion_id) } else { None }.map(|mut m| {
        for list in [&mut m.starters, &mut m.order, &mut m.situational] {
            list.retain(|id| !junk(*id));
        }
        m
    });
    let mut b = match opgg::analysis(&champ.key, &role, game_mode).await {
        Ok(a) => from_analysis(champion_id, if mode == "aram" { "aram" } else { "rift" }, &role, &a),
        // A hand-written build, or Hextech ARAM's statistics, show without OP.GG.
        Err(e) if mine.is_some() || mode == "aram" => {
            log::warn!("OP.GG ARAM build for {champion_id}: {e:#}");
            None
        }
        Err(e) => return Err(e),
    };
    if mode == "aram" {
        match crate::hexstats::aram_build(champion_id).await {
            Ok(Some(a)) => apply_aram(b.get_or_insert_with(|| Build { champion_id, mode: "aram".into(), item_source: "opgg".into(), ..Default::default() }), &a),
            Ok(None) => {}
            Err(e) => log::warn!("AramHex build for {champion_id}: {e:#}"),
        }
    }
    if let Some(m) = mine {
        let b = b.get_or_insert_with(|| Build { champion_id, mode: "aram".into(), item_source: "opgg".into(), ..Default::default() });
        b.mine = Some(m);
    }
    if let Some(b) = b.as_mut().filter(|b| b.mode == "rift" && !b.roles.iter().any(|r| r.value == role)) {
        let lane = opgg::tiers().await.ok().and_then(|t| t.rows.iter().find(|r| r.champion_id == champion_id && r.position == role).cloned());
        off_lane(b, &role, lane.as_ref());
    }
    Ok(b)
}

/// A lane the champion's own summary leaves out (it lists its usual lanes only): the lane's
/// numbers come from OP.GG's lane list when it has them (an off-meta pick with many games),
/// else from the build itself (a rare pick: a handful of games).
fn off_lane(b: &mut Build, role: &str, lane: Option<&opgg::TierRow>) {
    let (games, wins) = match lane {
        Some(r) => (r.games as u32, (r.games as f64 * r.win_rate).round() as u32),
        None => {
            b.rare = true;
            b.runes.first().map(|p| (p.games, p.wins)).unwrap_or((0, 0))
        }
    };
    if lane.is_some() {
        b.roles.push(Pick { value: role.to_string(), games, wins });
    }
    (b.games, b.wins) = (games, wins);
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
        // The sets are read in the game's shop: in the client's language, with its item names.
        let lang = crate::core::get().and_then(|c| c.client_locale.read().clone()).map(|l| crate::lang::from_client(&l)).unwrap_or_else(crate::lang::english);
        let local = crate::dd::get(lang.id).await.unwrap_or_else(|_| dd.clone());
        let champ_name = local.champions.get(&champion_id).map(|c| c.name.clone()).unwrap_or_else(|| champ.name.clone());
        let item_name = |id: i32| local.items.get(&id).or_else(|| dd.items.get(&id)).map(|i| i.name.clone()).unwrap_or_else(|| id.to_string());
        let valid = |id: &i32| dd.items.contains_key(id) && !junk(*id);
        let new_sets = item_sets(&b, champion_id, &champ_name, words(lang.id), item_name, valid);
        let title = new_sets[0]["title"].as_str().unwrap_or_default().to_string();
        let list = sets.get_mut("itemSets").and_then(Value::as_array_mut).ok_or_else(|| anyhow!("Item sets not available"))?;
        // This champion's sets from an earlier import go (their styles may have changed).
        list.retain(|s| !ours(s.get("uid").and_then(Value::as_str).unwrap_or(""), champion_id));
        for (i, s) in new_sets.into_iter().enumerate() {
            list.insert(i, s);
        }
        sets["timestamp"] = json!(crate::players::now_ms());
        lcu.put(&path, &sets).await?;
        out.item_set = Some(title);
    }
    Ok(out)
}

/// An item set Mattofessor imported for this champion: the build's set or one of its styles.
fn ours(uid: &str, champion_id: i32) -> bool {
    let base = format!("mattofessor-{champion_id}");
    uid == base || uid.strip_prefix(&base).is_some_and(|rest| rest.starts_with("-style-"))
}

/// What the item sets say, in one language (`lang` id). `{0}` stands for a number or a name.
pub struct SetWords {
    pub starting: &'static str,
    pub order: &'static str,
    pub situational: &'static str,
    pub core: &'static str,
    /// After the core items: how many of the champion's games made them.
    pub share: &'static str,
    pub boots: &'static str,
    pub more: &'static str,
    /// A style: the build around an item ("{0} build").
    pub style: &'static str,
    pub aram: &'static str,
    /// Top, jungle, mid, bot, support.
    pub roles: [&'static str; 5],
}

pub fn words(lang: &str) -> &'static SetWords {
    const EN: SetWords = SetWords {
        starting: "Starting items",
        order: "Build order",
        situational: "Situational",
        core: "Core items",
        share: "{0}% of games",
        boots: "Boots",
        more: "More options",
        style: "{0} build",
        aram: "ARAM",
        roles: ["Top", "Jungle", "Mid", "Bot", "Support"],
    };
    const ZH: SetWords = SetWords {
        starting: "出门装",
        order: "出装顺序",
        situational: "备选",
        core: "核心装备",
        share: "{0}% 的对局",
        boots: "鞋子",
        more: "其他选择",
        style: "{0}流",
        aram: "大乱斗",
        roles: ["上单", "打野", "中单", "下路", "辅助"],
    };
    const ZH_TW: SetWords = SetWords {
        starting: "出門裝",
        order: "出裝順序",
        situational: "備選",
        core: "核心道具",
        share: "{0}% 的對戰",
        boots: "鞋子",
        more: "其他選擇",
        style: "{0}流",
        aram: "隨機單中",
        roles: ["上單", "打野", "中單", "下路", "輔助"],
    };
    const JA: SetWords = SetWords {
        starting: "初期アイテム",
        order: "購入順",
        situational: "状況次第",
        core: "コアアイテム",
        share: "{0}% の試合",
        boots: "ブーツ",
        more: "その他の候補",
        style: "{0}型",
        aram: "ARAM",
        roles: ["トップ", "ジャングル", "ミッド", "ボット", "サポート"],
    };
    const KO: SetWords = SetWords {
        starting: "시작 아이템",
        order: "구매 순서",
        situational: "상황별",
        core: "핵심 아이템",
        share: "게임의 {0}%",
        boots: "장화",
        more: "그 밖의 선택",
        style: "{0} 빌드",
        aram: "칼바람",
        roles: ["탑", "정글", "미드", "바텀", "서포터"],
    };
    const ES: SetWords = SetWords {
        starting: "Objetos iniciales",
        order: "Orden de compra",
        situational: "Situacional",
        core: "Objetos clave",
        share: "{0}% de las partidas",
        boots: "Botas",
        more: "Más opciones",
        style: "Build de {0}",
        aram: "ARAM",
        roles: ["Superior", "Jungla", "Central", "Inferior", "Apoyo"],
    };
    const FR: SetWords = SetWords {
        starting: "Objets de départ",
        order: "Ordre d'achat",
        situational: "Selon la situation",
        core: "Objets principaux",
        share: "{0} % des parties",
        boots: "Bottes",
        more: "Autres options",
        style: "Build {0}",
        aram: "ARAM",
        roles: ["Haut", "Jungle", "Milieu", "Bas", "Support"],
    };
    const RU: SetWords = SetWords {
        starting: "Стартовые предметы",
        order: "Порядок сборки",
        situational: "По ситуации",
        core: "Основные предметы",
        share: "{0}% игр",
        boots: "Ботинки",
        more: "Другие варианты",
        style: "Сборка через {0}",
        aram: "ARAM",
        roles: ["Верх", "Лес", "Центр", "Низ", "Поддержка"],
    };
    match lang {
        "zh" => &ZH,
        "zh-TW" => &ZH_TW,
        "ja" => &JA,
        "ko" => &KO,
        "es" => &ES,
        "fr" => &FR,
        "ru" => &RU,
        _ => &EN,
    }
}

/// Styles of a Hextech ARAM build: its three-item builds grouped by their first item, each with
/// its most made build (routes come most made first). As the Builds page groups them.
fn styles(b: &Build) -> Vec<(i32, &Route)> {
    let mut out: Vec<(i32, &Route)> = Vec::new();
    for r in &b.routes {
        if let Some(first) = r.items.first() {
            if !out.iter().any(|(f, _)| f == first) {
                out.push((*first, r));
            }
        }
    }
    out
}

/// The item sets for a build, its own first: the build (the hand-written one when there is one),
/// then, for Hextech ARAM builds with two or more styles, one set per other style so the shop's
/// set list offers each; the build's own style is named in its title instead of a copy of it
/// (`item_name` names a style's item; `valid` keeps items the shop sells).
pub fn item_sets(b: &Build, champion_id: i32, champ_name: &str, w: &SetWords, item_name: impl Fn(i32) -> String, valid: impl Fn(&i32) -> bool) -> Vec<Value> {
    let role = match b.role.as_str() {
        "top" => w.roles[0],
        "jungle" => w.roles[1],
        "mid" => w.roles[2],
        "adc" => w.roles[3],
        "support" => w.roles[4],
        _ if b.mode == "aram" => w.aram,
        _ => "",
    };
    let share = |rate: f64| w.share.replace("{0}", &format!("{:.0}", rate * 100.0));
    let block = |name: String, ids: Vec<i32>| {
        json!({ "type": name, "hideIfSummonerSpell": "", "showIfSummonerSpell": "",
                "items": ids.iter().map(|i| json!({"id": i.to_string(), "count": 1})).collect::<Vec<_>>() })
    };
    let starting = || b.starters.first().map(|s| block(w.starting.into(), s.value.iter().copied().filter(&valid).collect()));
    let boots = || (!b.boots.is_empty()).then(|| block(w.boots.into(), b.boots.iter().map(|p| p.value).filter(&valid).collect()));
    let set = |uid: String, title: String, blocks: Vec<Value>| {
        let map = if b.mode == "aram" { 12 } else { 11 };
        json!({
            "uid": uid, "title": title, "type": "custom", "map": "any", "mode": "any", "sortrank": 0, "startedFrom": "blank",
            "associatedChampions": [champion_id], "associatedMaps": [map], "preferredItemSlots": [], "blocks": blocks,
        })
    };
    let style_name = |first: i32| w.style.replace("{0}", &item_name(first));

    let core: Vec<i32> = if b.mine.is_some() { Vec::new() } else { b.items.iter().take(b.core as usize).map(|p| p.value).collect() };
    // The build's core is one of the styles' routes (AramHex's most made one): its own share.
    let own_route = b.routes.iter().find(|r| !core.is_empty() && r.items == core);
    let mut blocks = Vec::new();
    if let Some(m) = &b.mine {
        blocks.push(block(w.starting.into(), m.starters.iter().copied().filter(&valid).collect()));
        blocks.push(block(w.order.into(), m.order.iter().copied().filter(&valid).collect()));
        if !m.situational.is_empty() {
            blocks.push(block(w.situational.into(), m.situational.iter().copied().filter(&valid).collect()));
        }
    } else {
        blocks.extend(starting());
        if let Some(first) = b.items.first().filter(|_| b.core > 0) {
            let rate = own_route.map(|r| r.share).unwrap_or(first.games as f64 / b.games.max(1) as f64);
            blocks.push(block(format!("{} ({})", w.core, share(rate)), core.iter().copied().filter(&valid).collect()));
        }
        blocks.extend(boots());
        if b.items.len() > b.core as usize {
            blocks.push(block(w.more.into(), b.items.iter().skip(b.core as usize).map(|p| p.value).filter(&valid).collect()));
        }
    }
    let styles = styles(b);
    let several = styles.len() >= 2;
    let mut title = format!("Mattofessor {champ_name} {role}").trim().to_string();
    if let (true, Some(first)) = (several, own_route.and_then(|r| r.items.first())) {
        title = format!("{title} · {}", style_name(*first));
    }
    let mut out = vec![set(format!("mattofessor-{champion_id}"), title, blocks)];

    if several {
        for (first, route) in styles.into_iter().filter(|(_, r)| Some(*r) != own_route) {
            let mut blocks: Vec<Value> = starting().into_iter().collect();
            blocks.push(block(format!("{} ({})", w.core, share(route.share)), route.items.iter().copied().filter(&valid).collect()));
            blocks.extend(boots());
            let others: Vec<i32> = b.items.iter().map(|p| p.value).filter(|i| !route.items.contains(i)).filter(&valid).collect();
            if !others.is_empty() {
                blocks.push(block(w.more.into(), others));
            }
            out.push(set(format!("mattofessor-{champion_id}-style-{first}"), format!("MF {champ_name}: {}", style_name(first)), blocks));
        }
    }
    out
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
        assert_eq!(b.starters[0].value, vec![1055], "the potion is left out");
        assert_eq!(b.boots[0].value, 3047);
        assert_eq!(b.core, 3);
        let item_ids: Vec<i32> = b.items.iter().map(|p| p.value).collect();
        assert_eq!(item_ids, [3078, 6610, 3157, 6333, 3053], "core first, then options without repeats");
        assert_eq!(b.skill_max, ["Q", "E", "W"]);
    }

    #[test]
    fn an_off_meta_lane_gets_its_own_numbers() {
        let a = opgg::parse(JAX).unwrap();
        // Mid: OP.GG's lane list has it (many games).
        let mut b = from_analysis(24, "rift", "mid", &a).unwrap();
        let row = opgg::TierRow { champion_id: 24, position: "mid".into(), tier: 4, rank: 50, games: 12000, win_rate: 0.5, pick_rate: 0.01, ban_rate: 0.0, role_rate: 0.12 };
        off_lane(&mut b, "mid", Some(&row));
        assert_eq!((b.games, b.wins, b.rare), (12000, 6000, false));
        assert!(b.roles.iter().any(|r| r.value == "mid" && r.games == 12000));
        // Support: nowhere but in the build itself.
        let mut b = from_analysis(24, "rift", "support", &a).unwrap();
        off_lane(&mut b, "support", None);
        assert_eq!((b.games, b.wins, b.rare), (45548, 21825, true), "the rune page's games, not the champion's");
    }

    #[test]
    fn item_sets_one_per_style_in_the_clients_language() {
        let p = |value: i32, games: u32| Pick { value, games, wins: games / 2 };
        let route = |items: [i32; 3], share: f64| Route { items: items.to_vec(), games: 100, wins: 50, share };
        let mut b = Build {
            champion_id: 24,
            mode: "aram".into(),
            games: 1000,
            wins: 520,
            starters: vec![Pick { value: vec![1055, 2003], games: 900, wins: 450 }],
            boots: vec![p(3047, 600)],
            items: vec![p(3078, 400), p(3153, 380), p(6333, 300), p(3053, 200), p(2003, 100)],
            core: 3,
            // Two routes start with Trinity Force: one style, its most made route (the build's own).
            routes: vec![route([3078, 3153, 6333], 0.30), route([3153, 3078, 6333], 0.20), route([3078, 6333, 3053], 0.10)],
            item_source: "aramhex".into(),
            ..Default::default()
        };
        let names = |id: i32| match id {
            3078 => "三相之力".to_string(),
            3153 => "破败王者之刃".to_string(),
            _ => id.to_string(),
        };
        let valid = |id: &i32| *id != 2003;
        let sets = item_sets(&b, 24, "贾克斯", words("zh"), names, valid);
        let titles: Vec<&str> = sets.iter().map(|s| s["title"].as_str().unwrap()).collect();
        assert_eq!(titles, ["Mattofessor 贾克斯 大乱斗 · 三相之力流", "MF 贾克斯: 破败王者之刃流"], "the build's own style is named, not copied");
        let uids: Vec<&str> = sets.iter().map(|s| s["uid"].as_str().unwrap()).collect();
        assert_eq!(uids, ["mattofessor-24", "mattofessor-24-style-3153"]);
        let block_names = |s: &Value| s["blocks"].as_array().unwrap().iter().map(|b| b["type"].as_str().unwrap().to_string()).collect::<Vec<_>>();
        assert_eq!(block_names(&sets[0]), ["出门装", "核心装备 (30% 的对局)", "鞋子", "其他选择"], "the route's own share of players");
        assert_eq!(block_names(&sets[1]), ["出门装", "核心装备 (20% 的对局)", "鞋子", "其他选择"]);
        let ids = |s: &Value, i: usize| s["blocks"][i]["items"].as_array().unwrap().iter().map(|x| x["id"].as_str().unwrap().to_string()).collect::<Vec<_>>();
        assert_eq!(ids(&sets[0], 0), ["1055"], "no potion");
        assert_eq!(ids(&sets[1], 1), ["3153", "3078", "6333"], "the style's own route");
        assert_eq!(ids(&sets[1], 3), ["3053"], "the other items, without the route's");

        // One style only: the build's own set, no style in its title.
        b.routes = vec![route([3078, 3153, 6333], 0.3), route([3078, 6333, 3153], 0.1)];
        let one = item_sets(&b, 24, "Jax", words("en"), names, valid);
        assert_eq!((one.len(), one[0]["title"].as_str().unwrap()), (1, "Mattofessor Jax ARAM"));
        // A Rift build keeps its lane; OP.GG's core share is its first item's games.
        let rift = Build { mode: "rift".into(), role: "top".into(), routes: Vec::new(), ..b };
        let sets = item_sets(&rift, 24, "Jax", words("en"), names, valid);
        assert_eq!(sets[0]["title"], "Mattofessor Jax Top");
        assert_eq!(sets[0]["blocks"][1]["type"], "Core items (40% of games)");
        assert!(ours("mattofessor-24", 24) && ours("mattofessor-24-style-3078", 24));
        assert!(!ours("mattofessor-245", 24) && !ours("mattofessor-245-style-1", 24) && !ours("my-own-set", 24));
    }
    #[test]
    fn aram_build_has_no_lane() {
        let a = opgg::parse(JAX).unwrap();
        let b = from_analysis(24, "aram", "mid", &a).unwrap();
        assert_eq!((b.role.as_str(), b.games, b.roles.len()), ("", 80000, 0));
        assert!(from_analysis(24, "rift", "top", &json!({"data": {}})).is_none());
    }
}
