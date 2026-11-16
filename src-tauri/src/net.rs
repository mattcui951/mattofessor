//! The HTTP client for public data (Data Dragon, CommunityDragon, OP.GG, AramHex, the Riot API):
//! one connection pool; each request sets its own timeout. The League client and the game's live
//! data have their own clients (local certificates).

use std::sync::OnceLock;

pub fn http() -> &'static reqwest::Client {
    static C: OnceLock<reqwest::Client> = OnceLock::new();
    C.get_or_init(|| reqwest::Client::builder().build().expect("http client"))
}
