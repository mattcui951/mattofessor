//! Turns a player's rank, recent games and champion mastery into the summary and tags shown
//! for them. Everything here is pure so the rules can be unit tested.
//!
//! Tag rules (games = their recent games without remakes; Rift = Summoner's Rift games; mode
//! games = games of the scouted game's kind, ARAM and Hextech ARAM together, when 5+, else all):
//!   oneTrick      60%+ of 10+ recent games on one champion
//!   strongChamp   70%+ win rate over 5+ mode games on one champion (not their one-trick, not the
//!                 champion an expert tag already covers)
//!   highWr/lowWr  60%+ / 40%- win rate over 15+ mode games (no highWr on a suspected smurf)
//!   style         40%+ of 10+ mode games on one kind of champion: tank, marksman, mage, poke
//!                 (long-range artillery), fighter, assassin or support (Data Dragon's first tag).
//!                 In Hextech ARAM a player's top kind is usually 25-35% (champions are partly
//!                 random), so 40% stands out.
//! In ARAM-like and Arena games the Rift-only tags below (role, play style, deaths) are left out:
//! everyone dies a lot in Hextech ARAM, and lanes and farming do not apply.
//!   mainRole      60%+ of 8+ Rift games in one role
//!   offRole       assigned a role they played in under 10% of 10+ Rift games whose role is certain
//!                 (see `positions` for how roles are worked out)
//!   firstTime     no recent games and under 10k mastery on the champion they picked
//!   expert        200k+ mastery, or 55%+ win rate over 10+ recent games, on that champion
//!   farmer        Rift CS/min at least 1 above their role's typical value (not supports)
//!   fighter       Rift kills+assists per minute of 0.65+ (0.55 for supports)
//!   carry         Rift damage to champions per minute of 900+ (not supports)
//!   frontliner    Rift damage taken per minute of 1400+
//!   vision        Rift vision score per minute above their role's bar
//!   diesALot      7.5+ deaths per Rift game over 5+;  safe: under 3.5 over 8+ Rift games
//!   winStreak / lossStreak   3+ in a row, most recent first
//!   smurf         level under 80 with 65%+ win rate and 3.5+ KDA over 10+ recent games, or level under
//!                 60 already Emerald or higher in solo queue, or (with a Riot API key, which gives
//!                 exact ranked losses) level under 80 with 62%+ over 15+ solo queue games.
//!   wasHigher     last season's peak 2+ tiers (8 divisions) above their current solo rank
//!   inactive      their latest game is 30+ days old
//!   premade       set by the game, not here (see `premade_groups`)

use crate::model::{GameRow, Mastery, Ranked, Summoner, TierDiv};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    /// i18n key, e.g. "oneTrick".
    pub key: String,
    /// "pos" | "neg" | "info" | "duo" | "" (neutral)
    pub tone: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arg: Option<Value>,
}

