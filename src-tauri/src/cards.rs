//! The augment card reader: during Hextech ARAM, finds the augment cards on screen and shows your
//! champion's win rate with each one in a small click-through window above the cards.
//!
//! Every `EVERY` it captures the band of the game window where card titles sit, reads it with
//! Windows' own OCR (the game's language first), joins the words that sit together into titles
//! and matches each title to an augment name. Two or more titles on one row mean the cards are
//! open. Nothing is saved and nothing leaves the PC.

use crate::aramhex::{self, AugRow};
use crate::core::Core;
use crate::ocr::{self, Line, Word};
use crate::os;
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::Emitter;

/// The band of the game window that is read (fractions of its height). Card titles sit at 41% of
/// a 3440x1440 window; the cards are sized by the HUD scale, so other setups may move them.
const BAND: (f64, f64) = (0.30, 0.62);
/// From a card title's middle up to the card's top edge, in card spacings (334 px for 489 px).
const TITLE_TO_TOP: f64 = 0.683;
/// Height of the window with the win rates, and the gap below it, in card spacings.
const STRIP: f64 = 0.2;
const GAP: f64 = 0.012;
/// How often the screen is read while a Hextech game runs (or twice the time a read takes, on a
/// slow PC: the reader never takes more than about a third of one core).
const EVERY: Duration = Duration::from_millis(350);
/// A read that found new cards is confirmed this soon (`SETTLED`).
const CONFIRM: Duration = Duration::from_millis(150);
const IDLE: Duration = Duration::from_secs(2);
/// Reads without cards before the win rates are hidden.
const MISSES_TO_HIDE: u32 = 2;
/// How far (fraction of the window width) a card may move between two reads and still count as
/// settled. The cards fly in when they open: a read during that shows them in the wrong places.
const SETTLED: f64 = 0.01;

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OfferCard {
    pub id: i32,
    /// The card's middle, as a fraction of the window width.
    pub x: f64,
    /// Your champion's numbers with this augment (none: no data).
    pub stat: Option<AugRow>,
    /// The best of the offered cards.
    pub best: bool,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Offer {
    pub champion_id: i32,
    /// The champion's own win rate.
    pub base: Option<f64>,
    pub cards: Vec<OfferCard>,
}

static OFFER: Mutex<Option<Offer>> = Mutex::new(None);

/// The cards on screen now, if any (the window asks when it opens).
pub fn current() -> Option<Offer> {
    OFFER.lock().clone()
}

// ---------------------------------------------------------------------------------------------
// The augment you took
//
// Which card was taken is not on screen once the cards close, and the game does not say during a
// match. While cards are shown the left button is watched (asked every `CLICK_EVERY`, no hook);
// when the cards close, the last click on a card (its face, not the reroll button under it)
// names the augment taken.

/// Cards as last shown: each augment and the part of the screen it is clicked on.
struct Shown {
    cards: Vec<(i32, os::Rect)>,
    since: Instant,
}

static SHOWN: Mutex<Option<Shown>> = Mutex::new(None);
/// Left button presses while cards were shown: when and where (screen units).
static CLICKS: Mutex<Vec<(Instant, i32, i32)>> = Mutex::new(Vec::new());
/// The augments taken this game, in order.
static TAKEN: Mutex<Vec<i32>> = Mutex::new(Vec::new());
static WATCH: AtomicBool = AtomicBool::new(false);
/// Debug builds: a demo pick is shown (`demo_pick`); the reader leaves it until `demo_close`.
static DEMO: AtomicBool = AtomicBool::new(false);

const CLICK_EVERY: Duration = Duration::from_millis(15);
/// Only a click this recent picks a card (a pick closes the cards within a second or so).
const CLICK_RECENT: Duration = Duration::from_secs(10);
/// A card's face around its title, in card spacings: 0.89 wide (gaps between cards are left
/// out), from its top edge (`TITLE_TO_TOP` above the title) to 0.55 under the title, short of
/// the reroll button below the card. Not measured on a real pick yet.
const CARD_W: f64 = 0.89;
const CARD_BELOW_TITLE: f64 = 0.55;

/// The augments taken this game, in order.
pub fn taken() -> Vec<i32> {
    TAKEN.lock().clone()
}

/// A new game: nothing taken yet.
pub fn new_game() {
    TAKEN.lock().clear();
    *SHOWN.lock() = None;
    CLICKS.lock().clear();
    if let Some(core) = crate::core::get() {
        let _ = core.app.emit("taken-augments", Vec::<i32>::new());
    }
}

/// Where each card is clicked (screen units), from where its title is.
pub fn card_rects(band: os::Rect, found: &[(i32, Line)]) -> Vec<(i32, os::Rect)> {
    let unit = spacing(found);
    found
        .iter()
        .map(|(id, l)| {
            let x = band.x as f64 + (l.x + l.w / 2.0) as f64;
            let title = band.y as f64 + (l.y + l.h / 2.0) as f64;
            let top = title - TITLE_TO_TOP * unit;
            let rect = os::Rect {
                x: (x - CARD_W * unit / 2.0).round() as i32,
                y: top.round() as i32,
                w: (CARD_W * unit).round() as i32,
                h: ((TITLE_TO_TOP + CARD_BELOW_TITLE) * unit).round() as i32,
            };
            (*id, rect)
        })
        .collect()
}

/// The card the last click landed on, of clicks made while these cards were shown.
pub fn clicked(cards: &[(i32, os::Rect)], clicks: &[(Instant, i32, i32)], since: Instant, now: Instant) -> Option<i32> {
    let inside = |r: &os::Rect, x: i32, y: i32| x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h;
    clicks
        .iter()
        .rev()
        .filter(|(t, _, _)| *t >= since && now.duration_since(*t) <= CLICK_RECENT)
        .find_map(|(_, x, y)| cards.iter().find(|(_, r)| inside(r, *x, *y)).map(|(id, _)| *id))
}

/// New cards are shown: a different set (not a reroll of one card) means the last set closed.
fn shown(core: &Core, cards: Vec<(i32, os::Rect)>) {
    let mut s = SHOWN.lock();
    let same_pick = s.as_ref().is_some_and(|old| old.cards.iter().any(|(id, _)| cards.iter().any(|(n, _)| n == id)));
    if same_pick {
        // A reroll, or the cards settling: the same pick, its clicks kept.
        if let Some(old) = s.as_mut() {
            old.cards = cards;
        }
    } else {
        let old = s.replace(Shown { cards, since: Instant::now() });
        drop(s);
        if let Some(old) = old {
            resolve(core, old);
        }
    }
    WATCH.store(true, Ordering::Relaxed);
}

/// The cards closed: which one was taken.
fn closed(core: &Core) {
    WATCH.store(false, Ordering::Relaxed);
    if let Some(old) = SHOWN.lock().take() {
        resolve(core, old);
    }
}

fn resolve(core: &Core, s: Shown) {
    let clicks = std::mem::take(&mut *CLICKS.lock());
    match clicked(&s.cards, &clicks, s.since, Instant::now()) {
        Some(id) => {
            let all = {
                let mut t = TAKEN.lock();
                t.push(id);
                t.clone()
            };
            log::info!("augment taken: {id} (of {})", s.cards.iter().map(|(id, _)| id.to_string()).collect::<Vec<_>>().join(", "));
            let _ = core.app.emit("taken-augments", all);
        }
        None => log::info!("augment taken: not seen ({} clicks while the cards were open)", clicks.len()),
    }
}

/// Debug builds: an offer shown over `band` as a read would show them (each a third of its
/// width, with the click watch on), to check a pick with a real click and no game.
#[cfg(debug_assertions)]
pub fn demo_pick(core: &Core, band: os::Rect, offer: Offer) {
    DEMO.store(true, Ordering::Relaxed);
    let n = offer.cards.len().max(1) as i32;
    let cards: Vec<(i32, os::Rect)> = offer.cards.iter().enumerate().map(|(i, c)| (c.id, os::Rect { x: band.x + band.w * i as i32 / n, y: band.y, w: band.w / n, h: band.h })).collect();
    *OFFER.lock() = Some(offer.clone());
    core.show_cards(os::Rect { y: band.y - band.h / 5, h: band.h / 5, ..band }, &offer);
    shown(core, cards);
}

/// Debug builds: a left click at a place (screen units), recorded as the click watch records one.
#[cfg(debug_assertions)]
pub fn demo_click(x: i32, y: i32) {
    if WATCH.load(Ordering::Relaxed) {
        CLICKS.lock().push((Instant::now(), x, y));
    }
}

/// Debug builds: the demo cards close (as after a pick).
#[cfg(debug_assertions)]
pub fn demo_close(core: &Core) {
    DEMO.store(false, Ordering::Relaxed);
    clear(core);
}

/// Watches the left button while cards are shown.
fn watch_clicks() {
    std::thread::Builder::new()
        .name("card clicks".into())
        .spawn(|| {
            let mut was_down = false;
            loop {
                if !WATCH.load(Ordering::Relaxed) {
                    was_down = false;
                    std::thread::sleep(Duration::from_millis(100));
                    continue;
                }
                let down = os::left_button_down();
                if down && !was_down {
                    if let Some((x, y)) = os::cursor_pos() {
                        let mut c = CLICKS.lock();
                        c.push((Instant::now(), x, y));
                        // Only the last few matter.
                        if c.len() > 32 {
                            c.remove(0);
                        }
                    }
                }
                was_down = down;
                std::thread::sleep(CLICK_EVERY);
            }
        })
        .expect("card click thread");
}

// ---------------------------------------------------------------------------------------------
// Matching text to augment names

/// Lower case letters and digits only (any script): OCR spacing and punctuation vary.
pub fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn levenshtein(a: &[char], b: &[char]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            cur[j + 1] = (prev[j] + usize::from(ca != cb))
                .min(prev[j + 1] + 1)
                .min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}

/// Augment names to match against: (normalized name, id), Hextech ARAM augments first so a
/// name shared with an Arena augment resolves to the Hextech one.
pub struct Names {
    list: Vec<(Vec<char>, i32)>,
}

impl Names {
    pub fn new<'a>(
        names: impl IntoIterator<Item = (i32, &'a str)>,
        hextech: &HashSet<i32>,
    ) -> Names {
        let mut list: Vec<(Vec<char>, i32)> = names
            .into_iter()
            .map(|(id, n)| (normalize(n).chars().collect::<Vec<char>>(), id))
            .filter(|(n, _)| n.len() >= 2)
            .collect();
        list.sort_by_key(|(n, id)| (!hextech.contains(id), n.len(), *id));
        list.dedup();
        Names { list }
    }

    /// The augment a line of text names, if it is close enough to one.
    pub fn find(&self, text: &str) -> Option<i32> {
        self.find_scored(text).map(|(id, _)| id)
    }

    /// The augment a text names and how many letters differ (0: exact).
    fn find_scored(&self, text: &str) -> Option<(i32, usize)> {
        let t: Vec<char> = normalize(text).chars().collect();
        if t.len() < 2 {
            return None;
        }
        if let Some((_, id)) = self.list.iter().find(|(n, _)| *n == t) {
            return Some((*id, 0));
        }
        // Close matches allow about one wrong letter in six (Latin) or one character in four
        // (Chinese, Japanese, Korean: short names, a character each word); shorter names must
        // match exactly. Only a clear winner counts: two names as close are a guess.
        let cjk = t.iter().any(|c| is_cjk(*c));
        let allowed = if cjk { t.len() / 4 } else { t.len() / 6 };
        if allowed == 0 {
            return None;
        }
        // The list holds Hextech augments first, so of two ids with one name the Hextech one wins.
        let mut best: Option<(usize, i32, &[char])> = None;
        let mut tied = false;
        for (n, id) in &self.list {
            if n.len().abs_diff(t.len()) > allowed {
                continue;
            }
            let d = levenshtein(n, &t);
            if d > allowed {
                continue;
            }
            match best {
                Some((bd, _, bn)) if d == bd && bn != n.as_slice() => tied = true,
                Some((bd, _, _)) if d >= bd => {}
                _ => {
                    best = Some((d, *id, n.as_slice()));
                    tied = false;
                }
            }
        }
        if tied {
            return None;
        }
        best.map(|(d, id, _)| (id, d))
    }
}

/// Words of at most this many letters and digits may sit around a title and be ignored: a
/// sparkle or a card's frame read as "x", "+" or "Il". Not Chinese (or Japanese, Korean) words:
/// the OCR gives each character of those as a word of its own.
const JUNK: usize = 2;

/// A Chinese, Japanese or Korean character.
fn is_cjk(c: char) -> bool {
    matches!(c as u32, 0x3040..=0x30FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xAC00..=0xD7AF | 0xF900..=0xFAFF)
}

/// The most cards an offer can have (three today; a mode with four is coming).
const MAX_CARDS: usize = 5;
/// Words closer than this many word heights belong to one piece of text (a card title). Words of
/// a title are about 0.3 heights apart (more in a wide font), titles of neighbouring cards 5+
/// (long names) and a card's frame about 4.5 away from its title.
const WORD_GAP: f32 = 3.0;

/// Texts on the screen: words that sit together on one row, joined (one card title each).
fn phrases(lines: &[Line]) -> Vec<Vec<Word>> {
    // Words, each once (wide images are read in overlapping tiles: a word can come twice).
    let mut words: Vec<Word> = Vec::new();
    for w in lines.iter().flat_map(|l| &l.words) {
        let twice = words.iter().any(|k| {
            k.text == w.text && (k.x - w.x).abs() < k.h / 2.0 && (k.y - w.y).abs() < k.h / 2.0
        });
        if !twice {
            words.push(w.clone());
        }
    }
    let mid = |w: &Word| w.y + w.h / 2.0;
    words.sort_by(|a, b| mid(a).total_cmp(&mid(b)));
    // Rows: words whose middles are within half a word height of the row's first word.
    let mut rows: Vec<Vec<Word>> = Vec::new();
    for w in words {
        match rows.last_mut() {
            Some(row) if (mid(&w) - mid(&row[0])).abs() < row[0].h.max(w.h) / 2.0 => row.push(w),
            _ => rows.push(vec![w]),
        }
    }
    let mut out = Vec::new();
    for mut row in rows {
        row.sort_by(|a, b| a.x.total_cmp(&b.x));
        let mut phrase: Vec<Word> = Vec::new();
        for w in row {
            if let Some(last) = phrase.last() {
                if w.x - (last.x + last.w) > WORD_GAP * last.h.max(w.h) {
                    out.push(std::mem::take(&mut phrase));
                }
            }
            phrase.push(w);
        }
        if !phrase.is_empty() {
            out.push(phrase);
        }
    }
    out
}

/// The augment a phrase names, with where its name is: the whole phrase, or the phrase without
/// junk words around it. A name inside other words does not count ("Recursion" is an augment of
/// its own, and so is a word in many descriptions).
fn title(phrase: &[Word], names: &Names) -> Option<(i32, Line)> {
    let junk = |w: &Word| {
        let n = normalize(&w.text);
        n.chars().count() <= JUNK && !n.chars().any(is_cjk)
    };
    let start = phrase.iter().position(|w| !junk(w))?;
    let end = phrase.iter().rposition(|w| !junk(w))?;
    // The longest run first: the whole phrase, then without junk at the ends.
    let mut runs = vec![(0, phrase.len() - 1)];
    if (start, end) != (0, phrase.len() - 1) {
        runs.push((start, end));
    }
    let mut best: Option<(i32, usize, usize, usize)> = None;
    for (i, j) in runs {
        let text = phrase[i..=j].iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" ");
        if let Some((id, d)) = names.find_scored(&text) {
            if best.is_none_or(|(_, bd, _, _)| d < bd) {
                best = Some((id, d, i, j));
            }
        }
    }
    let (id, _, i, j) = best?;
    let run = &phrase[i..=j];
    let x = run.iter().map(|w| w.x).fold(f32::MAX, f32::min);
    let y = run.iter().map(|w| w.y).fold(f32::MAX, f32::min);
    let x2 = run.iter().map(|w| w.x + w.w).fold(f32::MIN, f32::max);
    let y2 = run.iter().map(|w| w.y + w.h).fold(f32::MIN, f32::max);
    let text = run.iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" ");
    Some((id, Line { text, x, y, w: x2 - x, h: y2 - y, words: run.to_vec() }))
}

