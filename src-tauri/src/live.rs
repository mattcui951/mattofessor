//! The game's Live Client Data API (https://127.0.0.1:2999), served by the game itself once a
//! match has loaded. Used for summoner spells, items and the game clock (spell timers).

use serde::Serialize;
use serde_json::Value;
use std::time::Duration;

const URL: &str = "https://127.0.0.1:2999/liveclientdata/allgamedata";

fn client() -> &'static reqwest::Client {
    static C: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    C.get_or_init(|| {
        reqwest::Client::builder()
            // The game's own self-signed certificate, on 127.0.0.1 only.
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_millis(900))
            .build()
            .expect("http client")
    })
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LivePlayer {
    pub riot_id: String,
    /// Data Dragon key, e.g. "MonkeyKing".
    pub champion: String,
    pub team: String,
    pub level: u32,
    pub dead: bool,
    pub respawn: f64,
    /// Data Dragon spell keys, e.g. "SummonerFlash".
    pub spells: Vec<String>,
    pub items: Vec<i32>,
    pub me: bool,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InGame {
    pub game_time: f64,
    pub mode: String,
    pub my_team: String,
    pub players: Vec<LivePlayer>,
}

/// "game_character_displayname_MonkeyKing" -> "MonkeyKing"
fn champion_key(raw: &str) -> String {
    raw.rsplit('_').next().unwrap_or("").to_string()
}

/// "GeneratedTip_SummonerSpell_SummonerFlash_DisplayName" -> "SummonerFlash"
fn spell_key(raw: &str) -> String {
    raw.split('_').find(|p| p.starts_with("Summoner") && *p != "SummonerSpell").unwrap_or("").to_string()
}

pub fn parse(v: &Value) -> InGame {
    let s = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let me = s(v.get("activePlayer").unwrap_or(&Value::Null), "riotId");
    let players: Vec<LivePlayer> = v
        .get("allPlayers")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|p| {
                    let ss = p.get("summonerSpells").cloned().unwrap_or(Value::Null);
                    let spells = ["summonerSpellOne", "summonerSpellTwo"]
                        .iter()
                        .map(|k| spell_key(ss.pointer(&format!("/{k}/rawDisplayName")).and_then(Value::as_str).unwrap_or("")))
                        .collect();
                    let riot_id = s(p, "riotId");
                    LivePlayer {
                        me: !me.is_empty() && riot_id == me,
                        riot_id,
                        champion: champion_key(&s(p, "rawChampionName")),
                        team: s(p, "team"),
                        level: p.get("level").and_then(Value::as_u64).unwrap_or(0) as u32,
                        dead: p.get("isDead").and_then(Value::as_bool).unwrap_or(false),
                        respawn: p.get("respawnTimer").and_then(Value::as_f64).unwrap_or(0.0),
                        spells,
                        items: p
                            .get("items")
                            .and_then(Value::as_array)
                            .map(|a| a.iter().filter_map(|i| i.get("itemID").and_then(Value::as_i64)).map(|i| i as i32).collect())
                            .unwrap_or_default(),
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    let my_team = players.iter().find(|p| p.me).map(|p| p.team.clone()).unwrap_or_default();
    InGame {
        game_time: v.pointer("/gameData/gameTime").and_then(Value::as_f64).unwrap_or(0.0),
        mode: v.pointer("/gameData/gameMode").and_then(Value::as_str).unwrap_or("").to_string(),
        my_team,
        players,
    }
}

/// The game's full live data report, as it answered.
pub async fn fetch_raw() -> Option<Value> {
    let r = client().get(URL).send().await.ok()?;
    if !r.status().is_success() {
        return None;
    }
    r.json().await.ok()
}

pub async fn fetch() -> Option<InGame> {
    fetch_raw().await.as_ref().map(parse)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_players() {
        let v = json!({
            "activePlayer": {"riotId": "Me#NA1"},
            "gameData": {"gameTime": 312.5, "gameMode": "CLASSIC"},
            "allPlayers": [
                {"riotId": "Me#NA1", "rawChampionName": "game_character_displayname_MonkeyKing", "team": "ORDER", "level": 7,
                 "summonerSpells": {"summonerSpellOne": {"rawDisplayName": "GeneratedTip_SummonerSpell_SummonerFlash_DisplayName"},
                                    "summonerSpellTwo": {"rawDisplayName": "GeneratedTip_SummonerSpell_SummonerSmite_DisplayName"}},
                 "items": [{"itemID": 3158}]},
                {"riotId": "Foe#EUW", "rawChampionName": "game_character_displayname_Ahri", "team": "CHAOS", "isDead": true, "respawnTimer": 12.0,
                 "summonerSpells": {"summonerSpellOne": {"rawDisplayName": "GeneratedTip_SummonerSpell_SummonerDot_DisplayName"},
                                    "summonerSpellTwo": {"rawDisplayName": "GeneratedTip_SummonerSpell_SummonerFlash_DisplayName"}}}
            ]});
        let g = parse(&v);
        assert_eq!(g.game_time, 312.5);
        assert_eq!(g.my_team, "ORDER");
        assert_eq!(g.players[0].champion, "MonkeyKing");
        assert_eq!(g.players[0].spells, vec!["SummonerFlash", "SummonerSmite"]);
        assert_eq!(g.players[0].items, vec![3158]);
        assert!(g.players[0].me && !g.players[1].me);
        assert_eq!(g.players[1].spells[0], "SummonerDot");
        assert!(g.players[1].dead);
    }
}
