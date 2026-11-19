//! Champion search keywords from Tencent's official hero list (the lol.qq.com champion
//! database): nicknames players use ("大嘴" for Kog'Maw, "剑魔" for Aatrox), initials and pinyin.
//! The list changes only when champions are added or renamed: a copy is used for a week, and up
//! to 90 days old it still shows at once while a new one downloads.

use anyhow::{bail, Result};
use serde_json::Value;
use std::collections::BTreeMap;
use std::time::Duration;

const URL: &str = "https://game.gtimg.cn/images/lol/act/img/js/heroList/hero_list.js";
const TTL: crate::store::Ttl = crate::store::Ttl { fresh_ms: 7 * crate::store::DAY_MS, stale_ms: 90 * crate::store::DAY_MS };

/// Champion id to its keywords.
pub type Keywords = BTreeMap<i32, Vec<String>>;

pub fn parse(v: &Value) -> Keywords {
    let mut out = Keywords::new();
    for h in v.get("hero").and_then(Value::as_array).into_iter().flatten() {
        let Some(id) = h.get("heroId").and_then(Value::as_str).and_then(|s| s.parse::<i32>().ok()) else { continue };
        let mut words: Vec<String> = Vec::new();
        for w in h.get("keywords").and_then(Value::as_str).unwrap_or_default().split([',', '，']) {
            let w = w.trim();
            if !w.is_empty() && !words.iter().any(|x| x == w) {
                words.push(w.to_string());
            }
        }
        if !words.is_empty() {
            out.insert(id, words);
        }
    }
    out
}

pub async fn all() -> Result<Keywords> {
    let v = crate::store::cached("champion-keywords", TTL, "champion keywords", async {
        let r = crate::net::http().get(URL).timeout(Duration::from_secs(30)).send().await?;
        if !r.status().is_success() {
            bail!("champion keywords answered {}", r.status());
        }
        Ok(r.json().await?)
    })
    .await?;
    Ok(parse(&v))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_keywords() {
        let v: Value = serde_json::from_str(r#"{"hero": [{"heroId": "96", "keywords": "深渊巨口,克格莫,大嘴,KogMaw,dz,大嘴"}, {"heroId": "x"}, {"heroId": "1", "keywords": ""}]}"#).unwrap();
        let k = parse(&v);
        assert_eq!(k[&96], ["深渊巨口", "克格莫", "大嘴", "KogMaw", "dz"]);
        assert_eq!(k.len(), 1);
    }
}