/// The cards among the text read: augment titles, left to right, each with where its title is.
pub fn cards(lines: &[Line], names: &Names) -> Vec<(i32, Line)> {
    let mut found: Vec<(i32, Line)> = Vec::new();
    for phrase in phrases(lines) {
        if let Some((id, l)) = title(&phrase, names) {
            if !found.iter().any(|(f, _)| *f == id) {
                found.push((id, l));
            }
        }
    }
    // Card titles share one row: keep the row most matches are on (other text can name an
    // augment too, like a tooltip or a description).
    let mid = |l: &Line| l.y + l.h / 2.0;
    let row = found
        .iter()
        .max_by_key(|(_, l)| {
            found
                .iter()
                .filter(|(_, o)| (mid(o) - mid(l)).abs() < l.h)
                .count()
        })
        .map(|(_, l)| (mid(l), l.h));
    if let Some((y, h)) = row {
        found.retain(|(_, l)| (mid(l) - y).abs() < h);
    }
    found.sort_by(|a, b| a.1.x.total_cmp(&b.1.x));
    found.truncate(MAX_CARDS);
    found
}

/// The offer shown for matched cards: each card's numbers and the best one (only when at least two
/// cards have enough games to compare).
pub fn offer(champion_id: i32, champ: &aramhex::Champion, found: &[(i32, f64)]) -> Offer {
    let mut cards: Vec<OfferCard> = found
        .iter()
        .map(|(id, x)| OfferCard {
            id: *id,
            x: *x,
            stat: champ.augments.iter().find(|a| a.id == *id).cloned(),
            best: false,
        })
        .collect();
    let best = cards
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            c.stat
                .as_ref()
                .filter(|s| !s.low_sample)
                .map(|s| (i, s.delta))
        })
        .collect::<Vec<_>>();
    if best.len() >= 2 {
        if let Some((i, _)) = best.iter().max_by(|a, b| a.1.total_cmp(&b.1)) {
            cards[*i].best = true;
        }
    }
    Offer {
        champion_id,
        base: champ.champion.as_ref().map(|c| c.win_rate),
        cards,
    }
}

