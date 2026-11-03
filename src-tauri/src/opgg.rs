//! Summoner's Rift statistics and builds from OP.GG's public MCP endpoint
//! (https://mcp-api.op.gg/mcp, github.com/opgginc/opgg-mcp): lane tier lists (win, pick and ban
//! rates) and per-champion builds. No key; answers are cached locally (tier lists 12 hours,
//! builds a day) and the last copy is used when OP.GG cannot be reached.
//!
//! The endpoint speaks MCP over HTTP (JSON-RPC). Tool results come as text in a compact
//! constructor format, `class Top: champion,play` headers then `Top("Malphite",173872)`
//! values, which `parse` turns into JSON objects.

use crate::store;
use anyhow::{anyhow, bail, Context as _, Result};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

const ENDPOINT: &str = "https://mcp-api.op.gg/mcp";
const TIERS_TTL_MS: i64 = 12 * 3_600_000;
const BUILD_TTL_MS: i64 = 24 * 3_600_000;
pub const POSITIONS: [&str; 5] = ["top", "jungle", "mid", "adc", "support"];

static SESSION: Mutex<Option<String>> = Mutex::new(None);
/// One request at a time keeps us polite and the session simple.
static GATE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn client() -> reqwest::Client {
    reqwest::Client::builder().timeout(Duration::from_secs(25)).build().expect("http")
}

/// The JSON-RPC message of a reply, which is plain JSON or a server-sent event.
fn rpc_body(text: &str) -> Result<Value> {
    let json = text.lines().find_map(|l| l.strip_prefix("data:")).map(str::trim).unwrap_or(text.trim());
    serde_json::from_str(json).context("unreadable OP.GG reply")
}

async fn post(body: &Value, session: Option<&str>) -> Result<(Option<String>, Value)> {
    let mut req = client()
        .post(ENDPOINT)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .json(body);
    if let Some(s) = session {
        req = req.header("Mcp-Session-Id", s);
    }
    let resp = req.send().await.context("OP.GG not reachable")?;
    let status = resp.status();
    let sid = resp.headers().get("mcp-session-id").and_then(|v| v.to_str().ok()).map(str::to_string);
    let text = resp.text().await?;
    if !status.is_success() {
        bail!("OP.GG answered {status}");
    }
    if text.trim().is_empty() {
        return Ok((sid, Value::Null));
    }
    Ok((sid, rpc_body(&text)?))
}

async fn session() -> Result<String> {
    if let Some(s) = SESSION.lock().clone() {
        return Ok(s);
    }
    let init = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"protocolVersion": "2025-03-26", "capabilities": {}, "clientInfo": {"name": "Mattofessor", "version": env!("CARGO_PKG_VERSION")}}});
    let (sid, _) = post(&init, None).await?;
    let sid = sid.ok_or_else(|| anyhow!("OP.GG gave no session"))?;
    post(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}), Some(&sid)).await?;
    *SESSION.lock() = Some(sid.clone());
    Ok(sid)
}

