//! Performance score (0.0-10.0) for every player in a finished game, and the MVP (best on the
//! winning team) and SVP (best on the losing team).
//!
//! Every stat is judged against the other players in the same lobby, so a score means "how
//! much this player mattered in this game", whatever the mode, patch or game length.
//!
//! Step 1. Per player, these measures (per minute where it matters):
//!      kill participation   (kills + assists) / team kills
//!      kda                  ln(1 + (kills + 0.65 x assists) / max(deaths, 1)), so 20/0 is not 10x 2/0
//!      damage share         damage to champions / team damage to champions
//!      frontline            (damage taken + mitigated) / team total, discounted by deaths
//!                           (soaking damage counts when you survive it, not when you feed)
//!      economy              CS per minute; supports (support item) count as average here
//!      vision               vision score per minute (Summoner's Rift only)
//!      objectives           damage to buildings and epic monsters, plus turrets / inhibitors taken
//!      crowd control        seconds of CC applied to enemies
//!      healing              healing done, only for players who healed allies
//!      deaths share         deaths / team deaths (counts against)
//! Step 2. Each measure becomes a z-score within the lobby (clipped to +-2.5), and the z-scores are
//!    combined with the weights below (mode-specific: no vision in ARAM, less farming).
//! Step 3. Bonuses: multikills, first blood; minus 0.12 per death beyond 1.5x the lobby's median
//!    deaths (so feeding costs extra in any mode); plus 0.6 for a win or minus 0.6 for a loss
//!    (Arena: by placement). The total is mapped onto 0-10 around 5.0 with a soft (tanh) curve, so the
//!    ends are approached but not piled up at, and rounded to one decimal.
//!
//! Remakes get no score.

use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerScore {
    pub participant_id: i64,
    pub puuid: String,
    pub score: f32,
    /// "mvp" | "svp" | "" (Arena and ties never get a badge twice).
    pub badge: String,
    /// 1 = best in the lobby.
    pub place: u32,
}

struct P {
    pid: i64,
    puuid: String,
    team: i64,
    win: bool,
    placement: f64,
    k: f64,
    d: f64,
    a: f64,
    dmg: f64,
    tank: f64,
    cs: f64,
    vision: f64,
    obj: f64,
    structures: f64,
    cc: f64,
    heal: f64,
    healed_allies: bool,
    support: bool,
    multis: [f64; 4],
    first_blood: bool,
}

#[derive(Clone, Copy)]
struct Weights {
    kp: f64,
    kda: f64,
    dmg: f64,
    tank: f64,
    econ: f64,
    vision: f64,
    obj: f64,
    cc: f64,
    heal: f64,
    deaths: f64,
}

const RIFT: Weights = Weights { kp: 0.14, kda: 0.14, dmg: 0.18, tank: 0.08, econ: 0.09, vision: 0.08, obj: 0.09, cc: 0.05, heal: 0.03, deaths: 0.13 };
const ARAM: Weights = Weights { kp: 0.14, kda: 0.15, dmg: 0.26, tank: 0.12, econ: 0.03, vision: 0.0, obj: 0.04, cc: 0.07, heal: 0.05, deaths: 0.14 };
const ARENA: Weights = Weights { kp: 0.10, kda: 0.22, dmg: 0.30, tank: 0.14, econ: 0.0, vision: 0.0, obj: 0.0, cc: 0.10, heal: 0.06, deaths: 0.08 };

const SUPPORT_ITEMS: [i64; 8] = [3865, 3866, 3867, 3869, 3870, 3871, 3876, 3877];

fn n(s: &Value, k: &str) -> f64 {
    s.get(k).and_then(|v| v.as_f64().or_else(|| v.as_bool().map(|b| b as u8 as f64))).unwrap_or(0.0)
}

/// z-scores within the lobby, clipped.
fn z(values: &[f64]) -> Vec<f64> {
    let len = values.len().max(1) as f64;
    let mean = values.iter().sum::<f64>() / len;
    let var = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / len;
    let sd = var.sqrt();
    if sd < 1e-9 {
        return vec![0.0; values.len()];
    }
    values.iter().map(|v| ((v - mean) / sd).clamp(-2.5, 2.5)).collect()
}

fn share(v: f64, total: f64) -> f64 {
    if total > 0.0 {
        v / total
    } else {
        0.0
    }
}