/// Cards for the log: augment id @ place across the window.
fn describe(cards: &[(i32, f64)]) -> String {
    cards.iter().map(|(id, x)| format!("{id}@{x:.3}")).collect::<Vec<_>>().join(" ")
}

/// Whether two reads saw the same cards in the same places (the cards have stopped moving).
pub fn settled(prev: &[(i32, f64)], cur: &[(i32, f64)]) -> bool {
    prev.len() == cur.len()
        && prev
            .iter()
            .zip(cur)
            .all(|(a, b)| a.0 == b.0 && (a.1 - b.1).abs() < SETTLED)
}

/// Distance between neighbouring card centers (pixels): the unit the cards are drawn in.
fn spacing(found: &[(i32, Line)]) -> f64 {
    let xs: Vec<f64> = found
        .iter()
        .map(|(_, l)| (l.x + l.w / 2.0) as f64)
        .collect();
    let gap = xs.windows(2).map(|w| w[1] - w[0]).fold(f64::MAX, f64::min);
    // Two cards found may be the outer two: cards are about 20 title heights apart.
    let title = found
        .iter()
        .map(|(_, l)| l.h as f64)
        .fold(f64::MAX, f64::min);
    if found.len() == 2 && gap > 34.0 * title {
        gap / 2.0
    } else {
        gap
    }
}

