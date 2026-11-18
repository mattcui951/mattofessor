//! What the admin writes by hand and every copy of the app follows: Hextech ARAM builds and the
//! item and champion tier lists. It lives in one file, `content.json` on this repository's
//! `content` branch. Every app checks it every 3 minutes, keeps the last copy and starts from
//! the one it shipped with (`data/content.json`), whichever is newer.
//!
//! The admin's edits are kept as a draft (`content-draft.json`) and published a few seconds after
//! the last one, through the GitHub CLI (`gh`) the admin is signed in to. A failed publish keeps
//! the draft until the next try. Only a copy whose GitHub CLI account can push to this repository
//! is the admin's; everyone else never sees the editing tools.

use anyhow::{anyhow, bail, Context as _, Result};
use base64::Engine as _;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tauri::Emitter;

use crate::branch::REPO;
const FILE: &str = "content.json";
const CHECK_EVERY: Duration = Duration::from_secs(3 * 60);
/// Edits close together go out as one publish.
const PUBLISH_AFTER: Duration = Duration::from_secs(4);
pub const TIERS: [&str; 5] = ["S", "A", "B", "C", "D"];
/// Champion classes: Data Dragon's champion tags.
pub const CLASSES: [&str; 6] = ["Fighter", "Assassin", "Mage", "Tank", "Support", "Marksman"];
/// Skill brackets with their own champion tiers: AramHex's 娱乐, 中分 and 前 5%.
pub const BRACKETS: [&str; 3] = ["casual", "mid", "high"];

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct MayhemBuild {
    pub starters: Vec<i32>,
    /// Items in the order to buy them (boots where they belong).
    pub order: Vec<i32>,
    /// Swaps for some games.
    pub situational: Vec<i32>,
    pub note: String,
    /// Day of the last edit (YYYY-MM-DD).
    pub updated: String,
}

/// Tier ("S" to "D") to ids, best first.
pub type Tiers = BTreeMap<String, Vec<i32>>;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Content {
    /// When it last changed (RFC 3339, UTC); the newest copy wins.
    pub updated: String,
    /// Hextech ARAM builds by champion id.
    pub builds: BTreeMap<i32, MayhemBuild>,
    pub item_tiers: Tiers,
    /// Bracket (one of `BRACKETS`) to class (one of `CLASSES`) to its tiers.
    #[serde(rename = "championTiersByBracket")]
    pub champion_tiers: BTreeMap<String, BTreeMap<String, Tiers>>,
    /// What apps before 1.0.13 read: class to tiers for all players. Passed on unchanged.
    #[serde(rename = "championTiers", skip_serializing_if = "serde_json::Value::is_null")]
    pub old_champion_tiers: serde_json::Value,
    /// Fields a newer app version added: passed on as they are, so an older copy publishing an
    /// edit never drops them (1.0.12 dropped the per-bracket champion tiers that way).
    #[serde(flatten)]
    pub other: serde_json::Map<String, serde_json::Value>,
}

/// One admin edit.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Edit {
    /// `build: None` removes the champion's build.
    Build { champion_id: i32, build: Option<MayhemBuild> },
    ItemTiers { tiers: Tiers },
    ChampionTiers { bracket: String, class: String, tiers: Tiers },
}

/// Keeps known tiers only, each id once (the first place wins), and drops empty tiers.
fn clean(tiers: Tiers) -> Tiers {
    let mut seen = std::collections::HashSet::new();
    TIERS
        .iter()
        .filter_map(|t| {
            let ids: Vec<i32> = tiers.get(*t)?.iter().copied().filter(|id| seen.insert(*id)).collect();
            (!ids.is_empty()).then(|| (t.to_string(), ids))
        })
        .collect()
}

impl Content {
    fn apply(&mut self, edit: Edit) -> Result<()> {
        match edit {
            Edit::Build { champion_id, build } => match build.filter(|b| !(b.starters.is_empty() && b.order.is_empty())) {
                Some(mut b) => {
                    b.updated = chrono::Local::now().format("%Y-%m-%d").to_string();
                    self.builds.insert(champion_id, b);
                }
                None => {
                    self.builds.remove(&champion_id);
                }
            },
            Edit::ItemTiers { tiers } => self.item_tiers = clean(tiers),
            Edit::ChampionTiers { bracket, class, tiers } => {
                if !BRACKETS.contains(&bracket.as_str()) {
                    bail!("unknown bracket {bracket}");
                }
                if !CLASSES.contains(&class.as_str()) {
                    bail!("unknown class {class}");
                }
                let tiers = clean(tiers);
                let classes = self.champion_tiers.entry(bracket.clone()).or_default();
                if tiers.is_empty() {
                    classes.remove(&class);
                } else {
                    classes.insert(class, tiers);
                }
                if classes.is_empty() {
                    self.champion_tiers.remove(&bracket);
                }
            }
        }
        self.updated = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        Ok(())
    }
}