fn tag(key: &str, tone: &str, arg: Option<Value>) -> Tag {
    Tag { key: key.into(), tone: tone.into(), arg }
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChampStats {
    pub champion_id: i32,
    pub games: u32,
    pub wins: u32,
    pub kills: f32,
    pub deaths: f32,
    pub assists: f32,
    pub mastery_level: u32,
    pub mastery_points: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentGame {
    pub game_id: u64,
    pub champion_id: i32,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub win: bool,
    pub remake: bool,
    pub queue_id: i32,
    pub created: i64,
    pub placement: u32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleShare {
    pub role: String,
    pub games: u32,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerCard {
    pub summoner: Summoner,
    pub ranked: Ranked,
    /// Recent games (remakes excluded).
    pub games: u32,
    pub wins: u32,
    pub kills: f32,
    pub deaths: f32,
    pub assists: f32,
    pub recent: Vec<RecentGame>,
    /// Stats on the champion they are playing now, when known.
    pub champ: Option<ChampStats>,
    pub top_champs: Vec<ChampStats>,
    pub roles: Vec<RoleShare>,
    pub tags: Vec<Tag>,
    pub last_game: Option<i64>,
    /// Was a smurf suspected (also present as a tag; kept for sorting/highlighting).
    pub smurf: bool,
    /// Built from freshly fetched games (false while only cached games are known).
    pub fresh: bool,
}

/// What we know about the game being scouted.
#[derive(Clone, Debug, Default)]
pub struct Context {
    pub champion_id: Option<i32>,
    /// Assigned position as the client names it: TOP, JUNGLE, MIDDLE, BOTTOM, UTILITY.
    pub position: Option<String>,
    /// Game mode of the game being scouted (CLASSIC, ARAM, CHERRY...).
    pub mode: Option<String>,
    pub now_ms: i64,
    /// Champion id to its kind (`class_of`); empty until champion data has loaded.
    pub classes: std::sync::Arc<HashMap<i32, String>>,
}

/// Long-range poke champions, by Data Dragon key: their mage or marksman tag alone does not say it.
const POKE: [&str; 10] = ["Xerath", "Ziggs", "Velkoz", "Lux", "Jayce", "Varus", "Zoe", "Hwei", "Nidalee", "Mel"];

/// A champion's kind for the style tag, from its Data Dragon key and tags (the first tag leads).
pub fn class_of(key: &str, tags: &[String]) -> String {
    if POKE.contains(&key) {
        return "poke".into();
    }
    match tags.first().map(String::as_str) {
        Some("Tank") => "tank",
        Some("Marksman") => "marksman",
        Some("Mage") => "mage",
        Some("Fighter") => "fighter",
        Some("Assassin") => "assassin",
        Some("Support") => "support",
        _ => "",
    }
    .into()
}

/// The kind of game a mode is: "rift", "aram" (ARAM and Hextech ARAM) or "arena".
fn family(mode: &str) -> &'static str {
    if mode == "ARAM" || mode.starts_with("KIWI") {
        "aram"
    } else if mode == "CHERRY" {
        "arena"
    } else {
        "rift"
    }
}

fn position_role(p: &str) -> Option<&'static str> {
    match p.to_uppercase().as_str() {
        "TOP" => Some("top"),
        "JUNGLE" => Some("jungle"),
        "MIDDLE" | "MID" => Some("mid"),
        "BOTTOM" | "BOT" => Some("adc"),
        "UTILITY" | "SUPPORT" => Some("support"),
        _ => None,
    }
}

fn typical_csm(role: &str) -> f64 {
    match role {
        "top" => 6.6,
        "jungle" => 5.6,
        "mid" => 7.0,
        "adc" => 7.6,
        _ => 99.0,
    }
}

fn vision_bar(role: &str) -> f64 {
    match role {
        "support" => 2.3,
        "jungle" => 1.4,
        _ => 1.05,
    }
}

fn avg<F: Fn(&GameRow) -> f64>(rows: &[&GameRow], f: F) -> f64 {
    if rows.is_empty() {
        return 0.0;
    }
    rows.iter().map(|r| f(r)).sum::<f64>() / rows.len() as f64
}

fn champ_stats(champion_id: i32, rows: &[&GameRow], masteries: &[Mastery]) -> ChampStats {
    let mine: Vec<&GameRow> = rows.iter().copied().filter(|r| r.champion_id == champion_id).collect();
    let m = masteries.iter().find(|m| m.champion_id == champion_id);
    ChampStats {
        champion_id,
        games: mine.len() as u32,
        wins: mine.iter().filter(|r| r.win).count() as u32,
        kills: avg(&mine, |r| r.kills as f64) as f32,
        deaths: avg(&mine, |r| r.deaths as f64) as f32,
        assists: avg(&mine, |r| r.assists as f64) as f32,
        mastery_level: m.map(|m| m.level).unwrap_or(0),
        mastery_points: m.map(|m| m.points).unwrap_or(0),
    }
}

/// `history` is most recent first.
pub fn build_card(summoner: Summoner, ranked: Ranked, history: &[GameRow], masteries: &[Mastery], ctx: &Context) -> PlayerCard {
    let played: Vec<&GameRow> = history.iter().filter(|r| !r.remake).collect();
    // Judge them on games of the scouted game's kind when there is enough of it.
    let kind = ctx.mode.as_deref().map(family);
    let same_mode: Vec<&GameRow> = match kind {
        Some(k) => played.iter().copied().filter(|r| family(&r.mode) == k).collect(),
        None => Vec::new(),
    };
    let pool: Vec<&GameRow> = if same_mode.len() >= 5 { same_mode } else { played.clone() };
    let rift: Vec<&GameRow> = played.iter().copied().filter(|r| r.is_rift()).collect();
    // Rift habits (role, farming, vision, deaths) say nothing about an ARAM or Arena game.
    let rift_tags = kind.is_none_or(|k| k == "rift");

    let mut card = PlayerCard {
        games: played.len() as u32,
        wins: played.iter().filter(|r| r.win).count() as u32,
        kills: avg(&played, |r| r.kills as f64) as f32,
        deaths: avg(&played, |r| r.deaths as f64) as f32,
        assists: avg(&played, |r| r.assists as f64) as f32,
        recent: history
            .iter()
            .take(10)
            .map(|r| RecentGame {
                game_id: r.game_id,
                champion_id: r.champion_id,
                kills: r.kills,
                deaths: r.deaths,
                assists: r.assists,
                win: r.win,
                remake: r.remake,
                queue_id: r.queue_id,
                created: r.created,
                placement: r.placement,
            })
            .collect(),
        last_game: history.first().map(|r| r.created),
        ..Default::default()
    };

    // Champions by games played.
    let mut by_champ: HashMap<i32, u32> = HashMap::new();
    for r in &played {
        *by_champ.entry(r.champion_id).or_default() += 1;
    }
    let mut champs: Vec<(i32, u32)> = by_champ.into_iter().collect();
    champs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    card.top_champs = champs.iter().take(5).map(|(c, _)| champ_stats(*c, &played, masteries)).collect();
    card.champ = ctx.champion_id.filter(|c| *c > 0).map(|c| champ_stats(c, &played, masteries));

    // Roles on the Rift.
    let mut by_role: HashMap<String, u32> = HashMap::new();
    let mut sure_roles: HashMap<String, u32> = HashMap::new();
    let mut sure_games = 0u32;
    for r in &rift {
        let role = crate::positions::role_of(r);
        if !role.single.is_empty() {
            *by_role.entry(role.single.clone()).or_default() += 1;
        }
        if role.sure {
            sure_games += 1;
            *sure_roles.entry(role.single).or_default() += 1;
        }
    }
    let mut roles: Vec<RoleShare> = by_role.into_iter().map(|(role, games)| RoleShare { role, games }).collect();
    roles.sort_by(|a, b| b.games.cmp(&a.games).then(a.role.cmp(&b.role)));
    card.roles = roles;

    let mut tags: Vec<Tag> = Vec::new();

    // Smurf suspicion first: it is the most important thing to know about an opponent.
    let solo_score = ranked.solo.as_ref().and_then(|r| TierDiv::new(&r.tier, &r.division)).map(|t| t.score()).unwrap_or(-1);
    let recent_wr = card.wins as f64 / card.games.max(1) as f64;
    let recent_kda = (card.kills + card.assists) as f64 / (card.deaths as f64).max(1.0);
    let level = summoner.level;
    let ranked_hot = ranked.exact
        && ranked.solo.as_ref().is_some_and(|r| r.wins + r.losses >= 15 && r.wins as f64 / (r.wins + r.losses) as f64 >= 0.62);
    if level > 0
        && ((level < 80 && card.games >= 10 && recent_wr >= 0.65 && recent_kda >= 3.5) || (level < 60 && solo_score >= 20) || (level < 80 && ranked_hot))
    {
        card.smurf = true;
        tags.push(tag("smurf", "neg", None));
    }

    // Champion.
    let mut expert_on = None;
    if let Some(cs) = &card.champ {
        if cs.games == 0 && cs.mastery_points < 10_000 {
            tags.push(tag("firstTime", "neg", None));
        } else if cs.mastery_points >= 200_000 || (cs.games >= 10 && cs.wins as f64 / cs.games as f64 >= 0.55) {
            tags.push(tag("expert", "pos", None));
            expert_on = Some(cs.champion_id);
        }
    }
    let mut one_trick = None;
    if let Some((c, n)) = champs.first() {
        if played.len() >= 10 && *n as f64 / played.len() as f64 >= 0.6 {
            tags.push(tag("oneTrick", "info", Some(json!(c))));
            one_trick = Some(*c);
        }
    }

    // A champion they win a lot on, in this kind of game.
    let mut on_champ: HashMap<i32, (u32, u32)> = HashMap::new();
    for r in &pool {
        let e = on_champ.entry(r.champion_id).or_default();
        e.0 += 1;
        e.1 += u32::from(r.win);
    }
    let strong = on_champ
        .iter()
        .filter(|(c, (g, w))| *g >= 5 && *w as f64 / *g as f64 >= 0.7 && Some(**c) != one_trick && Some(**c) != expert_on)
        .max_by_key(|(c, (g, w))| (*w, *g, std::cmp::Reverse(**c)));
    if let Some((c, (g, w))) = strong {
        tags.push(tag("strongChamp", "pos", Some(json!([c, (*w as f64 * 100.0 / *g as f64).round()]))));
    }

    // Win rate in this kind of game.
    let pool_wins = pool.iter().filter(|r| r.win).count() as f64;
    if pool.len() >= 15 {
        let wr = pool_wins / pool.len() as f64;
        let pct = json!((wr * 100.0).round());
        if wr >= 0.6 && !card.smurf {
            tags.push(tag("highWr", "pos", Some(pct)));
        } else if wr <= 0.4 {
            tags.push(tag("lowWr", "neg", Some(pct)));
        }
    }

    // The kind of champion they play most.
    if pool.len() >= 10 {
        let mut by_class: HashMap<&str, u32> = HashMap::new();
        for r in &pool {
            if let Some(c) = ctx.classes.get(&r.champion_id).filter(|c| !c.is_empty()) {
                *by_class.entry(c.as_str()).or_default() += 1;
            }
        }
        if let Some((c, n)) = by_class.into_iter().max_by_key(|(c, n)| (*n, std::cmp::Reverse(*c))) {
            if n as f64 / pool.len() as f64 >= 0.4 {
                tags.push(tag("style", "info", Some(json!(c))));
            }
        }
    }

    // Role.
    let main_role = card.roles.first().filter(|r| rift.len() >= 8 && r.games as f64 / rift.len() as f64 >= 0.6).map(|r| r.role.clone());
    if let Some(assigned) = ctx.position.as_deref().and_then(position_role).filter(|_| rift_tags) {
        let share = sure_roles.get(assigned).copied().unwrap_or(0) as f64 / sure_games.max(1) as f64;
        if sure_games >= 8 && share < 0.1 {
            tags.push(tag("offRole", "neg", Some(json!(assigned))));
        }
    }
    if let Some(r) = main_role.as_ref().filter(|_| rift_tags) {
        tags.push(tag("mainRole", "info", Some(json!(r))));
    }

    // Play style on the Rift.
    if rift_tags && rift.len() >= 5 {
        let role = main_role.clone().unwrap_or_default();
        let csm = avg(&rift, |r| r.cs as f64 / r.minutes());
        if role != "support" && !role.is_empty() && csm >= typical_csm(&role) + 1.0 {
            tags.push(tag("farmer", "", None));
        }
        let ka = avg(&rift, |r| (r.kills + r.assists) as f64 / r.minutes());
        if ka >= if role == "support" { 0.55 } else { 0.65 } {
            tags.push(tag("fighter", "", None));
        }
        let dpm = avg(&rift, |r| r.damage as f64 / r.minutes());
        if role != "support" && dpm >= 900.0 {
            tags.push(tag("carry", "", None));
        }
        let tpm = avg(&rift, |r| r.taken as f64 / r.minutes());
        if tpm >= 1400.0 {
            tags.push(tag("frontliner", "", None));
        }
        let vpm = avg(&rift, |r| r.vision as f64 / r.minutes());
        if vpm >= vision_bar(&role) {
            tags.push(tag("vision", "pos", None));
        }
    }

    // Deaths, on the Rift only: ARAM games have far more deaths for everyone.
    let deaths = avg(&rift, |r| r.deaths as f64);
    if rift_tags && rift.len() >= 5 && deaths >= 7.5 {
        tags.push(tag("diesALot", "neg", None));
    } else if rift_tags && rift.len() >= 8 && deaths < 3.5 {
        tags.push(tag("safe", "pos", None));
    }

    // Streak (most recent first).
    if let Some(first) = played.first() {
        let n = played.iter().take_while(|r| r.win == first.win).count();
        if n >= 3 {
            tags.push(if first.win { tag("winStreak", "pos", Some(json!(n))) } else { tag("lossStreak", "neg", Some(json!(n))) });
        }
    }

    // Rank history.
    if let (Some(peak), Some(solo)) = (&ranked.prev_peak, &ranked.solo) {
        let now = TierDiv { tier: solo.tier.clone(), division: solo.division.clone() };
        if peak.score() - now.score() >= 8 {
            tags.push(tag("wasHigher", "info", Some(json!(peak.tier))));
        }
    }

    if let Some(last) = card.last_game {
        let days = (ctx.now_ms - last) / 86_400_000;
        if ctx.now_ms > 0 && days >= 30 {
            tags.push(tag("inactive", "", Some(json!(days))));
        }
    }

    card.tags = tags;
    card.summoner = summoner;
    card.ranked = ranked;
    card
}

/// Players who were on the same team in 2+ of their recent games. Returns, for each puuid,
/// a group number (1, 2...) shared by its premade partners.
pub fn premade_groups(players: &[(String, i32, Vec<GameRow>)]) -> HashMap<String, u32> {
    // players: (puuid, current team, history)
    let n = players.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(p: &mut [usize], x: usize) -> usize {
        let mut r = x;
        while p[r] != r {
            r = p[r];
        }
        let mut c = x;
        while p[c] != r {
            let next = p[c];
            p[c] = r;
            c = next;
        }
        r
    }
    let maps: Vec<HashMap<u64, i32>> = players.iter().map(|(_, _, h)| h.iter().map(|r| (r.game_id, r.team_id)).collect()).collect();
    for a in 0..n {
        for b in (a + 1)..n {
            if players[a].1 != players[b].1 {
                continue;
            }
            let together = maps[a].iter().filter(|(g, t)| maps[b].get(g) == Some(t)).count();
            if together >= 2 {
                let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
                parent[ra] = rb;
            }
        }
    }
    let mut sizes: HashMap<usize, u32> = HashMap::new();
    for i in 0..n {
        let r = find(&mut parent, i);
        *sizes.entry(r).or_default() += 1;
    }
    let mut ids: HashMap<usize, u32> = HashMap::new();
    let mut out = HashMap::new();
    for (i, (puuid, _, _)) in players.iter().enumerate() {
        let r = find(&mut parent, i);
        if sizes[&r] < 2 {
            continue;
        }
        let next = ids.len() as u32 + 1;
        let id = *ids.entry(r).or_insert(next);
        out.insert(puuid.clone(), id);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Rank;

    /// A game in a role: jungle has smite, support a support item, other roles a lane label
    /// (champions without Riot position data fall back to it).
    fn row(id: u64, champ: i32, win: bool, role: &str) -> GameRow {
        let (lane, spells, items) = match role {
            "jungle" => ("JUNGLE", [4, 11], vec![]),
            "support" => ("BOTTOM", [4, 14], vec![3877]),
            "top" => ("TOP", [4, 12], vec![]),
            "mid" => ("MIDDLE", [4, 14], vec![]),
            _ => ("BOTTOM", [4, 7], vec![]),
        };
        GameRow {
            game_id: id,
            created: 1_000_000 - id as i64,
            duration: 1800,
            queue_id: 420,
            map_id: 11,
            mode: "CLASSIC".into(),
            champion_id: champ,
            team_id: 100,
            win,
            kills: 5,
            deaths: 5,
            assists: 5,
            cs: 180,
            damage: 15000,
            taken: 20000,
            vision: 20,
            lane: lane.into(),
            spells,
            items,
            ..Default::default()
        }
    }

    fn keys(c: &PlayerCard) -> Vec<&str> {
        c.tags.iter().map(|t| t.key.as_str()).collect()
    }

    fn summ(level: u32) -> Summoner {
        Summoner { level, ..Default::default() }
    }

    #[test]
    fn one_trick_main_role_and_streak() {
        let mut h: Vec<GameRow> = (0..12).map(|i| row(i, 64, i % 2 == 0, "jungle")).collect();
        h[0].win = true;
        h[1].win = true;
        h[2].win = true;
        h[3].win = false;
        let ctx = Context { champion_id: Some(64), position: Some("JUNGLE".into()), mode: Some("CLASSIC".into()), now_ms: 1_000_000, ..Default::default() };
        let c = build_card(summ(300), Ranked::default(), &h, &[], &ctx);
        let k = keys(&c);
        assert!(k.contains(&"oneTrick"), "{k:?}");
        assert!(k.contains(&"mainRole"));
        assert!(k.contains(&"winStreak"));
        assert!(!k.contains(&"offRole"));
        assert!(!k.contains(&"firstTime"));
        assert_eq!(c.champ.as_ref().unwrap().games, 12);
        assert_eq!(c.recent.len(), 10);
    }

    #[test]
    fn first_time_and_off_role() {
        crate::positions::set((1..=4).map(|c| (c, vec!["mid".to_string()])).collect());
        let h: Vec<GameRow> = (0..12).map(|i| row(i, 1 + (i as i32 % 4), true, "mid")).collect();
        let ctx = Context { champion_id: Some(99), position: Some("UTILITY".into()), mode: None, now_ms: 0, ..Default::default() };
        let c = build_card(summ(300), Ranked::default(), &h, &[], &ctx);
        let k = keys(&c);
        assert!(k.contains(&"firstTime"));
        assert!(k.contains(&"offRole"));
        assert!(!k.contains(&"oneTrick"));
    }

    #[test]
    fn mastery_makes_expert() {
        let h: Vec<GameRow> = (0..3).map(|i| row(i, 7, false, "mid")).collect();
        let m = [Mastery { champion_id: 7, level: 40, points: 450_000, last_play: 0 }];
        let ctx = Context { champion_id: Some(7), ..Default::default() };
        let c = build_card(summ(300), Ranked::default(), &h, &m, &ctx);
        assert!(keys(&c).contains(&"expert"));
        assert_eq!(c.champ.unwrap().mastery_points, 450_000);
    }

    #[test]
    fn smurf_and_was_higher() {
        let ranked = Ranked {
            solo: Some(Rank { queue: "solo".into(), tier: "EMERALD".into(), division: "IV".into(), lp: 0, wins: 14, losses: 0 }),
            prev_peak: TierDiv::new("MASTER", ""),
            ..Default::default()
        };
        let h: Vec<GameRow> = (0..5).map(|i| row(i, 1, true, "top")).collect();
        let c = build_card(summ(42), ranked, &h, &[], &Context::default());
        let k = keys(&c);
        assert!(c.smurf);
        assert!(k.contains(&"smurf"));
        assert!(k.contains(&"wasHigher"));
        // A high level account at that rank is not a smurf.
        let ranked = Ranked { solo: Some(Rank { queue: "solo".into(), tier: "EMERALD".into(), division: "IV".into(), lp: 0, wins: 14, losses: 0 }), ..Default::default() };
        let c = build_card(summ(300), ranked, &h, &[], &Context::default());
        assert!(!c.smurf);
        // Exact ranked record (Riot API key): a hot solo queue run on a young account.
        let gold = |losses| Ranked {
            solo: Some(Rank { queue: "solo".into(), tier: "GOLD".into(), division: "II".into(), lp: 0, wins: 20, losses }),
            exact: true,
            ..Default::default()
        };
        assert!(build_card(summ(70), gold(8), &[], &[], &Context::default()).smurf);
        assert!(!build_card(summ(70), gold(18), &[], &[], &Context::default()).smurf);
    }

    #[test]
    fn remakes_ignored_and_dies_a_lot() {
        let mut h: Vec<GameRow> = (0..6).map(|i| row(i, 1, false, "top")).collect();
        for r in &mut h {
            r.deaths = 9;
        }
        h[0].remake = true;
        h[0].win = true;
        let c = build_card(summ(300), Ranked::default(), &h, &[], &Context::default());
        assert_eq!(c.games, 5);
        let k = keys(&c);
        assert!(k.contains(&"diesALot"));
        assert!(k.contains(&"lossStreak"));
        assert_eq!(c.tags.iter().find(|t| t.key == "lossStreak").unwrap().arg, Some(json!(5)));
    }

    /// Hextech ARAM games: lots of deaths, random champions.
    fn kiwi(id: u64, champ: i32, win: bool) -> GameRow {
        GameRow { mode: "KIWI".into(), queue_id: 2400, map_id: 12, deaths: 14, ..row(id, champ, win, "mid") }
    }

    #[test]
    fn hextech_aram_has_no_rift_or_death_tags() {
        let mut h: Vec<GameRow> = (0..20).map(|i| kiwi(i, 10 + (i as i32 % 7), i % 2 == 0)).collect();
        h.extend((20..30).map(|i| GameRow { deaths: 9, ..row(i, 64, false, "jungle") }));
        let ctx = Context { mode: Some("KIWI".into()), position: Some("UTILITY".into()), ..Default::default() };
        let c = build_card(summ(300), Ranked::default(), &h, &[], &ctx);
        let k = keys(&c);
        for rift_only in ["diesALot", "safe", "mainRole", "offRole", "farmer", "vision"] {
            assert!(!k.contains(&rift_only), "{rift_only} in {k:?}");
        }
        // Their profile (no game) still shows Rift habits, from Rift games only.
        let c = build_card(summ(300), Ranked::default(), &h, &[], &Context::default());
        let k = keys(&c);
        assert!(k.contains(&"diesALot"), "{k:?}");
        let only_kiwi: Vec<GameRow> = h.iter().filter(|r| r.mode == "KIWI").cloned().collect();
        let c = build_card(summ(300), Ranked::default(), &only_kiwi, &[], &Context::default());
        let k = keys(&c);
        assert!(!k.contains(&"diesALot"), "14 deaths a game is normal in Hextech ARAM: {k:?}");
    }

    #[test]
    fn style_win_rate_and_strong_champion() {
        let classes: HashMap<i32, String> =
            [(1, "tank"), (2, "tank"), (3, "tank"), (4, "mage"), (5, "poke")].map(|(c, k)| (c, k.to_string())).into();
        // 20 Hextech games: 12 on tanks, 13 wins, 6 of 7 won on champion 5.
        let h: Vec<GameRow> = (0..20)
            .map(|i| {
                let champ = if i < 12 { 1 + (i as i32 % 3) } else if i < 19 { 5 } else { 4 };
                let win = if champ == 5 { i != 18 } else { i % 2 == 0 };
                kiwi(i, champ, win)
            })
            .collect();
        let ctx = Context { mode: Some("KIWI".into()), classes: classes.into(), ..Default::default() };
        let c = build_card(summ(300), Ranked::default(), &h, &[], &ctx);
        let arg = |key: &str| c.tags.iter().find(|t| t.key == key).and_then(|t| t.arg.clone());
        assert_eq!(arg("style"), Some(json!("tank")), "{:?}", keys(&c));
        assert_eq!(arg("highWr"), Some(json!(60.0)));
        assert_eq!(arg("strongChamp"), Some(json!([5, 86.0])));
        assert!(!keys(&c).contains(&"lowWr"));
    }

    #[test]
    fn inactive() {
        let h = vec![row(1, 1, true, "top")];
        let ctx = Context { now_ms: h[0].created + 45 * 86_400_000, ..Default::default() };
        let c = build_card(summ(300), Ranked::default(), &h, &[], &ctx);
        assert!(keys(&c).contains(&"inactive"));
    }

    #[test]
    fn premades_need_two_shared_games_on_same_team() {
        let mk = |ids: &[u64], team: i32| ids.iter().map(|&id| GameRow { game_id: id, team_id: team, ..Default::default() }).collect::<Vec<_>>();
        let players = vec![
            ("a".to_string(), 100, mk(&[1, 2, 3], 100)),
            ("b".to_string(), 100, mk(&[1, 2, 9], 100)),
            ("c".to_string(), 100, mk(&[1, 7, 8], 100)),
            ("d".to_string(), 200, mk(&[1, 2, 3], 100)),
            ("e".to_string(), 200, mk(&[4, 5], 200)),
            ("f".to_string(), 200, mk(&[4, 5], 100)),
        ];
        let g = premade_groups(&players);
        assert_eq!(g.get("a"), g.get("b"));
        assert!(g.contains_key("a"));
        assert!(!g.contains_key("c"));
        assert!(!g.contains_key("d"));
        assert!(!g.contains_key("e"), "same games but different sides then");
    }
}