/// Scores for a full game from `/lol-match-history/v1/games/{id}`. Empty for remakes or
/// unreadable games.
pub fn score_game(game: &Value) -> Vec<PlayerScore> {
    let mode = game.get("gameMode").and_then(Value::as_str).unwrap_or("");
    let minutes = (n(game, "gameDuration") / 60.0).max(1.0);
    let parts = game.get("participants").and_then(Value::as_array).cloned().unwrap_or_default();
    let ids = game.get("participantIdentities").and_then(Value::as_array).cloned().unwrap_or_default();
    if parts.len() < 2 || n(game, "gameDuration") < 240.0 {
        return Vec::new();
    }
    let arena = parts.iter().any(|p| n(p.get("stats").unwrap_or(&Value::Null), "playerSubteamId") > 0.0) || mode == "CHERRY";
    let rift = mode == "CLASSIC" && n(game, "mapId") as i64 == 11;
    let weights = if arena {
        ARENA
    } else if rift {
        RIFT
    } else {
        ARAM
    };

    let ps: Vec<P> = parts
        .iter()
        .map(|p| {
            let s = p.get("stats").cloned().unwrap_or(Value::Null);
            let pid = n(p, "participantId") as i64;
            let puuid = ids
                .iter()
                .find(|i| n(i, "participantId") as i64 == pid)
                .and_then(|i| i.pointer("/player/puuid"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let items: Vec<i64> = (0..7).map(|i| n(&s, &format!("item{i}")) as i64).collect();
            P {
                pid,
                puuid,
                team: if arena { n(&s, "playerSubteamId") as i64 } else { n(p, "teamId") as i64 },
                win: n(&s, "win") > 0.0,
                placement: n(&s, "subteamPlacement"),
                k: n(&s, "kills"),
                d: n(&s, "deaths"),
                a: n(&s, "assists"),
                dmg: n(&s, "totalDamageDealtToChampions"),
                tank: (n(&s, "totalDamageTaken") + n(&s, "damageSelfMitigated")) / (1.0 + 0.15 * n(&s, "deaths")),
                cs: n(&s, "totalMinionsKilled") + n(&s, "neutralMinionsKilled"),
                vision: n(&s, "visionScore"),
                obj: n(&s, "damageDealtToTurrets") + 0.5 * n(&s, "damageDealtToObjectives"),
                structures: n(&s, "turretKills") + 1.5 * n(&s, "inhibitorKills"),
                cc: n(&s, "timeCCingOthers"),
                heal: n(&s, "totalHeal"),
                healed_allies: n(&s, "totalUnitsHealed") > 1.0,
                support: items.iter().any(|i| SUPPORT_ITEMS.contains(i)),
                multis: [n(&s, "doubleKills"), n(&s, "tripleKills"), n(&s, "quadraKills"), n(&s, "pentaKills")],
                first_blood: n(&s, "firstBloodKill") > 0.0,
            }
        })
        .collect();

    let team_sum = |team: i64, f: &dyn Fn(&P) -> f64| ps.iter().filter(|q| q.team == team).map(f).sum::<f64>();
    let col = |f: &dyn Fn(&P) -> f64| -> Vec<f64> { ps.iter().map(f).collect() };

    let kp = z(&col(&|p| share(p.k + p.a, team_sum(p.team, &|q| q.k))).iter().map(|v| v.min(1.0)).collect::<Vec<_>>());
    let kda = z(&col(&|p| (1.0 + (p.k + 0.65 * p.a) / p.d.max(1.0)).ln()));
    let dmg = z(&col(&|p| 0.6 * share(p.dmg, team_sum(p.team, &|q| q.dmg)) + 0.4 * p.dmg / minutes / 1000.0));
    let tank = z(&col(&|p| share(p.tank, team_sum(p.team, &|q| q.tank))));
    let vision = z(&col(&|p| p.vision / minutes));
    // Supports give up farm on purpose: neither rewarded nor punished for it (vision counts on its own).
    let cs_z = z(&col(&|p| p.cs / minutes));
    let econ: Vec<f64> = ps.iter().enumerate().map(|(i, p)| if p.support && rift { 0.0 } else { cs_z[i] }).collect();
    let obj = z(&col(&|p| p.obj / minutes / 1000.0 + 0.6 * p.structures));
    let cc = z(&col(&|p| p.cc / minutes));
    let heal = z(&col(&|p| if p.healed_allies { p.heal / minutes } else { 0.0 }));
    let deaths = z(&col(&|p| 0.7 * share(p.d, team_sum(p.team, &|q| q.d)) + 0.3 * p.d / minutes));

    let mut sorted_deaths: Vec<f64> = ps.iter().map(|p| p.d).collect();
    sorted_deaths.sort_by(f64::total_cmp);
    let feeding_line = (sorted_deaths[sorted_deaths.len() / 2] * 1.5).max(4.0);

    let w = weights;
    let total_w = w.kp + w.kda + w.dmg + w.tank + w.econ + w.vision + w.obj + w.cc + w.heal + w.deaths;
    let mut scored: Vec<(usize, f64)> = ps
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let raw = (w.kp * kp[i] + w.kda * kda[i] + w.dmg * dmg[i] + w.tank * tank[i] + w.econ * econ[i] + w.vision * vision[i]
                + w.obj * obj[i]
                + w.cc * cc[i]
                + w.heal * heal[i]
                - w.deaths * deaths[i])
                / total_w;
            let bonus = 0.12 * p.multis[0] + 0.25 * p.multis[1] + 0.45 * p.multis[2] + 0.8 * p.multis[3] + if p.first_blood { 0.1 } else { 0.0 };
            let feeding = 0.12 * (p.d - feeding_line).max(0.0);
            let result = if arena {
                // 1st: +0.8 ... 8th: -0.8
                if p.placement > 0.0 {
                    (4.5 - p.placement) / 3.5 * 0.8
                } else {
                    0.0
                }
            } else if p.win {
                0.6
            } else {
                -0.6
            };
            // Soft limits: great games approach 10 without piling up at the ceiling.
            let x = SCALE * raw + bonus.min(1.2) - feeding + result;
            (i, 5.0 + 4.95 * (x / 4.95).tanh())
        })
        .collect();

    // Places (1 = best), ties broken by damage.
    scored.sort_by(|a, b| b.1.total_cmp(&a.1).then(ps[b.0].dmg.total_cmp(&ps[a.0].dmg)));
    let mut out: Vec<PlayerScore> = scored
        .iter()
        .enumerate()
        .map(|(place, (i, s))| PlayerScore {
            participant_id: ps[*i].pid,
            puuid: ps[*i].puuid.clone(),
            score: ((s * 10.0).round() / 10.0) as f32,
            badge: String::new(),
            place: place as u32 + 1,
        })
        .collect();
    if !arena {
        let best_of = |winners: bool| scored.iter().position(|(i, _)| ps[*i].win == winners);
        if let Some(i) = best_of(true) {
            out[i].badge = "mvp".into();
        }
        if let Some(i) = best_of(false) {
            out[i].badge = "svp".into();
        }
    }
    out
}

