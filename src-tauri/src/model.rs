//! Shapes read from the League client and sent to the UI.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One player's own line in one game, as returned by their match history list.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GameRow {
    pub game_id: u64,
    /// Unix ms.
    pub created: i64,
    /// Seconds.
    pub duration: u32,
    pub queue_id: i32,
    pub map_id: i32,
    pub mode: String,
    pub champion_id: i32,
    pub team_id: i32,
    pub win: bool,
    /// Ended in an early surrender (remake): left out of every statistic.
    pub remake: bool,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub cs: u32,
    pub gold: u32,
    pub damage: u32,
    pub taken: u32,
    pub vision: u32,
    pub champ_level: u32,
    pub items: Vec<i32>,
    pub spells: [i32; 2],
    pub perks: Vec<i32>,
    pub perk_primary: i32,
    pub perk_sub: i32,
    /// The client's lane label (unreliable on its own; see `positions`).
    #[serde(default)]
    pub lane: String,
    /// Arena placement (1-8), 0 elsewhere.
    pub placement: u32,
    pub augments: Vec<i32>,
    pub largest_multikill: u32,
}

impl GameRow {
    pub fn minutes(&self) -> f64 {
        (self.duration as f64 / 60.0).max(1.0)
    }

    pub fn is_rift(&self) -> bool {
        self.map_id == 11 && self.mode == "CLASSIC"
    }

    pub fn kda(&self) -> f64 {
        (self.kills + self.assists) as f64 / self.deaths.max(1) as f64
    }
}

fn i(v: &Value, k: &str) -> i64 {
    v.get(k).and_then(Value::as_i64).unwrap_or(0)
}

fn u(v: &Value, k: &str) -> u32 {
    i(v, k).max(0) as u32
}

/// Build a row from a game entry of `/lol-match-history/v1/products/lol/{puuid}/matches`
/// (`participants` then holds only that player) or from a full game for a given participant.
pub fn row_from_participant(game: &Value, p: &Value) -> GameRow {
    let s = p.get("stats").cloned().unwrap_or(Value::Null);
    let tl = p.get("timeline").cloned().unwrap_or(Value::Null);
    let items: Vec<i32> = (0..7).map(|n| i(&s, &format!("item{n}")) as i32).collect();
    let spells = [i(p, "spell1Id") as i32, i(p, "spell2Id") as i32];
    let perks: Vec<i32> = (0..6).map(|n| i(&s, &format!("perk{n}")) as i32).collect();
    let mode = game.get("gameMode").and_then(Value::as_str).unwrap_or("").to_string();
    let map_id = i(game, "mapId") as i32;
    let duration = u(game, "gameDuration");
    GameRow {
        game_id: i(game, "gameId") as u64,
        created: i(game, "gameCreation"),
        duration,
        queue_id: i(game, "queueId") as i32,
        map_id,
        mode,
        champion_id: i(p, "championId") as i32,
        team_id: i(p, "teamId") as i32,
        win: s.get("win").and_then(Value::as_bool).unwrap_or(false),
        remake: s.get("gameEndedInEarlySurrender").and_then(Value::as_bool).unwrap_or(false) || duration < 240,
        kills: u(&s, "kills"),
        deaths: u(&s, "deaths"),
        assists: u(&s, "assists"),
        cs: u(&s, "totalMinionsKilled") + u(&s, "neutralMinionsKilled"),
        gold: u(&s, "goldEarned"),
        damage: u(&s, "totalDamageDealtToChampions"),
        taken: u(&s, "totalDamageTaken"),
        vision: u(&s, "visionScore"),
        champ_level: u(&s, "champLevel"),
        items,
        spells,
        perks,
        perk_primary: i(&s, "perkPrimaryStyle") as i32,
        perk_sub: i(&s, "perkSubStyle") as i32,
        lane: tl.get("lane").and_then(Value::as_str).unwrap_or("").to_string(),
        placement: u(&s, "subteamPlacement"),
        augments: (1..=6).map(|n| i(&s, &format!("playerAugment{n}")) as i32).filter(|a| *a > 0).collect(),
        largest_multikill: u(&s, "largestMultiKill"),
    }
}