/// Where the win rates go (physical pixels): across the game, just above the cards.
pub fn strip(game: os::Rect, band_y: i32, found: &[(i32, Line)]) -> os::Rect {
    let unit = spacing(found);
    let title_mid = found
        .iter()
        .map(|(_, l)| (l.y + l.h / 2.0) as f64)
        .sum::<f64>()
        / found.len() as f64;
    let card_top = band_y as f64 + title_mid - TITLE_TO_TOP * unit;
    let h = (STRIP * unit).round() as i32;
    let y = ((card_top - GAP * unit) as i32 - h).max(game.y);
    os::Rect {
        x: game.x,
        y,
        w: game.w,
        h,
    }
}

// ---------------------------------------------------------------------------------------------
// The reader loop

struct Reader {
    /// Names in both languages, built once per bracket data set.
    names: Option<Arc<Names>>,
    /// The OCR language that found cards last (`lang` id): tried first.
    lang: &'static str,
    misses: u32,
    /// The cards the last read found (ids and places), shown once the next read agrees.
    last: Vec<(i32, f64)>,
    /// This game's setup was logged.
    told: bool,
    /// A black screen capture was logged this game.
    told_black: bool,
    /// Asked for screen recording (macOS), once a game.
    asked_screen: bool,
    /// The card window and the bracket's numbers are ready for this game (`prepare`).
    prepared: bool,
    /// When the read that first saw these cards started (for the log).
    seen_at: Option<std::time::Instant>,
    /// How long the last read took.
    read_time: Duration,
    /// How long to wait before the next read.
    next: Duration,
}

impl Reader {
    fn new() -> Reader {
        Reader {
            names: None,
            lang: "en",
            misses: 0,
            last: Vec::new(),
            told: false,
            told_black: false,
            asked_screen: false,
            prepared: false,
            seen_at: None,
            read_time: Duration::ZERO,
            next: EVERY,
        }
    }
}

/// Ready for the first pick of a game: the card window made (hidden; a new web view takes a
/// moment to start), the numbers of the bracket in use loaded, the game's language read first.
async fn prepare(core: &Arc<Core>, r: &mut Reader) {
    r.prepared = true;
    let started = std::time::Instant::now();
    core.prepare_cards();
    let bracket = core.settings().aram_bracket;
    if let Err(e) = aramhex::bracket(&bracket).await {
        log::warn!("card reader stats: {e:#}");
    }
    if let Some(l) = status(core).game_lang.filter(|l| ocr::available(l)) {
        r.lang = l;
    }
    log::info!("card reader ready in {} ms", started.elapsed().as_millis());
}

/// Whether the system can read the game's text: its language ("zh_CN", "en_US"...), the screen.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReaderStatus {
    /// The League client's language (`lang` id); none before it connected once.
    pub game_lang: Option<&'static str>,
    /// The OCR language tag for it ("ja-JP"...), for installing it on Windows.
    pub ocr_tag: Option<&'static str>,
    /// The game's language cannot be read on this computer (its OCR language is not installed).
    pub missing: bool,
    /// macOS has not let the app record the screen.
    pub screen_blocked: bool,
}

pub fn status(core: &Core) -> ReaderStatus {
    let game = core.client_locale.read().as_deref().map(crate::lang::from_client);
    ReaderStatus {
        game_lang: game.map(|l| l.id),
        ocr_tag: game.map(|l| l.ocr),
        missing: game.is_some_and(|l| !ocr::available(l.id)),
        screen_blocked: !os::screen_capture_allowed(),
    }
}

/// Mostly black: what a screen capture of a game in exclusive Full screen gives.
pub fn black(rgba: &[u8]) -> bool {
    let (px, _) = rgba.as_chunks::<4>();
    let n = px.len().max(1);
    let dark = px.iter().step_by(7).filter(|p| p[0] < 12 && p[1] < 12 && p[2] < 12).count();
    dark * 7 * 100 >= n * 98
}

async fn names() -> anyhow::Result<Names> {
    let (_, b) = aramhex::bracket("all").await?;
    let hextech: HashSet<i32> = b
        .hero_augments
        .values()
        .flat_map(|m| m.keys().copied())
        .collect();
    // English and the client's language (Chinese too while the client is not known yet).
    let game = core_lang();
    let mut sets = vec![crate::augments::all("en").await?];
    for l in [game.unwrap_or("zh")] {
        if l != "en" {
            sets.push(crate::augments::all(l).await?);
        }
    }
    let all = sets.iter().flat_map(|s| s.iter()).map(|(id, a)| (*id, a.name.as_str()));
    Ok(Names::new(all, &hextech))
}

/// The League client's language, when it connected once.
fn core_lang() -> Option<&'static str> {
    let core = crate::core::get()?;
    let locale = core.client_locale.read().clone()?;
    Some(crate::lang::from_client(&locale).id)
}