// ---------------------------------------------------------------------------------------------
// Copies on disk

const SHIPPED: &str = include_str!("../data/content.json");

/// The repository branch: `content`, or `MATTOFESSOR_CONTENT_BRANCH` (to try publishing safely).
fn branch() -> String {
    std::env::var("MATTOFESSOR_CONTENT_BRANCH").ok().filter(|b| !b.is_empty()).unwrap_or_else(|| "content".into())
}

fn published_path() -> PathBuf {
    crate::paths::config_dir().join("content.json")
}

fn draft_path() -> PathBuf {
    crate::paths::config_dir().join("content-draft.json")
}

fn read(path: &PathBuf) -> Option<Content> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

fn write(path: &PathBuf, c: &Content) -> Result<()> {
    std::fs::write(path, serde_json::to_string_pretty(c)? + "\n")?;
    Ok(())
}

/// The newest published copy: the last one fetched or the shipped one.
fn published() -> Content {
    let shipped: Content = serde_json::from_str(SHIPPED).unwrap_or_default();
    match read(&published_path()) {
        Some(c) if c.updated >= shipped.updated => c,
        _ => shipped,
    }
}

static CURRENT: Mutex<Option<Content>> = Mutex::new(None);

/// The content in use: the admin's draft, else the newest published copy.
pub fn current() -> Content {
    CURRENT.lock().get_or_insert_with(|| draft().unwrap_or_else(published)).clone()
}

/// The admin's unpublished edits (none once this copy is known not to be the admin's).
fn draft() -> Option<Content> {
    if *ACCESS.lock() == Access::No {
        return None;
    }
    read(&draft_path())
}

pub fn build(champion_id: i32) -> Option<MayhemBuild> {
    current().builds.remove(&champion_id)
}

fn changed(c: Content) {
    *CURRENT.lock() = Some(c.clone());
    if let Some(core) = crate::core::get() {
        let _ = core.app.emit("content", c);
    }
}

// ---------------------------------------------------------------------------------------------
// Following the published copy

/// The published copy, unless its commit is the one read last time (`crate::branch`).
async fn fetch(seen: &mut Option<String>) -> Result<Option<Content>> {
    let Some(rev) = crate::branch::next(&branch(), seen).await else { return Ok(None) };
    let c = serde_json::from_slice(&crate::branch::file(&rev, FILE).await?).context("unreadable content.json")?;
    crate::branch::read_done(seen, &rev);
    Ok(Some(c))
}

/// Takes a newer published copy; the admin's draft stays in use until it is published.
fn receive(remote: Content) -> Result<()> {
    if remote.updated <= published().updated {
        return Ok(());
    }
    write(&published_path(), &remote)?;
    log::info!("content updated ({})", remote.updated);
    if draft().is_none() {
        changed(remote);
    }
    Ok(())
}

pub fn spawn() {
    tauri::async_runtime::spawn(async {
        loop {
            let a = tauri::async_runtime::spawn_blocking(check_access).await.unwrap_or(Access::Unknown);
            set_access(a);
            if a != Access::Unknown {
                break;
            }
            tokio::time::sleep(CHECK_EVERY).await;
        }
    });
    tauri::async_runtime::spawn(async {
        tokio::time::sleep(Duration::from_secs(3)).await;
        let mut seen: Option<String> = None;
        loop {
            match fetch(&mut seen).await {
                Ok(Some(c)) => {
                    if let Err(e) = receive(c) {
                        log::warn!("content: {e:#}");
                    }
                }
                Ok(None) => {}
                Err(e) => log::info!("content check: {e:#}"),
            }
            if can_edit() && draft_path().exists() && *PUBLISH.lock() == Publish::Failed {
                schedule_publish();
            }
            tokio::time::sleep(CHECK_EVERY).await;
        }
    });
}

// ---------------------------------------------------------------------------------------------
// Admin: who it is

#[derive(Clone, Copy, Debug, PartialEq)]
enum Access {
    /// Not asked yet, or GitHub could not answer.
    Unknown,
    Yes,
    No,
}

static ACCESS: Mutex<Access> = Mutex::new(Access::Unknown);

/// Whether this copy is the admin's: its GitHub CLI account can push to the repository.
pub fn can_edit() -> bool {
    *ACCESS.lock() == Access::Yes
}

/// Asks GitHub whether the GitHub CLI's account can push here. Without the CLI or signed in to
/// no account the answer is no; when GitHub cannot be reached it stays open for the next try.
fn check_access() -> Access {
    match gh_api(&[&format!("repos/{REPO}"), "--jq", ".permissions.push"], None) {
        Ok(push) if push == "true" => Access::Yes,
        Ok(_) => Access::No,
        Err(e) if e.is::<GhUnusable>() => {
            log::info!("admin check: {e}");
            Access::No
        }
        Err(e) => {
            log::info!("admin check: {e:#}");
            Access::Unknown
        }
    }
}

