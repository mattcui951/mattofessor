//! macOS: the Vision framework's text recognition (VNRecognizeTextRequest).

use super::{tiles, Line, Word};
use anyhow::{anyhow, Result};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{msg_send, AnyThread};
use objc2_core_foundation::CGRect;
use objc2_foundation::{NSArray, NSDictionary, NSError, NSRange, NSString};
use objc2_vision::{VNImageRequestHandler, VNRecognizeTextRequest, VNRequest, VNRequestTextRecognitionLevel};

/// Vision's tag for a language: Windows' tags with the region left out where Vision has none
/// ("zh-Hans-CN" is "zh-Hans").
fn tag(lang: &str) -> String {
    let t = crate::lang::get(lang).ocr;
    match t.strip_suffix("-CN").or_else(|| t.strip_suffix("-TW")) {
        Some(script) if script.starts_with("zh-") => script.to_string(),
        _ => t.to_string(),
    }
}

/// The languages Vision reads on this Mac (built in: no language packs to install).
fn supported() -> &'static [String] {
    static LANGS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    LANGS.get_or_init(|| {
        let req = VNRecognizeTextRequest::new();
        req.setRecognitionLevel(VNRequestTextRecognitionLevel::Accurate);
        unsafe { req.supportedRecognitionLanguagesAndReturnError() }
            .map(|l| l.iter().map(|s| s.to_string()).collect())
            .unwrap_or_default()
    })
}

pub fn available(lang: &str) -> bool {
    let t = tag(lang);
    supported().iter().any(|s| s.eq_ignore_ascii_case(&t))
}

/// Han characters and kana, which Windows' OCR (and the card reader after it) treats as a word
/// each; Vision gives whole lines of them.
fn ideographic(c: char) -> bool {
    matches!(c as u32, 0x3000..=0x30FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0xFF00..=0xFFEF)
}

/// A recognized line's words with their UTF-16 ranges (what Vision locates).
fn split_words(text: &str) -> Vec<(String, NSRange)> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut start, mut pos) = (0usize, 0usize);
    let flush = |cur: &mut String, start: usize, out: &mut Vec<(String, NSRange)>| {
        if !cur.is_empty() {
            let len = cur.encode_utf16().count();
            out.push((std::mem::take(cur), NSRange::new(start, len)));
        }
    };
    for c in text.chars() {
        let n = c.len_utf16();
        if c.is_whitespace() {
            flush(&mut cur, start, &mut out);
        } else if ideographic(c) {
            flush(&mut cur, start, &mut out);
            out.push((c.to_string(), NSRange::new(pos, n)));
        } else {
            if cur.is_empty() {
                start = pos;
            }
            cur.push(c);
        }
        pos += n;
    }
    flush(&mut cur, start, &mut out);
    out
}

/// Widest piece Vision reads at once. It scales an image down to its own working size before
/// reading, so a wide strip (the band across a 3440 pixel game) loses its small titles: wide
/// images are read in overlapping tiles instead; a line read twice is kept once.
const TILE: u32 = 1024;

/// Lines of text in an RGBA image (rows top to bottom).
pub fn read(rgba: &[u8], w: u32, h: u32, lang: &str) -> Result<Vec<Line>> {
    if !available(lang) {
        return Err(anyhow!("Vision cannot read {lang} on this Mac"));
    }
    let tile_w = TILE.max(h * 2);
    let mut lines: Vec<Line> = Vec::new();
    for x0 in tiles(w, tile_w) {
        let tw = w.min(tile_w);
        let mut px = Vec::with_capacity((tw * h * 4) as usize);
        for row in 0..h {
            let start = ((row * w + x0) * 4) as usize;
            px.extend_from_slice(&rgba[start..start + (tw * 4) as usize]);
        }
        for mut found in read_tile(&px, tw, h, lang)? {
            found = shifted(found, x0 as f32);
            // Kept once; a title cut at a tile's edge loses to the whole one.
            let same = |l: &Line| (l.y - found.y).abs() < found.h.max(l.h) * 0.5 && l.x < found.x + found.w && found.x < l.x + l.w;
            match lines.iter().position(same) {
                Some(i) if lines[i].w >= found.w => {}
                Some(i) => lines[i] = found,
                None => lines.push(found),
            }
        }
    }
    Ok(lines)
}