/// Spread of the 0-10 scale: calibrated on real games so lobbies span roughly 2-9.5.
const SCALE: f64 = 4.2;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A 5v5 game where player 1 (blue) is clearly best, player 6 best on red, 10 worst.
    fn game(mode: &str, map: i64) -> Value {
        let mut parts = Vec::new();
        let mut ids = Vec::new();
        for pid in 1..=10 {
            let blue = pid <= 5;
            let star = pid == 1 || pid == 6;
            let worst = pid == 10;
            let stats = json!({
                "win": blue,
                "kills": if star { 12 } else if worst { 0 } else { 3 },
                "deaths": if star { 1 } else if worst { 11 } else { 5 },
                "assists": if star { 8 } else { 5 },
                "totalDamageDealtToChampions": if star { 42000 } else if worst { 6000 } else { 16000 },
                "totalDamageTaken": 18000, "damageSelfMitigated": 9000,
                "totalMinionsKilled": if star { 260 } else { 150 }, "neutralMinionsKilled": 0,
                "visionScore": 25, "damageDealtToTurrets": if star { 9000 } else { 2000 },
                "damageDealtToObjectives": 4000, "turretKills": if star { 3 } else { 0 },
                "timeCCingOthers": 20, "totalHeal": 3000, "totalUnitsHealed": 1,
                "doubleKills": if star { 2 } else { 0 }, "pentaKills": 0
            });
            parts.push(json!({"participantId": pid, "teamId": if blue { 100 } else { 200 }, "stats": stats}));
            ids.push(json!({"participantId": pid, "player": {"puuid": format!("p{pid}")}}));
        }
        json!({"gameMode": mode, "mapId": map, "gameDuration": 1800, "participants": parts, "participantIdentities": ids})
    }

    fn by(scores: &[PlayerScore], puuid: &str) -> PlayerScore {
        scores.iter().find(|s| s.puuid == puuid).unwrap().clone()
    }

    #[test]
    fn mvp_and_svp() {
        let s = score_game(&game("CLASSIC", 11));
        assert_eq!(s.len(), 10);
        assert_eq!(by(&s, "p1").badge, "mvp");
        assert_eq!(by(&s, "p6").badge, "svp");
        assert_eq!(s.iter().filter(|x| !x.badge.is_empty()).count(), 2);
        assert!(by(&s, "p1").score > by(&s, "p2").score);
        assert!(by(&s, "p10").score < by(&s, "p7").score);
        assert!(s.iter().all(|x| (0.0..=10.0).contains(&x.score)));
        // One decimal.
        assert!(s.iter().all(|x| ((x.score * 10.0).round() - x.score * 10.0).abs() < 1e-4));
        // Winning helps: the two stars played alike, the winner scores higher.
        assert!(by(&s, "p1").score > by(&s, "p6").score);
        assert_eq!(by(&s, "p1").place, 1);
    }

    #[test]
    fn aram_and_remakes() {
        let s = score_game(&game("ARAM", 12));
        assert_eq!(by(&s, "p1").badge, "mvp");
        let mut short = game("CLASSIC", 11);
        short["gameDuration"] = json!(200);
        assert!(score_game(&short).is_empty());
        assert!(score_game(&json!({})).is_empty());
    }

    #[test]
    fn arena_has_no_badges() {
        let mut g = game("CHERRY", 30);
        for (i, p) in g["participants"].as_array_mut().unwrap().iter_mut().enumerate() {
            p["stats"]["playerSubteamId"] = json!(i / 2 + 1);
            p["stats"]["subteamPlacement"] = json!(i / 2 + 1);
        }
        let s = score_game(&g);
        assert!(s.iter().all(|x| x.badge.is_empty()));
        assert!(by(&s, "p1").score > by(&s, "p10").score);
    }
}