fn set_access(a: Access) {
    let was = std::mem::replace(&mut *ACCESS.lock(), a);
    log::info!("admin: {a:?}");
    if a == Access::No && was != Access::No && draft_path().exists() {
        // Edits left from when this copy could publish no longer apply.
        changed(published());
    }
    if let Some(core) = crate::core::get() {
        let _ = core.app.emit("content-status", status());
    }
}

// ---------------------------------------------------------------------------------------------
// Admin: edit and publish

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Publish {
    /// Nothing waits.
    Done,
    /// An edit waits to go out.
    Waiting,
    Publishing,
    Failed,
}

static PUBLISH: Mutex<Publish> = Mutex::new(Publish::Done);
static LAST_ERROR: Mutex<String> = Mutex::new(String::new());
/// Bumped by each edit, so only the last of a quick run publishes.
static EDITS: AtomicU64 = AtomicU64::new(0);
static PUBLISHING: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub publish: Publish,
    pub error: String,
    /// Whether this copy is the admin's (see `can_edit`).
    pub can_edit: bool,
}

pub fn status() -> Status {
    let mut publish = *PUBLISH.lock();
    if publish == Publish::Done && draft_path().exists() {
        publish = Publish::Waiting;
    }
    Status { publish, error: LAST_ERROR.lock().clone(), can_edit: can_edit() }
}

fn set_publish(p: Publish, error: String) {
    *PUBLISH.lock() = p;
    *LAST_ERROR.lock() = error;
    if let Some(core) = crate::core::get() {
        let _ = core.app.emit("content-status", status());
    }
}

/// Applies an admin edit to the draft and publishes it shortly; returns the content now in use.
pub fn edit(e: Edit) -> Result<Content> {
    if !can_edit() {
        bail!("Only the admin can edit");
    }
    let mut c = current();
    c.apply(e)?;
    write(&draft_path(), &c)?;
    changed(c.clone());
    schedule_publish();
    Ok(c)
}

pub fn schedule_publish() {
    if !can_edit() {
        return;
    }
    let n = EDITS.fetch_add(1, Ordering::SeqCst) + 1;
    set_publish(Publish::Waiting, String::new());
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(PUBLISH_AFTER).await;
        if EDITS.load(Ordering::SeqCst) == n {
            publish().await;
        }
    });
}

async fn publish() {
    let _one = PUBLISHING.lock().await;
    let Some(draft) = read(&draft_path()) else {
        set_publish(Publish::Done, String::new());
        return;
    };
    set_publish(Publish::Publishing, String::new());
    let result = tauri::async_runtime::spawn_blocking({
        let draft = draft.clone();
        move || put(&draft)
    })
    .await
    .map_err(|e| anyhow!("{e}"))
    .and_then(|r| r);
    match result {
        Ok(()) => {
            let _ = write(&published_path(), &draft);
            // A newer edit made while this one went out stays as the draft.
            if read(&draft_path()).as_ref() == Some(&draft) {
                let _ = std::fs::remove_file(draft_path());
            }
            log::info!("content published ({})", draft.updated);
            set_publish(if draft_path().exists() { Publish::Waiting } else { Publish::Done }, String::new());
        }
        Err(e) => {
            log::warn!("content publish: {e:#}");
            set_publish(Publish::Failed, format!("{e:#}"));
        }
    }
}

const GH_INSTALLED: &str = r"C:\Program Files\GitHub CLI\gh.exe";

/// The GitHub CLI, from PATH or where its installer puts it.
fn gh() -> std::process::Command {
    let exe = if which_gh() { PathBuf::from("gh") } else { PathBuf::from(GH_INSTALLED) };
    let mut cmd = std::process::Command::new(exe);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

fn which_gh() -> bool {
    std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join("gh.exe").is_file()))
}

