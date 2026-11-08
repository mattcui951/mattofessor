//! The augment card reader: during Hextech ARAM, finds the augment cards on screen and shows your
//! champion's win rate with each one in a small click-through window above the cards.
//!
//! About twice a second it captures the band of the game window where card titles sit, reads it
//! with Windows' own OCR (English, then Chinese), joins the words that sit together into titles
//! and matches each title to an augment name. Two or three matches mean the cards are open.
//! Nothing is saved and nothing leaves the PC.

use crate::aramhex::{self, AugRow};
use crate::core::Core;
use crate::ocr::{self, Line, Word};
use crate::win;
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

/// The band of the game window that is read (fractions of its height). Card titles sit at 41% of
/// a 3440x1440 window; the cards are sized by the HUD scale, so other setups may move them.
const BAND: (f64, f64) = (0.30, 0.62);
/// From a card title's middle up to the card's top edge, in card spacings (334 px for 489 px).
const TITLE_TO_TOP: f64 = 0.683;
/// Height of the window with the win rates, and the gap below it, in card spacings.
const STRIP: f64 = 0.2;
const GAP: f64 = 0.012;
const EVERY: Duration = Duration::from_millis(500);
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
        // Close matches allow about one wrong letter in six; short names must match exactly.
        if t.len() < 5 {
            return None;
        }
        let mut best: Option<(usize, i32)> = None;
        for (n, id) in &self.list {
            if n.len().abs_diff(t.len()) > t.len() / 6 {
                continue;
            }
            let d = levenshtein(n, &t);
            if d <= n.len().max(t.len()) / 6 && best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, *id));
            }
        }
        best.map(|(d, id)| (id, d))
    }
}

/// Words of at most this many letters and digits may sit around a title and be ignored: a
/// sparkle or a card's frame read as "x", "+" or "Il".
const JUNK: usize = 2;
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
    let junk = |w: &Word| normalize(&w.text).chars().count() <= JUNK;
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

/// The cards among the text read: augment titles, left to right, at most three, each with where
/// its title is.
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
    found.truncate(3);
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
pub fn strip(game: win::Rect, band_y: i32, found: &[(i32, Line)]) -> win::Rect {
    let unit = spacing(found);
    let title_mid = found
        .iter()
        .map(|(_, l)| (l.y + l.h / 2.0) as f64)
        .sum::<f64>()
        / found.len() as f64;
    let card_top = band_y as f64 + title_mid - TITLE_TO_TOP * unit;
    let h = (STRIP * unit).round() as i32;
    let y = ((card_top - GAP * unit) as i32 - h).max(game.y);
    win::Rect {
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
    /// The OCR language that found cards last ("en" or "zh"): tried first.
    lang: &'static str,
    misses: u32,
    /// The cards the last read found (ids and places), shown once the next read agrees.
    last: Vec<(i32, f64)>,
}

async fn names() -> anyhow::Result<Names> {
    let (_, b) = aramhex::bracket("all").await?;
    let hextech: HashSet<i32> = b
        .hero_augments
        .values()
        .flat_map(|m| m.keys().copied())
        .collect();
    let en = crate::augments::all("en").await?;
    let zh = crate::augments::all("zh").await?;
    let all = en
        .iter()
        .chain(zh.iter())
        .map(|(id, a)| (*id, a.name.as_str()));
    Ok(Names::new(all, &hextech))
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
    }
}

/// One look at the screen. Returns false when there is nothing to watch (no game, not Hextech).
async fn step(core: &Arc<Core>, r: &mut Reader) -> bool {
    let Some(champion_id) = my_hextech_champion(core) else {
        return false;
    };
    let Some(game) = win::game_window().and_then(win::window_rect) else {
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
    let band = win::Rect {
        x: game.x,
        y: game.y + (game.h as f64 * BAND.0) as i32,
        w: game.w,
        h: (game.h as f64 * (BAND.1 - BAND.0)) as i32,
    };
    let first = r.lang;
    let read = tauri::async_runtime::spawn_blocking(move || {
        let px = win::capture(band)?;
        let other = if first == "en" { "zh" } else { "en" };
        for lang in [first, other] {
            if !ocr::available(lang) {
                continue;
            }
            match ocr::read(&px, band.w as u32, band.h as u32, lang) {
                Ok(lines) => {
                    let found = cards(&lines, &names);
                    if found.len() >= 2 {
                        return Some((lang, found));
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

    let Some((lang, found)) = read else {
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
    if !settled(&previous, &xs) {
        // Still moving (opening, or a card just rerolled): wait for the next read.
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
        log::info!("card win rates shown: {}", describe(&xs));
        *OFFER.lock() = Some(o.clone());
        core.show_cards(rect, &o);
    }
    true
}

pub fn spawn(core: Arc<Core>) {
    tauri::async_runtime::spawn(async move {
        let mut r = Reader {
            names: None,
            lang: "en",
            misses: 0,
            last: Vec::new(),
        };
        loop {
            let active = core.settings().card_reader && step(&core, &mut r).await;
            if !active {
                clear(&core);
                r.misses = 0;
                r.last.clear();
            }
            tokio::time::sleep(if active { EVERY } else { IDLE }).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let game = win::Rect {
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
}