/// Your champion in a Hextech ARAM game, if that is what is being played.
/// The game's own data says who you are when the client's does not (another account's game).
fn my_hextech_champion(core: &Core) -> Option<i32> {
    let ingame = core.ingame.lock().clone();
    let live = core.live.lock();
    if live.mode != "KIWI" && ingame.as_ref().is_none_or(|g| g.mode != "KIWI") {
        return None;
    }
    let slots = || live.teams.iter().flat_map(|t| &t.players);
    let me = slots().find(|p| p.me).or_else(|| {
        let id = &ingame.as_ref()?.players.iter().find(|p| p.me)?.riot_id;
        slots().find(|p| !p.name.is_empty() && &p.name == id)
    });
    me.map(|p| p.champion_id).filter(|c| *c > 0)
}

fn clear(core: &Core) {
    if OFFER.lock().take().is_some() {
        core.hide_cards();
        closed(core);
    }
}

/// One look at the screen. Returns false when there is nothing to watch (no game, not Hextech).
async fn step(core: &Arc<Core>, r: &mut Reader) -> bool {
    let Some(champion_id) = my_hextech_champion(core) else {
        return false;
    };
    let Some(game) = os::game_window().and_then(os::window_rect) else {
        return false;
    };
    if r.names.is_none() {
        match names().await {
            Ok(n) => r.names = Some(Arc::new(n)),
            Err(e) => {
                log::warn!("card reader: {e:#}");
                return false;
            }
        }
    }
    let names = r.names.clone().expect("names");
    if !r.prepared {
        prepare(core, r).await;
    }
    // macOS shows nothing of other apps to an app the user did not let record the screen.
    if !os::screen_capture_allowed() {
        if !r.asked_screen {
            r.asked_screen = true;
            log::warn!("card reader: not allowed to record the screen (macOS privacy settings)");
            os::request_screen_capture();
        }
        return true;
    }
    // Mattofessor's own panel covers the cards and lists augments by name in columns: read
    // through it, its names look like cards in the wrong places. Wait until it closes.
    if core.overlay_visible() {
        r.last.clear();
        r.misses += 1;
        if r.misses >= MISSES_TO_HIDE {
            clear(core);
        }
        return true;
    }
    let band = os::Rect {
        x: game.x,
        y: game.y + (game.h as f64 * BAND.0) as i32,
        w: game.w,
        h: (game.h as f64 * (BAND.1 - BAND.0)) as i32,
    };
    if !r.told {
        r.told = true;
        let st = status(core);
        let mode = crate::display::window_mode().await.unwrap_or("unknown");
        log::info!(
            "card reader: game window {}x{} at {},{}, League {mode}, game language {} (OCR {})",
            game.w, game.h, game.x, game.y, st.game_lang.unwrap_or("?"), if st.missing { "missing" } else { "ok" }
        );
        if st.missing {
            log::warn!("card reader: the system cannot read {} text (its OCR language is not installed)", st.game_lang.unwrap_or("?"));
        }
    }
    let first = r.lang;
    let tell_black = !r.told_black;
    let read_started = std::time::Instant::now();
    let read = tauri::async_runtime::spawn_blocking(move || {
        let img = os::capture(band)?;
        if tell_black && black(&img.rgba) {
            log::warn!("card reader: the game screen captures as black (League in Full screen?)");
            return Some(Err(()));
        }
        let mut langs = vec![first];
        if first != "en" {
            langs.push("en");
        }
        for lang in langs {
            if !ocr::available(lang) {
                continue;
            }
            match ocr::read(&img.rgba, img.w, img.h, lang) {
                Ok(lines) => {
                    // A Retina capture has more pixels than the band has points.
                    let f = band.w as f32 / img.w as f32;
                    let lines: Vec<Line> = lines.into_iter().map(|l| l.scaled(f)).collect();
                    let found = cards(&lines, &names);
                    if found.len() >= 2 {
                        return Some(Ok((lang, found)));
                    }
                }
                Err(e) => log::warn!("card reader OCR: {e:#}"),
            }
        }
        None
    })
    .await
    .ok()
    .flatten();

    r.read_time = read_started.elapsed();
    if matches!(read, Some(Err(()))) {
        r.told_black = true;
    }
    let Some(Ok((lang, found))) = read else {
        r.last.clear();
        r.misses += 1;
        if r.misses >= MISSES_TO_HIDE {
            clear(core);
        }
        return true;
    };
    r.misses = 0;
    r.lang = lang;
    let xs: Vec<(i32, f64)> = found
        .iter()
        .map(|(id, l)| (*id, (l.x + l.w / 2.0) as f64 / band.w as f64))
        .collect();
    if cfg!(debug_assertions) {
        log::info!("card read ({lang}): {}", describe(&xs));
    }
    let previous = std::mem::replace(&mut r.last, xs.clone());
    let same_cards = previous.len() == xs.len() && previous.iter().zip(&xs).all(|(a, b)| a.0 == b.0);
    if !same_cards {
        r.seen_at = Some(read_started);
    }
    if !settled(&previous, &xs) {
        // Still moving (opening, or a card just rerolled): look again soon.
        r.next = CONFIRM;
        return true;
    }

    let bracket_name = core.settings().aram_bracket;
    let champ = match aramhex::bracket(&bracket_name).await {
        Ok((info, b)) => aramhex::champion(&info, &bracket_name, &b, champion_id),
        Err(e) => {
            log::warn!("card reader stats: {e:#}");
            return true;
        }
    };
    let o = offer(champion_id, &champ, &xs);
    let rect = strip(game, band.y, &found);
    let changed = OFFER.lock().as_ref() != Some(&o);
    if changed {
        let after = r.seen_at.take().map(|t| t.elapsed().as_millis()).unwrap_or(0);
        log::info!("card win rates shown: {} ({after} ms after the cards were first seen; a read takes {} ms)", describe(&xs), r.read_time.as_millis());
        *OFFER.lock() = Some(o.clone());
        core.show_cards(rect, &o);
        shown(core, card_rects(band, &found));
    }
    true
}

