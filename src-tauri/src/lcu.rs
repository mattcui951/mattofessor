//! League Client API (LCU): the HTTPS API the signed-in League client serves on localhost.
//!
//! The port and password come from the `lockfile` in the League install folder, which the
//! client writes while it runs (`LeagueClient:<pid>:<port>:<password>:https`). The install
//! folder is found from the running client's executable path. `MATTOFESSOR_LOCKFILE` points
//! at another lockfile (the mock client in `tools/`).

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine;
use parking_lot::RwLock;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

pub struct Lcu {
    http: reqwest::Client,
    base: String,
    auth: String,
    /// Caps concurrent requests so a full lobby of lookups never floods the client.
    gate: Semaphore,
}

/// HTTP status error, so callers can tell "not found" from "client unreachable".
#[derive(Debug)]
pub struct Status(pub u16, pub String);

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "League client returned {}: {}", self.0, self.1)
    }
}

impl std::error::Error for Status {}

pub fn is_not_found(e: &anyhow::Error) -> bool {
    e.downcast_ref::<Status>().is_some_and(|s| s.0 == 404)
}

impl Lcu {
    fn new(port: u16, password: &str) -> Self {
        let http = reqwest::Client::builder()
            // The client uses a self-signed Riot certificate, and only ever on 127.0.0.1.
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_secs(25))
            .build()
            .expect("http client");
        let auth = format!("Basic {}", base64::engine::general_purpose::STANDARD.encode(format!("riot:{password}")));
        Self { http, base: format!("https://127.0.0.1:{port}"), auth, gate: Semaphore::new(6) }
    }

    async fn request(&self, method: reqwest::Method, path: &str, body: Option<&Value>) -> Result<Value> {
        let _permit = self.gate.acquire().await?;
        let mut req = self.http.request(method, format!("{}{path}", self.base)).header("Authorization", &self.auth);
        if let Some(b) = body {
            req = req.json(b);
        }
        let resp = req.send().await.with_context(|| format!("League client not reachable ({path})"))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            let msg = serde_json::from_str::<Value>(&text)
                .ok()
                .and_then(|v| v.get("message").and_then(Value::as_str).map(str::to_string))
                .unwrap_or_else(|| text.chars().take(160).collect());
            return Err(Status(status.as_u16(), msg).into());
        }
        if text.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&text).with_context(|| format!("unexpected reply from {path}"))
    }

    pub async fn get_value(&self, path: &str) -> Result<Value> {
        self.request(reqwest::Method::GET, path, None).await
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let v = self.get_value(path).await?;
        serde_json::from_value(v).with_context(|| format!("unexpected shape from {path}"))
    }

    /// GET that maps 404 to `None` (e.g. no gameflow session).
    pub async fn get_opt(&self, path: &str) -> Result<Option<Value>> {
        match self.get_value(path).await {
            Ok(v) => Ok(Some(v)),
            Err(e) if is_not_found(&e) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub async fn post(&self, path: &str, body: &Value) -> Result<Value> {
        self.request(reqwest::Method::POST, path, Some(body)).await
    }

    pub async fn put(&self, path: &str, body: &Value) -> Result<Value> {
        self.request(reqwest::Method::PUT, path, Some(body)).await
    }

    pub async fn delete(&self, path: &str) -> Result<Value> {
        self.request(reqwest::Method::DELETE, path, None).await
    }
}

struct Conn {
    lockfile: PathBuf,
    contents: String,
    lcu: Arc<Lcu>,
}

static CURRENT: RwLock<Option<Conn>> = RwLock::new(None);

fn lockfile_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("MATTOFESSOR_LOCKFILE") {
        return Some(p.into());
    }
    let procs = crate::win::processes();
    ["LeagueClientUx.exe", "LeagueClient.exe"]
        .iter()
        .filter_map(|n| crate::win::find_process(&procs, n))
        .filter_map(crate::win::process_path)
        .filter_map(|exe| exe.parent().map(|d| d.join("lockfile")))
        .find(|p| p.exists())
}

fn parse_lockfile(text: &str) -> Result<(u16, String)> {
    let parts: Vec<&str> = text.trim().split(':').collect();
    if parts.len() < 5 {
        bail!("malformed lockfile");
    }
    let port = parts[2].parse().map_err(|_| anyhow!("bad port in lockfile"))?;
    Ok((port, parts[3].to_string()))
}

/// Re-check the lockfile; call about once a second. Returns whether the client is connected.
pub fn refresh() -> bool {
    let Some(path) = lockfile_path() else {
        if CURRENT.write().take().is_some() {
            log::info!("League client closed");
        }
        return false;
    };
    let contents = std::fs::read_to_string(&path).unwrap_or_default();
    {
        let cur = CURRENT.read();
        if cur.as_ref().is_some_and(|c| c.lockfile == path && c.contents == contents) {
            return true;
        }
    }
    match parse_lockfile(&contents) {
        Ok((port, password)) => {
            log::info!("League client found on port {port}");
            *CURRENT.write() = Some(Conn { lockfile: path, contents, lcu: Arc::new(Lcu::new(port, &password)) });
            true
        }
        Err(_) => {
            *CURRENT.write() = None;
            false
        }
    }
}

pub fn current() -> Option<Arc<Lcu>> {
    CURRENT.read().as_ref().map(|c| c.lcu.clone())
}

pub fn require() -> Result<Arc<Lcu>> {
    if current().is_none() {
        refresh();
    }
    current().ok_or_else(|| anyhow!("[client-off] League client is not running"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lockfile() {
        let (port, pw) = parse_lockfile("LeagueClient:40380:5571:test-password:https").unwrap();
        assert_eq!(port, 5571);
        assert_eq!(pw, "test-password");
        assert!(parse_lockfile("garbage").is_err());
    }
}