fn shifted(mut l: Line, dx: f32) -> Line {
    l.x += dx;
    for w in &mut l.words {
        w.x += dx;
    }
    l
}

fn read_tile(rgba: &[u8], w: u32, h: u32, lang: &str) -> Result<Vec<Line>> {
    let image = crate::os::cg_image(rgba, w, h).ok_or_else(|| anyhow!("could not make an image for OCR"))?;
    let handler = unsafe { VNImageRequestHandler::initWithCGImage_options(VNImageRequestHandler::alloc(), &image, &NSDictionary::new()) };
    let req = VNRecognizeTextRequest::new();
    req.setRecognitionLevel(VNRequestTextRecognitionLevel::Accurate);
    req.setRecognitionLanguages(&NSArray::from_retained_slice(&[NSString::from_str(&tag(lang))]));
    // Augment names are made up: correcting them to dictionary words only does harm.
    req.setUsesLanguageCorrection(false);
    let as_request: &VNRequest = &req;
    handler.performRequests_error(&NSArray::from_slice(&[as_request])).map_err(|e| anyhow!("OCR failed: {}", e.localizedDescription()))?;
    let (fw, fh) = (w as f64, h as f64);
    // Vision's boxes are fractions of the image from its bottom left.
    let to_px = |b: CGRect| (b.origin.x * fw, (1.0 - b.origin.y - b.size.height) * fh, b.size.width * fw, b.size.height * fh);
    let mut lines = Vec::new();
    for obs in req.results().unwrap_or_default().iter() {
        let Some(best) = obs.topCandidates(1).firstObject() else { continue };
        let text = best.string().to_string();
        let mut words = Vec::new();
        for (word, range) in split_words(&text) {
            let found: Result<Retained<AnyObject>, Retained<NSError>> = unsafe { msg_send![&*best, boundingBoxForRange: range, error: _] };
            let Ok(found) = found else { continue };
            let b: CGRect = unsafe { msg_send![&*found, boundingBox] };
            let (x, y, bw, bh) = to_px(b);
            words.push(Word { text: word, x: x as f32, y: y as f32, w: bw as f32, h: bh as f32 });
        }
        let (mut x1, mut y1, mut x2, mut y2) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for wd in &words {
            x1 = x1.min(wd.x);
            y1 = y1.min(wd.y);
            x2 = x2.max(wd.x + wd.w);
            y2 = y2.max(wd.y + wd.h);
        }
        if x1 > x2 {
            continue;
        }
        // Words joined by spaces, as Windows writes its lines ("吞 噬 灵 魂").
        let text = words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" ");
        lines.push(Line { text, x: x1, y: y1, w: x2 - x1, h: y2 - y1, words });
    }
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_words_like_windows() {
        let w = split_words("Infinite Recursion");
        assert_eq!(w.iter().map(|(t, r)| (t.as_str(), r.location, r.length)).collect::<Vec<_>>(), [("Infinite", 0, 8), ("Recursion", 9, 9)]);
        let w = split_words("吞噬灵魂 x2");
        assert_eq!(w.iter().map(|(t, _)| t.as_str()).collect::<Vec<_>>(), ["吞", "噬", "灵", "魂", "x2"]);
        assert_eq!(w[4].1.location, 5);
    }

    #[test]
    fn vision_tags() {
        assert_eq!(tag("zh"), "zh-Hans");
        assert_eq!(tag("zh-TW"), "zh-Hant");
        assert_eq!(tag("ja"), "ja-JP");
        assert!(available("en"));
        assert!(available("zh") && available("ko") && available("ru"));
    }
}

