//! Files on a branch of this repository that every copy of the app follows: the admin's content
//! (`content`) and the refreshed game data (`data`). The branch's latest commit comes from
//! GitHub's API (cached up to a minute; 60 calls an hour per address), then each file is read at
//! that commit, so the raw CDN (which caches a branch's file up to 5 minutes) never hands back an
//! older copy than the commit. When the API cannot answer, the branch's files are read directly.

use anyhow::{bail, Context as _, Result};
use std::time::Duration;

pub const REPO: &str = "mattcui951/mattofessor";

/// The branch's latest commit.
pub async fn latest_commit(branch: &str) -> Result<String> {
    let url = format!("https://api.github.com/repos/{REPO}/commits/{branch}");
    let r = crate::net::http()
        .get(&url)
        .timeout(Duration::from_secs(30))
        .header("Accept", "application/vnd.github.sha")
        .header("User-Agent", "Mattofessor")
        .send()
        .await
        .context("GitHub not reachable")?;
    if !r.status().is_success() {
        bail!("GitHub API answered {}", r.status());
    }
    let sha = r.text().await?.trim().to_string();
    if sha.len() != 40 {
        bail!("GitHub gave no commit");
    }
    Ok(sha)
}

/// What to read next: the branch's new commit, nothing when it is the one already read (`seen`),
/// or the branch itself when the API cannot answer. Pass the result to `file`, and once every
/// file is read, `seen` takes it (`read_done`).
pub async fn next(branch: &str, seen: &Option<String>) -> Option<String> {
    match latest_commit(branch).await {
        Ok(sha) if seen.as_deref() == Some(sha.as_str()) => None,
        Ok(sha) => Some(sha),
        Err(e) => {
            log::info!("{branch} branch commit: {e:#}");
            Some(branch.to_string())
        }
    }
}

/// Remembers a commit whose files were all read (a branch name is not remembered).
pub fn read_done(seen: &mut Option<String>, rev: &str) {
    if rev.len() == 40 {
        *seen = Some(rev.to_string());
    }
}

/// A file at a commit (or branch).
pub async fn file(rev: &str, name: &str) -> Result<Vec<u8>> {
    let url = format!("https://raw.githubusercontent.com/{REPO}/{rev}/{name}");
    let r = crate::net::http().get(&url).timeout(Duration::from_secs(60)).send().await.context("GitHub not reachable")?;
    if !r.status().is_success() {
        bail!("{name}: GitHub answered {}", r.status());
    }
    Ok(r.bytes().await?.to_vec())
}