/// The GitHub CLI cannot act for anyone here: not installed, or signed in to no account.
#[derive(Debug)]
struct GhUnusable(&'static str);

impl std::fmt::Display for GhUnusable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for GhUnusable {}

/// Runs `gh api` and returns its output; `input` goes to its standard input.
fn gh_api(args: &[&str], input: Option<&str>) -> Result<String> {
    use std::io::Write as _;
    use std::process::Stdio;
    let mut child = gh()
        .arg("api")
        .args(args)
        .stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| GhUnusable("GitHub CLI (gh) is not installed"))?;
    if let (Some(text), Some(mut stdin)) = (input, child.stdin.take()) {
        stdin.write_all(text.as_bytes())?;
    }
    let out = child.wait_with_output()?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        // Signed out, or a token GitHub no longer takes.
        if err.contains("auth login") || err.contains("HTTP 401") {
            return Err(GhUnusable("GitHub CLI is not signed in (run gh auth login)").into());
        }
        bail!("{}", err.trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Writes the content file on the branch (GitHub's contents API).
fn put(c: &Content) -> Result<()> {
    let branch = branch();
    // The current file's id: GitHub needs it to replace the file (none when it is new).
    let sha = gh_api(&[&format!("repos/{REPO}/contents/{FILE}?ref={branch}"), "--jq", ".sha"], None).ok();
    let body = serde_json::json!({
        "message": format!("Update content ({})", c.updated),
        "content": base64::engine::general_purpose::STANDARD.encode(serde_json::to_string_pretty(c)? + "\n"),
        "branch": branch,
        "sha": sha,
    });
    gh_api(&["-X", "PUT", &format!("repos/{REPO}/contents/{FILE}"), "--input", "-", "--jq", ".commit.sha"], Some(&body.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiers(rows: &[(&str, &[i32])]) -> Tiers {
        rows.iter().map(|(t, ids)| (t.to_string(), ids.to_vec())).collect()
    }

    #[test]
    fn tiers_keep_each_id_once_and_drop_empty_rows() {
        let t = clean(tiers(&[("S", &[1, 2]), ("A", &[2, 3]), ("B", &[]), ("Z", &[4])]));
        assert_eq!(t, tiers(&[("S", &[1, 2]), ("A", &[3])]));
    }

    #[test]
    fn edits_change_content_and_its_time() {
        let mut c = Content::default();
        let b = MayhemBuild { order: vec![3031], ..Default::default() };
        c.apply(Edit::Build { champion_id: 222, build: Some(b) }).unwrap();
        assert!(!c.updated.is_empty() && !c.builds[&222].updated.is_empty());
        c.apply(Edit::Build { champion_id: 222, build: Some(MayhemBuild::default()) }).unwrap();
        assert!(c.builds.is_empty(), "an empty build removes it");
        let mage = |bracket: &str, t: Tiers| Edit::ChampionTiers { bracket: bracket.into(), class: "Mage".into(), tiers: t };
        c.apply(mage("high", tiers(&[("S", &[1])]))).unwrap();
        c.apply(mage("casual", tiers(&[("D", &[1])]))).unwrap();
        assert_eq!(c.champion_tiers["high"]["Mage"]["S"], [1]);
        assert_eq!(c.champion_tiers["casual"]["Mage"]["D"], [1], "each bracket has its own");
        c.apply(mage("high", Tiers::new())).unwrap();
        assert!(!c.champion_tiers.contains_key("high"), "an empty bracket goes");
        assert!(c.apply(mage("all", Tiers::new())).is_err());
        assert!(c.apply(Edit::ChampionTiers { bracket: "mid".into(), class: "Jungler".into(), tiers: Tiers::new() }).is_err());
    }

    #[test]
    fn edits_read_from_the_page() {
        let e: Edit = serde_json::from_str(r#"{"kind": "championTiers", "bracket": "mid", "class": "Tank", "tiers": {"S": [1]}}"#).unwrap();
        assert!(matches!(e, Edit::ChampionTiers { bracket, class, .. } if bracket == "mid" && class == "Tank"));
        let e: Edit = serde_json::from_str(r#"{"kind": "build", "championId": 7, "build": null}"#).unwrap();
        assert!(matches!(e, Edit::Build { champion_id: 7, build: None }));
    }

    #[test]
    fn content_file_round_trips() {
        let shipped: Content = serde_json::from_str(SHIPPED).unwrap();
        let text = serde_json::to_string(&shipped).unwrap();
        assert_eq!(serde_json::from_str::<Content>(&text).unwrap(), shipped);
        let c: Content = serde_json::from_str(r#"{"builds": {"1": {"order": [3089]}}}"#).unwrap();
        assert_eq!(c.builds[&1].order, [3089]);
        assert!(!serde_json::to_string(&c).unwrap().contains(r#""championTiers""#), "no old key when there was none");
        // A field from a newer version survives a round trip through this one.
        let c: Content = serde_json::from_str(r#"{"updated": "x", "futureThing": {"a": [1]}}"#).unwrap();
        assert!(serde_json::to_string(&c).unwrap().contains(r#""futureThing":{"a":[1]}"#));
        // Older apps' all-players champion tiers are passed on as they are.
        let c: Content = serde_json::from_str(r#"{"championTiers": {"Mage": {"S": [1]}}, "championTiersByBracket": {"mid": {"Tank": {"A": [2]}}}}"#).unwrap();
        assert_eq!(c.champion_tiers["mid"]["Tank"]["A"], [2]);
        assert!(serde_json::to_string(&c).unwrap().contains(r#""championTiers":{"Mage":{"S":[1]}}"#));
    }
}
