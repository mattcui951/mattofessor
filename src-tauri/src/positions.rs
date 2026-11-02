//! Which role a game was played in.
//!
//! The client's own lane labels are unreliable (one Challenger Riven was labeled mid, jungle,
//! bot and "none" across a week of top lane games), so roles come from, in order:
//!   1. smite: jungle (certain)
//!   2. a support quest item: support (certain)
//!   3. the champion's laning positions from Riot (`/lol-perks/v1/recommended-champion-positions`):
//!      one lane is certain; several make a group like "mid+top" (or a guess from the lane label
//!      when one lane must be named).

use crate::lcu::Lcu;
use crate::model::GameRow;
use parking_lot::RwLock;
use serde_json::Value;
use std::collections::HashMap;

/// Champion id -> laning roles ("top", "mid", "adc"), sorted.
static LANES: RwLock<Option<HashMap<i32, Vec<String>>>> = RwLock::new(None);

const SMITE: i32 = 11;
const SUPPORT_ITEMS: [i32; 8] = [3865, 3866, 3867, 3869, 3870, 3871, 3876, 3877];

fn lane_role(p: &str) -> Option<&'static str> {
    match p {
        "TOP" => Some("top"),
        "MIDDLE" | "MID" => Some("mid"),
        "BOTTOM" | "BOT" => Some("adc"),
        _ => None,
    }
}

pub fn parse(v: &Value) -> HashMap<i32, Vec<String>> {
    let mut out = HashMap::new();
    if let Some(m) = v.as_object() {
        for (id, e) in m {
            let Ok(id) = id.parse::<i32>() else { continue };
            let mut lanes: Vec<String> = e
                .get("recommendedPositions")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_str).filter_map(lane_role).map(str::to_string).collect())
                .unwrap_or_default();
            lanes.sort();
            lanes.dedup();
            out.insert(id, lanes);
        }
    }
    out
}

/// Load once per client session (cheap; about 9 KB).
pub async fn load(lcu: &Lcu) {
    if LANES.read().is_some() {
        return;
    }
    match lcu.get_value("/lol-perks/v1/recommended-champion-positions").await {
        Ok(v) => *LANES.write() = Some(parse(&v)),
        Err(e) => log::warn!("champion positions: {e:#}"),
    }
}

#[cfg(test)]
pub fn set(map: HashMap<i32, Vec<String>>) {
    *LANES.write() = Some(map);
}

pub struct Role {
    /// "top" | "jungle" | "mid" | "adc" | "support", or a group like "mid+top"; "" when unknown.
    pub group: String,
    /// A single role when one must be named (best guess for groups).
    pub single: String,
    pub sure: bool,
}

fn role_with(lanes: Option<&Vec<String>>, r: &GameRow) -> Role {
    let known = |g: &str, sure| Role { group: g.into(), single: g.into(), sure };
    if r.spells.contains(&SMITE) {
        return known("jungle", true);
    }
    if r.items.iter().any(|i| SUPPORT_ITEMS.contains(i)) {
        return known("support", true);
    }
    let label = lane_role(&r.lane).unwrap_or("");
    match lanes {
        Some(l) if l.len() == 1 => known(&l[0], true),
        Some(l) if !l.is_empty() => {
            let single = if l.iter().any(|x| x == label) { label.to_string() } else { l[0].clone() };
            Role { group: l.join("+"), single, sure: false }
        }
        _ => known(label, false),
    }
}

/// Role of one of a player's Summoner's Rift games.
pub fn role_of(r: &GameRow) -> Role {
    let guard = LANES.read();
    role_with(guard.as_ref().and_then(|m| m.get(&r.champion_id)), r)
}

/// Does a (possibly grouped) role include `role`?
pub fn group_has(group: &str, role: &str) -> bool {
    group.split('+').any(|g| g == role)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn roles() {
        let lanes = parse(&json!({
            "92": {"recommendedPositions": ["MIDDLE", "TOP"]},
            "222": {"recommendedPositions": ["BOTTOM"]},
            "64": {"recommendedPositions": ["JUNGLE"]},
            "99": {"recommendedPositions": ["MIDDLE", "UTILITY"]}
        }));
        let row = |champ, lane: &str, spells: [i32; 2], items: Vec<i32>| GameRow { champion_id: champ, lane: lane.into(), spells, items, ..Default::default() };
        let r = role_with(lanes.get(&92), &row(92, "JUNGLE", [4, 14], vec![]));
        assert_eq!((r.group.as_str(), r.single.as_str(), r.sure), ("mid+top", "mid", false));
        let r = role_with(lanes.get(&92), &row(92, "TOP", [4, 12], vec![]));
        assert_eq!(r.single, "top");
        let r = role_with(lanes.get(&222), &row(222, "MIDDLE", [4, 7], vec![]));
        assert_eq!((r.group.as_str(), r.sure), ("adc", true));
        let r = role_with(lanes.get(&64), &row(64, "MIDDLE", [4, 11], vec![]));
        assert_eq!((r.group.as_str(), r.sure), ("jungle", true));
        let r = role_with(lanes.get(&99), &row(99, "MIDDLE", [4, 14], vec![3866]));
        assert_eq!((r.group.as_str(), r.sure), ("support", true));
        let r = role_with(lanes.get(&99), &row(99, "NONE", [4, 14], vec![]));
        assert_eq!((r.group.as_str(), r.sure), ("mid", true), "UTILITY is not a lane");
        assert!(group_has("mid+top", "top") && !group_has("mid+top", "adc"));
    }
}