pub fn spawn(core: Arc<Core>) {
    watch_clicks();
    tauri::async_runtime::spawn(async move {
        let mut r = Reader::new();
        loop {
            r.next = EVERY;
            if DEMO.load(Ordering::Relaxed) {
                tokio::time::sleep(IDLE).await;
                continue;
            }
            let active = core.settings().card_reader && step(&core, &mut r).await;
            if !active {
                clear(&core);
                // A new game starts over (its window, language and data are prepared again).
                if r.prepared || r.told {
                    let names = r.names.take();
                    r = Reader::new();
                    r.names = names;
                }
            }
            let wait = if active { r.next.max(r.read_time * 2) } else { IDLE };
            tokio::time::sleep(wait).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_black_capture_is_seen() {
        let black = [0u8, 0, 0, 255].repeat(1000);
        assert!(super::black(&black));
        let mut cards = black.clone();
        for p in cards.as_chunks_mut::<4>().0.iter_mut().step_by(3) {
            p[0] = 200;
        }
        assert!(!super::black(&cards));
    }

    fn names() -> Names {
        let hextech: HashSet<i32> = [1088, 1325, 1225].into();
        Names::new(
            [
                (1088, "Ultimate Revolution"),
                (1325, "Glass Cannon"),
                (1225, "Dual Wield"),
                (9225, "Dual Wield"),
                (1326, "Glass Canon Pro"),
                (1, "Tank"),
                (1225, "双持"),
                (1310, "Clown College"),
                (1004, "Back To Basics"),
                (1388, "Infinite Recursion"),
                (1068, "Recursion"),
            ],
            &hextech,
        )
    }

    /// A line as the OCR gives it: its words side by side (10 px per letter, 6 px apart).
    fn line(text: &str, x: f32) -> Line {
        let mut words = Vec::new();
        let mut at = x;
        for t in text.split(' ') {
            let w = 10.0 * t.chars().count() as f32;
            words.push(Word { text: t.into(), x: at, y: 10.0, w, h: 20.0 });
            at += w + 6.0;
        }
        Line { text: text.into(), x, y: 10.0, w: at - 6.0 - x, h: 20.0, words }
    }

    fn middle(l: &Line) -> f32 {
        l.x + l.w / 2.0
    }

    /// The cards of a real pick (Hecarim, 7 Nov) as the OCR cut them in the game: two titles came
    /// as two lines each. Reading lines, "Clown College" was lost and "Recursion" (another
    /// augment) was shown at the right half of "Infinite Recursion".
    #[test]
    fn a_title_cut_into_two_lines_is_still_one_card() {
        let n = names();
        let lines = [
            line("Clown", 1142.0),
            line("College", 1208.0),
            line("Back To Basics", 1630.0),
            line("Infinite", 2103.0),
            line("Recursion", 2189.0),
        ];
        let found = cards(&lines, &n);
        let ids: Vec<i32> = found.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, [1310, 1004, 1388]);
        assert_eq!(middle(&found[2].1), (2103.0 + 2279.0) / 2.0, "the whole title's middle");
        assert_eq!(n.find("Recursion"), Some(1068), "the half alone names another augment");
        // Words of a title can sit far apart in a wide font (here 50 px for 20 px high words);
        // titles of neighbouring cards are farther apart.
        let wide = [line("Infinite", 2103.0), line("Recursion", 2233.0), line("Clown College", 2425.0)];
        let ids: Vec<i32> = cards(&wide, &n).iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, [1388, 1310]);
    }

    #[test]
    fn junk_around_a_title_is_left_out() {
        let n = names();
        // A card frame and a sparkle read as letters on each side of the title.
        let lines = [line("Il Infinite Recursion x", 2083.0), line("+ Clown College", 1122.0)];
        let found = cards(&lines, &n);
        let ids: Vec<i32> = found.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, [1310, 1388]);
        assert_eq!(found[1].1.text, "Infinite Recursion");
        assert_eq!(found[1].1.x, 2083.0 + 26.0, "the box starts at the title");
    }

    #[test]
    fn a_name_inside_other_words_is_not_a_card() {
        let n = names();
        let lines = [
            line("Gain Recursion stacks on takedowns", 100.0),
            line("Deal Recursion", 900.0),
        ];
        assert!(cards(&lines, &n).is_empty());
    }

    #[test]
    fn matches_titles_despite_ocr_noise() {
        let n = names();
        assert_eq!(n.find("Ultimate Revolution"), Some(1088));
        assert_eq!(
            n.find("Ultimate Revo1ution"),
            Some(1088),
            "one wrong letter"
        );
        assert_eq!(
            n.find("Dual Wield"),
            Some(1225),
            "the Hextech id wins over an Arena one with the same name"
        );
        assert_eq!(n.find("双 持"), Some(1225), "Chinese with OCR spaces");
        assert_eq!(n.find("Tamk"), None, "short names match exactly only");
        assert_eq!(n.find("Reduce max Health by 30%. Deal an extra"), None);
    }

    #[test]
    fn chinese_titles_are_cards() {
        // What Windows' Chinese OCR read from a real Hextech ARAM pick (1525 px wide): each
        // character a word, with a description line under each title.
        let zh = |text: &str, x: f32, y: f32| {
            let chars: Vec<&str> = text.split(' ').collect();
            let words = chars
                .iter()
                .enumerate()
                .map(|(i, c)| Word { text: (*c).into(), x: x + i as f32 * 19.0, y, w: 16.0, h: 18.0 })
                .collect::<Vec<_>>();
            Line { text: text.into(), x, y, w: chars.len() as f32 * 19.0, h: 18.0, words }
        };
        let names = Names::new(
            [(1375, "吞噬灵魂"), (2116, "威能之追求"), (3116, "将他们放倒倒倒"), (1323, "残忍"), (3, "放倒")],
            &HashSet::new(),
        );
        let lines = [
            zh("吞 噬 灵 魂", 383.0, 99.0),
            zh("定 身 一 个 敌 方 雄 时 会 提 供", 313.0, 166.0),
            // The mouse covered the last character; the "、" is the cursor's edge.
            zh("威 能 之 追 求 、", 700.0, 99.0),
            zh("将 他 们 放 倒 倒 倒", 1028.0, 99.0),
        ];
        let ids: Vec<i32> = cards(&lines, &names).iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, [1375, 2116, 3116]);
        // One wrong character in a long enough name still matches; a short one must be exact.
        assert_eq!(names.find("将他们放到倒倒"), Some(3116));
        assert_eq!(names.find("残恩"), None);
    }

    #[test]
    fn four_cards_are_read() {
        let lines = [
            line("Ultimate Revolution", 100.0),
            line("Glass Cannon", 700.0),
            line("Dual Wield", 1300.0),
            line("Clown College", 1900.0),
        ];
        let ids: Vec<i32> = cards(&lines, &names()).iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, [1088, 1325, 1225, 1310]);
    }

    #[test]
    fn a_close_match_needs_a_clear_winner() {
        let names = Names::new([(1, "Glass Cannon"), (2, "Glass Cannot")], &HashSet::new());
        // One letter from both: a guess, not a match.
        assert_eq!(names.find("Glass Cannox"), None);
        assert_eq!(names.find("Glass Canon"), Some(1));
    }

    #[test]
    fn cards_are_ordered_left_to_right() {
        let mut tooltip = line("Glass Cannon Pro", 1300.0);
        for w in &mut tooltip.words {
            w.y = 60.0;
        }
        let lines = [
            line("Dual Wield", 1600.0),
            line("Speed", 700.0),
            tooltip,
            line("Ultimate Revolution", 100.0),
            line("Glass Cannon", 1100.0),
        ];
        let ids: Vec<i32> = cards(&lines, &names()).iter().map(|(id, _)| *id).collect();
        assert_eq!(
            ids,
            [1088, 1325, 1225],
            "the name on another row is not a card"
        );
    }

    #[test]
    fn shown_only_once_the_cards_stop_moving() {
        let opening = [(1097, 0.40), (1318, 0.55)];
        let open = [(1097, 0.357), (1318, 0.5), (1205, 0.643)];
        assert!(!settled(&[], &opening), "first read");
        assert!(!settled(&opening, &open), "a card more, and moved");
        assert!(settled(&open, &[(1097, 0.358), (1318, 0.5), (1205, 0.642)]), "same places");
        assert!(
            !settled(&open, &[(1097, 0.357), (1318, 0.5), (1088, 0.643)]),
            "the third card was rerolled"
        );
    }

    #[test]
    fn best_card_skips_small_samples() {
        let row = |id, delta: f64, low| AugRow {
            id,
            games: 1000,
            win_rate: 0.5 + delta,
            smoothed: 0.5 + delta,
            delta,
            low_sample: low,
        };
        let champ = aramhex::Champion {
            source: aramhex::Source {
                patch: String::new(),
                snapshot_at: String::new(),
                matches: 0,
                bracket: "all".into(),
            },
            champion: None,
            augments: vec![
                row(1, 0.09, true),
                row(2, 0.005, false),
                row(3, -0.04, false),
            ],
        };
        let o = offer(119, &champ, &[(1, 0.3), (2, 0.5), (3, 0.7), (4, 0.9)]);
        let best: Vec<i32> = o.cards.iter().filter(|c| c.best).map(|c| c.id).collect();
        assert_eq!(best, [2]);
        assert!(o.cards[3].stat.is_none());
        let alone = offer(119, &champ, &[(1, 0.3), (3, 0.5), (4, 0.7)]);
        assert!(
            alone.cards.iter().all(|c| !c.best),
            "one card with enough games has nothing to be best against"
        );
    }

    #[test]
    fn strip_sits_above_the_cards() {
        let game = os::Rect {
            x: 0,
            y: 0,
            w: 3440,
            h: 1440,
        };
        // Titles 489 px apart with their middles at y 593 (band from y 432).
        let found: Vec<(i32, Line)> = [1231.0, 1720.0, 2209.0]
            .iter()
            .enumerate()
            .map(|(i, x)| {
                (
                    i as i32,
                    Line {
                        text: String::new(),
                        x: x - 60.0,
                        y: 151.0,
                        w: 120.0,
                        h: 20.0,
                        words: Vec::new(),
                    },
                )
            })
            .collect();
        let r = strip(game, 432, &found);
        assert_eq!((r.y, r.h), (155, 98), "card top at 259");
        let outer = [found[0].clone(), found[2].clone()];
        assert_eq!(
            strip(game, 432, &outer).h,
            98,
            "two outer cards give the same spacing"
        );
    }

    #[test]
    fn the_card_clicked_last_is_the_one_taken() {
        // Three cards 489 px apart, titles at y 593 of the screen (band from y 432).
        let found: Vec<(i32, Line)> = [(11, 1231.0), (22, 1720.0), (33, 2209.0)]
            .iter()
            .map(|(id, x)| (*id, Line { text: String::new(), x: x - 60.0, y: 151.0, w: 120.0, h: 20.0, words: Vec::new() }))
            .collect();
        let band = os::Rect { x: 0, y: 432, w: 3440, h: 461 };
        let rects = card_rects(band, &found);
        let r = rects[1].1;
        assert_eq!((r.x, r.y, r.w), (1502, 259, 435), "the middle card's face, from its top edge");
        assert_eq!(r.y + r.h, 862, "short of the reroll button under the card");

        let start = Instant::now();
        let at = |ms: u64, x: i32, y: i32| (start + Duration::from_millis(ms), x, y);
        let now = start + Duration::from_secs(3);
        // A reroll under the left card, then a click on the right card.
        let clicks = [at(100, 1231, 900), at(1500, 2209, 600)];
        assert_eq!(clicked(&rects, &clicks, start, now), Some(33));
        assert_eq!(clicked(&rects, &clicks[..1], start, now), None, "a reroll is not a pick");
        // A click between two cards, or from before these cards were shown, picks nothing.
        assert_eq!(clicked(&rects, &[at(100, 1475, 600)], start, now), None);
        assert_eq!(clicked(&rects, &[at(100, 1720, 600)], start + Duration::from_millis(500), now), None);
        // The last click on a card wins.
        assert_eq!(clicked(&rects, &[at(100, 1231, 600), at(200, 1720, 600)], start, now), Some(22));
        // Too long ago.
        assert_eq!(clicked(&rects, &[at(100, 1720, 600)], start, start + Duration::from_secs(30)), None);
    }

    /// The real cards from a Hextech ARAM game: the title row of a 3440x1440 screen (its left
    /// 2752 pixels), English client.
    #[test]
    fn reads_real_cards() {
        if !ocr::available("en") {
            eprintln!("no English OCR on this PC: skipped");
            return;
        }
        let file = std::fs::File::open(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/cards-en-3440x1440.png"
        ))
        .unwrap();
        let mut dec = png::Decoder::new(std::io::BufReader::new(file));
        dec.set_transformations(png::Transformations::EXPAND | png::Transformations::ALPHA);
        let mut reader = dec.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        assert_eq!(info.color_type, png::ColorType::Rgba);
        let lines = ocr::read(&buf[..info.buffer_size()], info.width, info.height, "en").unwrap();
        let found = cards(&lines, &names());
        let ids: Vec<i32> = found.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, [1088, 1325, 1225], "lines read: {lines:?}");
        let xs: Vec<f64> = found
            .iter()
            .map(|(_, l)| ((l.x + l.w / 2.0) / info.width as f32) as f64)
            .collect();
        assert!(
            (xs[0] - 0.447).abs() < 0.02 && (xs[2] - 0.803).abs() < 0.02,
            "{xs:?}"
        );
    }
}