/// Rows from a match history list reply.
pub fn rows_from_history(v: &Value) -> Vec<GameRow> {
    v.pointer("/games/games")
        .and_then(Value::as_array)
        .map(|games| {
            games
                .iter()
                .filter_map(|g| {
                    let p = g.get("participants")?.as_array()?.first()?;
                    Some(row_from_participant(g, p))
                })
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Rank {
    pub queue: String,
    /// The client reports losses only for the signed-in player; 0 for everyone else.
    pub tier: String,
    pub division: String,
    pub lp: i32,
    pub wins: u32,
    pub losses: u32,
}

/// A tier with its division, ordered on one ladder (Iron IV = 0 ... Challenger = 36+).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TierDiv {
    pub tier: String,
    pub division: String,
}

const TIERS: [&str; 10] = ["IRON", "BRONZE", "SILVER", "GOLD", "PLATINUM", "EMERALD", "DIAMOND", "MASTER", "GRANDMASTER", "CHALLENGER"];

impl TierDiv {
    pub fn new(tier: &str, division: &str) -> Option<Self> {
        let t = tier.to_uppercase();
        TIERS.contains(&t.as_str()).then(|| Self { tier: t, division: division.to_string() })
    }

    pub fn score(&self) -> i32 {
        let t = TIERS.iter().position(|x| *x == self.tier).map(|p| p as i32).unwrap_or(-1);
        if t < 0 {
            return -1;
        }
        if t >= 7 {
            return 28 + (t - 7) * 4;
        }
        let d = match self.division.as_str() {
            "I" => 3,
            "II" => 2,
            "III" => 1,
            _ => 0,
        };
        t * 4 + d
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Ranked {
    pub solo: Option<Rank>,
    pub flex: Option<Rank>,
    /// Best of this season's peaks (solo or flex).
    pub season_peak: Option<TierDiv>,
    /// Last season's peak and final rank (solo, falling back to flex).
    pub prev_peak: Option<TierDiv>,
    pub prev_end: Option<TierDiv>,
    /// Wins and losses are exact (from the Riot API key); otherwise others' losses read 0.
    #[serde(default)]
    pub exact: bool,
}

fn rank_of(q: &Value, queue: &str) -> Option<Rank> {
    let tier = q.get("tier").and_then(Value::as_str).unwrap_or("");
    let td = TierDiv::new(tier, q.get("division").and_then(Value::as_str).unwrap_or(""))?;
    Some(Rank {
        queue: queue.into(),
        tier: td.tier,
        division: td.division,
        lp: i(q, "leaguePoints") as i32,
        wins: u(q, "wins"),
        losses: u(q, "losses"),
    })
}

fn best(a: Option<TierDiv>, b: Option<TierDiv>) -> Option<TierDiv> {
    match (a, b) {
        (Some(a), Some(b)) => Some(if b.score() > a.score() { b } else { a }),
        (a, b) => a.or(b),
    }
}

/// Parse `/lol-ranked/v1/ranked-stats/{puuid}`.
pub fn ranked_from(v: &Value) -> Ranked {
    let q = |name: &str| v.pointer(&format!("/queueMap/{name}")).cloned().unwrap_or(Value::Null);
    let solo_q = q("RANKED_SOLO_5x5");
    let flex_q = q("RANKED_FLEX_SR");
    let field = |q: &Value, t: &str, d: &str| {
        TierDiv::new(q.get(t).and_then(Value::as_str).unwrap_or(""), q.get(d).and_then(Value::as_str).unwrap_or(""))
    };
    let season_peak = best(field(&solo_q, "highestTier", "highestDivision"), field(&flex_q, "highestTier", "highestDivision"));
    let prev_peak = field(&solo_q, "previousSeasonHighestTier", "previousSeasonHighestDivision")
        .or_else(|| field(&flex_q, "previousSeasonHighestTier", "previousSeasonHighestDivision"));
    let prev_end = field(&solo_q, "previousSeasonEndTier", "previousSeasonEndDivision")
        .or_else(|| field(&flex_q, "previousSeasonEndTier", "previousSeasonEndDivision"));
    Ranked { solo: rank_of(&solo_q, "solo"), flex: rank_of(&flex_q, "flex"), season_peak, prev_peak, prev_end, exact: false }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mastery {
    pub champion_id: i32,
    pub level: u32,
    pub points: u64,
    pub last_play: i64,
}

pub fn masteries_from(v: &Value) -> Vec<Mastery> {
    v.as_array()
        .map(|a| {
            a.iter()
                .map(|m| Mastery {
                    champion_id: i(m, "championId") as i32,
                    level: u(m, "championLevel"),
                    points: i(m, "championPoints").max(0) as u64,
                    last_play: i(m, "lastPlayTime"),
                })
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Summoner {
    pub puuid: String,
    pub game_name: String,
    pub tag_line: String,
    pub level: u32,
    pub icon: i32,
    pub summoner_id: u64,
}

pub fn summoner_from(v: &Value) -> Summoner {
    Summoner {
        puuid: v.get("puuid").and_then(Value::as_str).unwrap_or("").into(),
        game_name: v.get("gameName").and_then(Value::as_str).unwrap_or("").into(),
        tag_line: v.get("tagLine").and_then(Value::as_str).unwrap_or("").into(),
        level: u(v, "summonerLevel"),
        icon: i(v, "profileIconId") as i32,
        summoner_id: i(v, "summonerId").max(0) as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tier_scores() {
        let s = |t, d| TierDiv::new(t, d).unwrap().score();
        assert_eq!(s("IRON", "IV"), 0);
        assert_eq!(s("GOLD", "I"), 15);
        assert_eq!(s("EMERALD", "IV"), 20);
        assert_eq!(s("MASTER", ""), 28);
        assert_eq!(s("CHALLENGER", ""), 36);
        assert!(TierDiv::new("NONE", "").is_none());
    }

    #[test]
    fn ranked_parse() {
        let v = json!({"queueMap": {
            "RANKED_SOLO_5x5": {"tier": "", "division": "NA", "highestTier": "", "previousSeasonHighestTier": "", "wins": 0, "losses": 0},
            "RANKED_FLEX_SR": {"tier": "GOLD", "division": "I", "leaguePoints": 74, "wins": 8, "losses": 1,
                "highestTier": "GOLD", "highestDivision": "I",
                "previousSeasonHighestTier": "PLATINUM", "previousSeasonHighestDivision": "IV",
                "previousSeasonEndTier": "GOLD", "previousSeasonEndDivision": "II"}}});
        let r = ranked_from(&v);
        assert!(r.solo.is_none());
        assert_eq!(r.flex.as_ref().unwrap().lp, 74);
        assert_eq!(r.prev_peak.unwrap().tier, "PLATINUM");
        assert_eq!(r.prev_end.unwrap().division, "II");
        assert_eq!(r.season_peak.unwrap().tier, "GOLD");
    }

    #[test]
    fn history_rows() {
        let v = json!({"games": {"games": [{
            "gameId": 5, "gameCreation": 1000, "gameDuration": 1800, "queueId": 420, "mapId": 11, "gameMode": "CLASSIC",
            "participants": [{"championId": 64, "teamId": 100, "spell1Id": 4, "spell2Id": 11,
                "stats": {"win": true, "kills": 7, "deaths": 2, "assists": 9, "totalMinionsKilled": 30, "neutralMinionsKilled": 150,
                    "item0": 3071, "perk0": 8010, "perkPrimaryStyle": 8000, "perkSubStyle": 8400},
                "timeline": {"lane": "JUNGLE", "role": "NONE"}}]}]}});
        let rows = rows_from_history(&v);
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        assert_eq!(r.lane, "JUNGLE");
        assert_eq!(r.cs, 180);
        assert!(r.win && !r.remake);
        assert_eq!(r.items[0], 3071);
        assert_eq!(r.perks[0], 8010);
    }
}