/// Calls one tool and returns its parsed result.
async fn call(tool: &str, args: Value) -> Result<Value> {
    let _gate = GATE.lock().await;
    let body = json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": tool, "arguments": args}});
    for attempt in 0..2 {
        let sid = session().await?;
        match post(&body, Some(&sid)).await {
            Ok((_, reply)) => {
                if let Some(e) = reply.get("error") {
                    bail!("OP.GG: {}", e.get("message").and_then(Value::as_str).unwrap_or("error"));
                }
                let result = reply.get("result").ok_or_else(|| anyhow!("OP.GG gave no result"))?;
                let text = result.pointer("/content/0/text").and_then(Value::as_str).unwrap_or_default();
                if result.get("isError").and_then(Value::as_bool).unwrap_or(false) {
                    bail!("OP.GG: {}", text.chars().take(200).collect::<String>());
                }
                return parse(text);
            }
            // An expired session: start a new one once.
            Err(e) if attempt == 0 => {
                log::info!("OP.GG retry with a new session: {e:#}");
                *SESSION.lock() = None;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}

/// A cached tool call: fresh copy when younger than `ttl`, else asks OP.GG and falls back to the
/// old copy when that fails.
async fn cached(key: &str, ttl: i64, tool: &str, args: Value) -> Result<Value> {
    let now = crate::players::now_ms();
    let st = store::get();
    let old: Option<(i64, Value)> = st.kv_get(key).and_then(|t| serde_json::from_str::<(i64, Value)>(&t).ok());
    if let Some((at, v)) = &old {
        if now - at < ttl {
            return Ok(v.clone());
        }
    }
    match call(tool, args).await {
        Ok(v) => {
            st.kv_set(key, &serde_json::to_string(&(now, &v))?);
            Ok(v)
        }
        Err(e) => match old {
            Some((_, v)) => {
                log::warn!("{tool}: {e:#}; using the last copy");
                Ok(v)
            }
            None => Err(e),
        },
    }
}

// ---------------------------------------------------------------------------------------------
// The constructor text format

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
    classes: &'a HashMap<String, Vec<String>>,
}

/// Turns a tool's text answer into JSON: each `Name(a, b)` becomes an object with the field names
/// its `class Name: x,y` header lists; lists, strings, numbers, True/False/None as usual.
pub fn parse(text: &str) -> Result<Value> {
    let mut classes = HashMap::new();
    let mut body_start = 0;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix("class ") {
            if let Some((name, fields)) = rest.split_once(':') {
                let fields = fields.split(',').map(|f| f.trim().to_string()).filter(|f| !f.is_empty()).collect();
                classes.insert(name.trim().to_string(), fields);
            }
            body_start = offset + line.len();
        } else if !l.is_empty() && body_start == offset {
            break;
        }
        offset += line.len();
    }
    let body = &text[body_start..];
    let mut p = Parser { s: body.as_bytes(), i: 0, classes: &classes };
    p.ws();
    if p.i >= p.s.len() {
        bail!("empty OP.GG answer");
    }
    let v = p.value()?;
    Ok(v)
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    fn expect(&mut self, c: u8) -> Result<()> {
        self.ws();
        if self.peek() == Some(c) {
            self.i += 1;
            Ok(())
        } else {
            bail!("expected '{}' at {}", c as char, self.i)
        }
    }

    fn value(&mut self) -> Result<Value> {
        self.ws();
        match self.peek() {
            Some(b'[') => {
                self.i += 1;
                let items = self.items(b']')?;
                Ok(Value::Array(items))
            }
            Some(q @ (b'"' | b'\'')) => self.string(q).map(Value::String),
            Some(c) if c == b'-' || c.is_ascii_digit() => self.number(),
            Some(c) if c.is_ascii_alphabetic() || c == b'_' => {
                let start = self.i;
                while self.peek().is_some_and(|c| c.is_ascii_alphanumeric() || c == b'_') {
                    self.i += 1;
                }
                let name = std::str::from_utf8(&self.s[start..self.i])?.to_string();
                self.ws();
                if self.peek() == Some(b'(') {
                    self.i += 1;
                    let args = self.items(b')')?;
                    let fields = self.classes.get(&name).cloned().unwrap_or_default();
                    let mut obj = Map::new();
                    for (k, v) in fields.into_iter().zip(args) {
                        obj.insert(k, v);
                    }
                    return Ok(Value::Object(obj));
                }
                Ok(match name.as_str() {
                    "True" | "true" => Value::Bool(true),
                    "False" | "false" => Value::Bool(false),
                    _ => Value::Null,
                })
            }
            other => bail!("unexpected {:?} at {}", other.map(|c| c as char), self.i),
        }
    }

    fn items(&mut self, close: u8) -> Result<Vec<Value>> {
        let mut out = Vec::new();
        self.ws();
        if self.peek() == Some(close) {
            self.i += 1;
            return Ok(out);
        }
        loop {
            out.push(self.value()?);
            self.ws();
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(c) if c == close => {
                    self.i += 1;
                    return Ok(out);
                }
                _ => self.expect(close).map(|_| ())?,
            }
        }
    }

    fn string(&mut self, quote: u8) -> Result<String> {
        self.i += 1;
        let mut out = Vec::new();
        while let Some(c) = self.peek() {
            self.i += 1;
            match c {
                b'\\' => {
                    let e = self.peek().ok_or_else(|| anyhow!("bad escape"))?;
                    self.i += 1;
                    match e {
                        b'n' => out.push(b'\n'),
                        b't' => out.push(b'\t'),
                        b'u' => {
                            let hex = std::str::from_utf8(self.s.get(self.i..self.i + 4).ok_or_else(|| anyhow!("bad \\u"))?)?;
                            let ch = char::from_u32(u32::from_str_radix(hex, 16)?).unwrap_or('\u{fffd}');
                            self.i += 4;
                            let mut buf = [0u8; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        }
                        other => out.push(other),
                    }
                }
                c if c == quote => return Ok(String::from_utf8(out)?),
                c => out.push(c),
            }
        }
        bail!("unterminated string")
    }

    fn number(&mut self) -> Result<Value> {
        let start = self.i;
        while self.peek().is_some_and(|c| c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.' | b'e' | b'E')) {
            self.i += 1;
        }
        let t = std::str::from_utf8(&self.s[start..self.i])?;
        if let Ok(n) = t.parse::<i64>() {
            return Ok(json!(n));
        }
        Ok(json!(t.parse::<f64>().with_context(|| format!("bad number {t}"))?))
    }
}

// ---------------------------------------------------------------------------------------------
// Tools

/// Champion key ("MonkeyKing") as the tools want it ("MONKEYKING").
pub fn tool_name(key: &str) -> String {
    key.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_uppercase()
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TierRow {
    pub champion_id: i32,
    pub position: String,
    /// OP.GG tier: 1 (OP) to 5; 0 when unknown.
    pub tier: i32,
    pub rank: i32,
    pub games: u64,
    pub win_rate: f64,
    pub pick_rate: f64,
    pub ban_rate: f64,
    /// Share of the champion's games played in this position.
    pub role_rate: f64,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tiers {
    pub rows: Vec<TierRow>,
    pub fetched_at: i64,
}

/// Rows of the lane tier list answer, with champion names (English) turned into ids.
pub fn tier_rows(v: &Value, ids: &HashMap<String, i32>) -> Vec<TierRow> {
    let mut out = Vec::new();
    for pos in POSITIONS {
        let rows = v.pointer(&format!("/data/positions/{pos}")).and_then(Value::as_array).cloned().unwrap_or_default();
        for r in rows {
            let name = r.get("champion").and_then(Value::as_str).unwrap_or_default();
            let Some(&champion_id) = ids.get(&norm(name)) else { continue };
            let f = |k: &str| r.get(k).and_then(Value::as_f64).unwrap_or(0.0);
            // OP.GG rounds rates to whole percents; the win count gives the exact rate.
            let win_rate = if f("play") > 0.0 && f("win") > 0.0 { f("win") / f("play") } else { f("win_rate") };
            out.push(TierRow {
                champion_id,
                position: pos.to_string(),
                tier: f("tier") as i32,
                rank: f("rank") as i32,
                games: f("play") as u64,
                win_rate,
                pick_rate: f("pick_rate"),
                ban_rate: f("ban_rate"),
                role_rate: f("role_rate"),
            });
        }
    }
    out
}

fn norm(name: &str) -> String {
    name.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase()
}

static TIERS: Mutex<Option<Arc<Tiers>>> = Mutex::new(None);

/// Every champion in every lane (ranked, all tiers).
pub async fn tiers() -> Result<Arc<Tiers>> {
    if let Some(t) = TIERS.lock().clone() {
        if crate::players::now_ms() - t.fetched_at < TIERS_TTL_MS {
            return Ok(t);
        }
    }
    let fields: Vec<String> = POSITIONS
        .iter()
        .map(|p| format!("data.positions.{p}[].{{champion,play,win,win_rate,pick_rate,ban_rate,role_rate,tier,rank}}"))
        .collect();
    let v = cached("opgg:tiers:v2", TIERS_TTL_MS, "lol_list_lane_meta_champions", json!({"position": "all", "lang": "en_US", "desired_output_fields": fields})).await?;
    let dd = crate::dd::get("en").await?;
    let ids: HashMap<String, i32> = dd.champions.iter().flat_map(|(id, c)| [(norm(&c.name), *id), (norm(&c.key), *id)]).collect();
    let rows = tier_rows(&v, &ids);
    if rows.is_empty() {
        bail!("OP.GG tier list was empty");
    }
    let t = Arc::new(Tiers { rows, fetched_at: crate::players::now_ms() });
    *TIERS.lock() = Some(t.clone());
    Ok(t)
}

/// "ranked" (Summoner's Rift) or "aram".
pub async fn analysis(champion_key: &str, position: &str, game_mode: &str) -> Result<Value> {
    let fields = [
        "data.summary.positions[].name",
        "data.summary.positions[].stats.{play,win_rate,role_rate,pick_rate,ban_rate}",
        "data.summary.average_stats.{play,win_rate,pick_rate,ban_rate}",
        "data.runes.{play,win,primary_page_id,primary_rune_ids[],secondary_page_id,secondary_rune_ids[],stat_mod_ids[]}",
        "data.summoner_spells.{ids[],play,win}",
        "data.starter_items.{ids[],play,win}",
        "data.boots.{ids[],play,win}",
        "data.core_items.{ids[],play,win}",
        "data.fourth_items[].{ids[],play,win}",
        "data.fifth_items[].{ids[],play,win}",
        "data.sixth_items[].{ids[],play,win}",
        "data.skill_masteries.{ids[],play,win}",
        "data.skills.{order[],play,win}",
    ];
    let name = tool_name(champion_key);
    let key = format!("opgg:build:{game_mode}:{name}:{position}");
    cached(&key, BUILD_TTL_MS, "lol_get_champion_analysis", json!({"game_mode": game_mode, "champion": name, "position": position, "lang": "en_US", "desired_output_fields": fields})).await
}

#[cfg(test)]
mod tests {
    use super::*;

    const TIERS: &str = "class LolListLaneMetaChampions: data\nclass Data: positions\nclass Positions: top,mid\nclass Top: champion,play,win_rate,pick_rate,ban_rate,tier,rank\nclass Mid: champion,play,win_rate,pick_rate,ban_rate,tier,rank\n\nLolListLaneMetaChampions(Data(Positions([Top(\"Malphite\",173872,0.52,0.07,0.18,1,1),Top(\"Kai'Sa\",10,0.5,0.01,0.0,3,40)],[Mid(\"Dr. Mundo\",500,0.49,0.01,0.0,4,60)])))";

    #[test]
    fn parses_constructor_text() {
        let v = parse(TIERS).unwrap();
        assert_eq!(v.pointer("/data/positions/top/0/champion").unwrap(), "Malphite");
        assert_eq!(v.pointer("/data/positions/top/0/play").unwrap(), 173872);
        assert_eq!(v.pointer("/data/positions/top/0/win_rate").unwrap().as_f64(), Some(0.52));
        assert_eq!(v.pointer("/data/positions/mid/0/champion").unwrap(), "Dr. Mundo");
    }

    #[test]
    fn parses_builds_and_escapes() {
        let text = "class LolGetChampionAnalysis: data\nclass Data: core_items,runes,flag,missing\nclass CoreItems: ids,ids_names,play,win,pick_rate\nclass Runes: primary_page_name,primary_rune_names\n\nLolGetChampionAnalysis(Data(CoreItems([3078,6610],[\"Trinity Force\",\"Sterak\\u0027s Gage\"],6514,3679,0.15),Runes('Precision',[\"Conqueror\"]),True,None))";
        let v = parse(text).unwrap();
        assert_eq!(v.pointer("/data/core_items/ids/1").unwrap(), 6610);
        assert_eq!(v.pointer("/data/core_items/ids_names/1").unwrap(), "Sterak's Gage");
        assert_eq!(v.pointer("/data/runes/primary_page_name").unwrap(), "Precision");
        assert_eq!(v.pointer("/data/flag").unwrap(), true);
        assert!(v.pointer("/data/missing").unwrap().is_null());
    }

    #[test]
    fn tier_rows_map_names_to_ids() {
        let v = parse(TIERS).unwrap();
        let ids = HashMap::from([(norm("Malphite"), 54), (norm("Kai'Sa"), 145), (norm("Dr. Mundo"), 36)]);
        let rows = tier_rows(&v, &ids);
        assert_eq!(rows.len(), 3);
        assert_eq!((rows[0].champion_id, rows[0].position.as_str(), rows[0].games), (54, "top", 173872));
        assert_eq!((rows[2].champion_id, rows[2].position.as_str()), (36, "mid"));
    }

    #[test]
    fn rpc_reply_from_event_stream() {
        let v = rpc_body("event: message\ndata: {\"result\": {\"x\": 1}}\n\n").unwrap();
        assert_eq!(v.pointer("/result/x").unwrap(), 1);
        assert_eq!(tool_name("MonkeyKing"), "MONKEYKING");
        assert_eq!(tool_name("KSante"), "KSANTE");
    }
}