/// Scans saved augment card samples (`MF_SAMPLES` = a capture folder, `MF_AUGMENTS` = CDragon's
/// cherry-augments.json) and prints where cards were found. Run by hand:
/// `cargo test --lib scan_samples -- --ignored --nocapture`.
#[cfg(test)]
mod scan {
    use super::*;

    #[test]
    #[ignore]
    fn scan_samples() {
        let dir = std::env::var("MF_SAMPLES").expect("MF_SAMPLES");
        let raw: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(std::env::var("MF_AUGMENTS").expect("MF_AUGMENTS")).unwrap(),
        )
        .unwrap();
        let augs = crate::augments::parse(&raw);
        let names = Names::new(
            augs.iter().map(|(id, a)| (*id, a.name.as_str())),
            &HashSet::new(),
        );
        let mut files: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "png"))
            .collect();
        files.sort();
        let (mut with, mut total) = (0, 0);
        for f in files {
            let mut dec =
                png::Decoder::new(std::io::BufReader::new(std::fs::File::open(&f).unwrap()));
            dec.set_transformations(png::Transformations::EXPAND | png::Transformations::ALPHA);
            let mut reader = dec.read_info().unwrap();
            let mut buf = vec![0; reader.output_buffer_size()];
            let info = reader.next_frame(&mut buf).unwrap();
            // Samples hold 12%-88% of the window height; read the same band the reader does.
            let win_h = info.height as f64 / 0.76;
            let y0 = ((BAND.0 - 0.12) * win_h) as u32;
            let h = ((BAND.1 - BAND.0) * win_h) as u32;
            let row = (info.width * 4) as usize;
            let band = &buf[y0 as usize * row..(y0 + h) as usize * row];
            let lines = ocr::read(band, info.width, h, "en").unwrap();
            // MF_LINES=1 prints every line the OCR read, to see why a title did not match.
            if std::env::var_os("MF_LINES").is_some() {
                for l in &lines {
                    println!("  {:?} x={:.0} y={:.0} w={:.0} h={:.0}", l.text, l.x, l.y, l.w, l.h);
                }
            }
            let found = cards(&lines, &names);
            total += 1;
            if found.len() >= 2 {
                with += 1;
                let desc: Vec<String> = found
                    .iter()
                    .map(|(id, l)| {
                        format!(
                            "{} x={:.3} y={:.3}",
                            augs[id].name,
                            (l.x + l.w / 2.0) / info.width as f32,
                            (y0 as f64 + (l.y + l.h / 2.0) as f64) / win_h + 0.12
                        )
                    })
                    .collect();
                println!(
                    "{}: {}",
                    f.file_name().unwrap().to_string_lossy(),
                    desc.join(" | ")
                );
            }
        }
        println!("cards found in {with} of {total} samples");
    }
    /// One screenshot (, a PNG of the whole game window) read in both languages, with
    /// the client's augment lists as the app caches them (, : the
    /// "augments:*" values of the kv table). Prints what each step saw. Run by hand:
    /// .
    #[test]
    #[ignore]
    fn scan_image() {
        let list = |var: &str| -> Vec<(i32, String)> {
            let (_, raw): (i64, serde_json::Value) = serde_json::from_str(&std::fs::read_to_string(std::env::var(var).expect(var)).unwrap()).unwrap();
            crate::augments::parse(&raw).into_iter().map(|(id, a)| (id, a.name)).collect()
        };
        let (en, zh) = (list("MF_AUG_EN"), list("MF_AUG_ZH"));
        let names = Names::new(en.iter().chain(zh.iter()).map(|(id, n)| (*id, n.as_str())), &HashSet::new());
        let path = std::env::var("MF_IMAGE").expect("MF_IMAGE");
        let mut dec = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(&path).unwrap()));
        dec.set_transformations(png::Transformations::EXPAND | png::Transformations::ALPHA);
        let mut reader = dec.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        let (w, h) = (info.width, info.height);
        let y0 = (BAND.0 * h as f64) as u32;
        let bh = ((BAND.1 - BAND.0) * h as f64) as u32;
        let row = (w * 4) as usize;
        let band = &buf[y0 as usize * row..(y0 + bh) as usize * row];
        println!("{path}: {w}x{h}, band y {y0}..{}", y0 + bh);
        for lang in ["en", "zh", "zh"] {
            let t = std::time::Instant::now();
            let lines = ocr::read(band, w, bh, lang).unwrap();
            let ms = t.elapsed().as_millis();
            let found = cards(&lines, &names);
            println!("[{lang}] {ms} ms, {} lines, cards: {:?}", lines.len(), found.iter().map(|(id, l)| (*id, l.text.clone(), ((l.x + l.w / 2.0) / w as f32 * 1000.0).round() / 1000.0)).collect::<Vec<_>>());
            if std::env::var_os("MF_LINES").is_some() {
                for l in &lines {
                    println!("  {:?} x={:.0} y={:.0} w={:.0} h={:.0}", l.text, l.x, l.y, l.w, l.h);
                }
            }
        }
    }
}
